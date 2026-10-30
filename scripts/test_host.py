#!/usr/bin/env python3
"""Two authenticated peers, real HTTP services, deterministic host clock; no GPU or paid AI."""
import json
import math
import os
from pathlib import Path
import secrets
import socket
import subprocess
import time
import urllib.request
import urllib.error

ROOT = Path(__file__).resolve().parents[1]
EXT = '.exe' if os.name == 'nt' else ''

def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]

def main():
    subprocess.run(['cargo', 'build', '--locked', '-p', 'border-caller-server', '--bins'], cwd=ROOT, check=True)
    host_port, gateway_port = free_port(), free_port()
    url = f'http://127.0.0.1:{host_port}'
    env = dict(os.environ, BORDER_HOST_BIND=f'127.0.0.1:{host_port}', BORDER_HOST_URL=url, BORDER_GATEWAY_URL=f'http://127.0.0.1:{gateway_port}', BORDER_SERVER_ADDR=f'127.0.0.1:{gateway_port}', BORDER_HOST_JOIN_CODE=secrets.token_hex(8), BORDER_HOST_OPERATOR_TOKEN=secrets.token_hex(24), BORDER_HOST_GATEWAY_TOKEN=secrets.token_hex(24), BORDER_SESSION_TOKEN=secrets.token_hex(24), BORDER_HOST_MANUAL_CLOCK='1')
    env.pop('OPENROUTER_API_KEY', None)
    children = []
    def request(path, payload=None, token=None, expected=200):
        headers = {'Content-Type': 'application/json'}
        if token:
            headers['Authorization'] = f'Bearer {token}'
        req = urllib.request.Request(url + path, data=None if payload is None else json.dumps(payload).encode(), headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=35) as response:
                status, body = response.status, json.load(response)
        except urllib.error.HTTPError as error:
            status, body = error.code, error.read().decode()
        assert status == expected, (path, status, body)
        return body
    try:
        for binary in ('border-caller-server', 'border-host'):
            children.append(subprocess.Popen([str(ROOT / 'target/debug' / (binary + EXT))], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
        for _ in range(100):
            try:
                health = request('/health')
                break
            except OSError:
                time.sleep(.05)
        else:
            raise RuntimeError('host startup timeout')
        version = health['protocol_version']
        peers = [request('/join', {'protocol_version': version, 'join_code': env['BORDER_HOST_JOIN_CODE'], 'display_name': name}) for name in ('Officer A', 'Officer B')]
        session = peers[0]['session_id']
        frames = [0, 0]
        sequences = [1, 1]
        def inputs(i, move=(0., 0.), held=False, expected=200, **extra):
            frames[i] += 1
            return request('/input', dict(protocol_version=version, session_id=session, frame=frames[i], movement=list(move), yaw=math.pi, ready_held=held, **extra), peers[i]['peer_token'], expected)
        def snapshot(i=0):
            return request('/snapshot', token=peers[i]['peer_token'])
        def command(i, body, expected=200):
            result = request('/command', {'protocol_version': version, 'session_id': session, 'request_id': f'{i}-{sequences[i]}', 'sequence': sequences[i], 'expected_revision': None, 'command': body}, peers[i]['peer_token'], expected)
            if expected == 200:
                sequences[i] += 1
            return result
        def tick(ms=100):
            return request('/operator', {'protocol_version': version, 'session_id': session, 'command': {'type': 'advance_world', 'delta_ms': ms}}, env['BORDER_HOST_OPERATOR_TOKEN'])
        def move_to(i, target):
            for _ in range(200):
                p = snapshot(i)['peers'][i]['position']
                dx, dz = target[0] - p[0], target[1] - p[2]
                length = math.hypot(dx, dz)
                if length < .12:
                    inputs(i)
                    return
                scale = min(1., length / .4)
                inputs(i, (dx / length * scale, dz / length * scale))
                inputs(1-i)
                tick()
            raise AssertionError(('movement blocked', i, target, p))
        assert len(peers[0]['snapshot']['lanes']) == 2
        assert all(p['health'] == 100 and len(p['equipment']) == 4 for p in peers[1]['snapshot']['peers'])
        request('/join', {'protocol_version': version, 'join_code': env['BORDER_HOST_JOIN_CODE'], 'display_name': 'Third'}, expected=400)
        command(0, {'type': 'set_ready', 'ready': True}, expected=400)
        inputs(0, held=True, elapsed_ms=3000, expected=422)
        inputs(0, position=[-12, 1.7, -4.8], expected=422)
        for _ in range(35):
            inputs(0, held=True)
            inputs(1)
            state = tick()
        assert state['phase'] == 'lobby' and not state['vehicles'] and state['remaining_ms'] == 600000
        move_to(1, (-12., -4.8))
        for _ in range(32):
            inputs(0)
            inputs(1, held=True)
            state = tick()
        assert state['phase'] == 'lobby' and state['peers'][1]['ready']
        command(1, {'type': 'set_ready', 'ready': False})
        inputs(1)
        move_to(0, (-12., -4.8))
        for _ in range(10):
            inputs(0, held=True)
            inputs(1)
            state = tick()
        inputs(0)
        state = tick()
        assert state['peers'][0]['hold_ms'] == 0
        for _ in range(29):
            inputs(0, held=True)
            inputs(1)
            state = tick()
        assert state['phase'] == 'lobby', 'partial hold must not start a shift'
        for _ in range(3):
            inputs(0, held=True)
            inputs(1)
            state = tick()
        assert state['phase'] == 'active' and state['door_open'] and state['events_active']
        assert len(state['vehicles']) == 10 and state['peers'][0]['ready'] and not state['peers'][1]['ready']
        start = state['shift_started_ms']
        for i in (0, 1):
            inputs(i)
        command(0, {'type': 'request_lane_swap'})
        assert snapshot()['lanes'][0]['owner'] == peers[0]['peer_id']
        command(1, {'type': 'accept_lane_swap'})
        assert snapshot()['lanes'][0]['owner'] == peers[1]['peer_id']
        command(1, {'type': 'request_lane_swap'})
        command(0, {'type': 'accept_lane_swap'})
        command(1, {'type': 'assist_lane', 'lane_id': 1})
        for waypoint in [(-12., -2.), (-4., -2.), (-4., -4.2), (-1.6, -4.2), (-1.6, 0.)]:
            move_to(1, waypoint)
        for _ in range(80):
            inputs(0)
            inputs(1)
            state = tick()
        assert state['vehicles'][0]['stage'] == 'stopped'
        assert snapshot(0)['remaining_ms'] == snapshot(1)['remaining_ms']
        assert state['remaining_ms'] == 600000 - (state['server_time_ms'] - start)
        serialized = json.dumps(state)
        assert '"truth"' not in serialized and '"smuggling"' not in serialized
        request('/start', {'session_id': '1'}, peers[0]['peer_token'], expected=400)
        request('/start', {'session_id': '1'}, peers[1]['peer_token'])
        reply = request('/turn', {'session_id': '1', 'utterance': 'What is your name?'}, peers[1]['peer_token'])
        assert reply['action'] == 'none'
        assert len(snapshot(0)['vehicles'][0]['history']) == len(snapshot(1)['vehicles'][0]['history']) == 1
        request('/inspect', {'session_id': '1'}, peers[1]['peer_token'])
        request('/decide', {'session_id': '1', 'decision': 'admit'}, peers[1]['peer_token'])
        request('/decide', {'session_id': '1', 'decision': 'admit'}, peers[1]['peer_token'])
        assert len(snapshot()['ledger']) == 1
        command(1, {'type': 'release_vehicle', 'vehicle_id': 1})
        assert [v['kind'] for v in snapshot()['vehicles'][:2]] == ['sedan','truck']
        for waypoint in [(-1.6,-4.2),(5.4,-4.2),(5.4,-1.2)]:
            move_to(1,waypoint)
        request('/start', {'session_id':'2'}, peers[1]['peer_token'])
        command(1, {'type':'inspect_vehicle_point','vehicle_id':2,'point':'cargo'}, expected=400)
        for waypoint in [(5.4,3.95),(7.,3.95)]:
            move_to(1,waypoint)
        command(0, {'type':'inspect_vehicle_point','vehicle_id':2,'point':'cargo'}, expected=400)
        command(1, {'type':'inspect_vehicle_point','vehicle_id':2,'point':'cargo'})
        truck=snapshot()['vehicles'][1]
        assert truck['secondary_inspected'] and truck['inspected_points']==['cargo'] and truck['observed_evidence']
        for waypoint in [(5.4,3.95),(5.4,-1.2)]:
            move_to(1,waypoint)
        request('/decide', {'session_id':'2','decision':'deny'}, peers[1]['peer_token'])
        command(1, {'type':'release_vehicle','vehicle_id':2})
        for _ in range(70):
            inputs(0)
            inputs(1)
            state = tick(1000)
        assert state['vehicles'][2]['stage'] == state['vehicles'][3]['stage'] == 'stopped', 'both lanes must advance independently'
        assert state['vehicles'][0]['stage'] == state['vehicles'][1]['stage'] == 'exited'
        assert state['notices'], 'security hooks must activate after shift start'
        while state['remaining_ms']:
            inputs(0)
            inputs(1)
            state = tick(1000)
        assert state['phase'] == 'grace' and not state['events_active']
        command(1, {'type': 'reset_shift'}, expected=400)
        command(0, {'type': 'reset_shift'})
        state = snapshot()
        assert state['phase'] == 'lobby' and not state['door_open'] and not state['vehicles']
        print('PASS: two peers; authoritative 3-second hold; host override; exact ten-minute clock; two lanes; consented swaps; shared assisted interview; sedan/truck physical cargo inspection and both departure routes; event hooks; safe restart')
    finally:
        for child in reversed(children):
            child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()

if __name__ == '__main__':
    main()
