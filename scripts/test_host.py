#!/usr/bin/env python3
"""Real host + two CLI peer processes; no GPU, model download or paid AI call."""
import json
import os
from pathlib import Path
import queue
import secrets
import socket
import subprocess
import threading
import time
import urllib.request
import urllib.error

ROOT = Path(__file__).resolve().parents[1]
EXT = '.exe' if os.name == 'nt' else ''

def main():
    subprocess.run(['cargo', 'build', '--locked', '-p', 'border-caller-server', '--bin', 'border-host', '-p', 'border-net', '--bin', 'border-peer', '--bin', 'border-caller-server'], cwd=ROOT, check=True)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    url = f'http://127.0.0.1:{port}'
    env = dict(os.environ, BORDER_HOST_BIND=f'127.0.0.1:{port}', BORDER_HOST_JOIN_CODE=secrets.token_hex(8), BORDER_HOST_OPERATOR_TOKEN=secrets.token_hex(24), BORDER_HOST_GATEWAY_TOKEN=secrets.token_hex(24))
    children = []
    def request(path, payload=None, token=None, expected=200):
        headers = {'Content-Type': 'application/json'}
        if token:
            headers['Authorization'] = f'Bearer {token}'
        req = urllib.request.Request(url + path, data=None if payload is None else json.dumps(payload).encode(), headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=5) as response:
                status, body = response.status, json.load(response)
        except urllib.error.HTTPError as error:
            status, body = error.code, error.read().decode()
        assert status == expected, (path, status, body)
        return body
    def peer(name):
        child = subprocess.Popen([str(ROOT / 'target/debug' / ('border-peer' + EXT)), url, name], cwd=ROOT, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
        children.append(child)
        lines = queue.Queue()
        def reader():
            for line in child.stdout:
                lines.put(json.loads(line))
        threading.Thread(target=reader, daemon=True).start()
        return child, lines, lines.get(timeout=10)
    def send(p, command):
        child, lines, _ = p
        child.stdin.write((command if isinstance(command, str) else json.dumps(command)) + '\n')
        child.stdin.flush()
        return lines.get(timeout=10)
    host = subprocess.Popen([str(ROOT / 'target/debug' / ('border-host' + EXT))], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    children.append(host)
    try:
        for _ in range(100):
            try:
                request('/health')
                break
            except (OSError, urllib.error.URLError):
                if host.poll() is not None:
                    raise RuntimeError(host.stderr.read())
                time.sleep(.05)
        else:
            raise RuntimeError('host startup timeout')
        a, b = peer('Officer A'), peer('Officer B')
        session = a[2]['session_id']
        assert session == b[2]['session_id']
        for p, lane in [(a, 1), (b, 2)]:
            send(p, {'type': 'set_ready', 'ready': True})
            send(p, {'type': 'claim_lane', 'lane_id': lane})
        def operator(command):
            return request('/operator', {'protocol_version': 1, 'session_id': session, 'command': command}, env['BORDER_HOST_OPERATOR_TOKEN'])
        snapshot = operator({'type': 'start_shift'})
        assert len([p for p in snapshot['peers'] if p['connected']]) == 2
        request('/join', {'protocol_version': 99, 'join_code': env['BORDER_HOST_JOIN_CODE'], 'display_name': 'old'}, expected=400)
        request('/snapshot', expected=400)
        request('/operator', {'protocol_version': 1, 'session_id': session, 'command': {'type': 'apply_damage', 'peer_id': 1, 'amount': 100}}, env['BORDER_HOST_JOIN_CODE'], expected=400)
        operator({'type': 'spawn_next_vehicle', 'lane_id': 1, 'kind': 'car'})
        for _ in range(2):
            snapshot = operator({'type': 'advance_world', 'delta_ms': 1000})
        assert snapshot['vehicles'][0]['stage'] == 'stopped'
        assert snapshot['vehicles'][0]['position_mm'] == 10000
        raw = json.dumps(snapshot)
        assert 'smuggling' not in raw and 'contradictions' not in raw and 'truth' not in raw
        delivery = {'expected_turn': 0, 'protocol_version': 1, 'session_id': session, 'vehicle_id': 1, 'officer_utterance': 'Hello', 'raw_reply': json.dumps({'dialogue': 'Hello officer.', 'emotion': 'cooperative', 'action': 'none', 'fact_ids': [], 'end_conversation': False})}
        request('/actor', delivery, env['BORDER_HOST_JOIN_CODE'], expected=400)
        request('/actor-context', {'protocol_version': 1, 'session_id': session, 'vehicle_id': 1}, env['BORDER_HOST_JOIN_CODE'], expected=400)
        request('/actor', delivery, env['BORDER_HOST_GATEWAY_TOKEN'])
        request('/actor', delivery, env['BORDER_HOST_GATEWAY_TOKEN'], expected=400)  # stale provider retry
        attack = dict(delivery, expected_turn=1, raw_reply=delivery['raw_reply'].replace('"none"', '"award_money"'))
        request('/actor', attack, env['BORDER_HOST_GATEWAY_TOKEN'], expected=400)
        # Exercise the real provider gateway bridge in deterministic offline mode.
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            gateway_port = sock.getsockname()[1]
        gateway_env = dict(env, BORDER_HOST_URL=url, BORDER_SESSION_TOKEN=secrets.token_hex(24), BORDER_SERVER_ADDR=f'127.0.0.1:{gateway_port}')
        gateway_env.pop('OPENROUTER_API_KEY', None)
        gateway = subprocess.Popen([str(ROOT / 'target/debug' / ('border-caller-server' + EXT))], cwd=ROOT, env=gateway_env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        children.append(gateway)
        for _ in range(100):
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{gateway_port}/health', timeout=1) as response:
                    assert json.load(response)['mode'] == 'offline rehearsal'
                break
            except (OSError, urllib.error.URLError):
                time.sleep(.05)
        else:
            raise RuntimeError('gateway startup timeout')
        bridge_req = urllib.request.Request(f'http://127.0.0.1:{gateway_port}/host-turn', data=json.dumps({'protocol_version': 1, 'session_id': session, 'vehicle_id': 1, 'utterance': 'What is your name?'}).encode(), headers={'Content-Type': 'application/json', 'Authorization': f"Bearer {gateway_env['BORDER_SESSION_TOKEN']}"})
        with urllib.request.urlopen(bridge_req, timeout=10) as response:
            reply = json.load(response)
            assert reply['action'] == 'none' and reply['dialogue']
        receipt = send(a, {'type': 'decide', 'vehicle_id': 1, 'disposition': 'admit'})
        assert receipt['sequence'] == 3
        snapshot = operator({'type': 'create_incident', 'kind': 'fence_breach'})
        assert len(snapshot['ledger']) == 1 and snapshot['money'] in (100, -400)
        view_a, view_b = send(a, '/snapshot'), send(b, '/snapshot')
        assert view_a == view_b == snapshot
        # Both peers observe the same public ledger, vehicle and incident state.
        replay_a = send(a, '/events')
        assert replay_a['latest_sequence'] == snapshot['latest_event_sequence']
        for p in (a, b):
            p[0].stdin.close()
            assert p[0].wait(timeout=10) == 0, p[0].stderr.read()
        snapshot = operator({'type': 'resolve_incident', 'incident_id': 1})
        assert all(not p['connected'] for p in snapshot['peers'])
        assert all(lane['owner'] is None for lane in snapshot['lanes'])
        print('PASS: two real peers, shared session, ready/lane authority, host motion, hidden-state boundary, trusted actor/gateway delivery, matching peer snapshots, settlement and disconnect cleanup')
    finally:
        for child in reversed(children):
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()

if __name__ == '__main__':
    main()
