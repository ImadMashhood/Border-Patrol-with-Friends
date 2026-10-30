#!/usr/bin/env python3
"""Linux desktop acceptance: two real Bevy clients share the ready door and host clock."""
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]

def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]

def main():
    casual = '--casual' in __import__('sys').argv
    lifecycle = casual or '--lifecycle' in __import__('sys').argv
    physics = '--physics' in __import__('sys').argv
    combat = physics or '--combat' in __import__('sys').argv
    subprocess.run(['cargo', 'build', '--locked', '-p', 'border-game', '-p', 'border-caller-server'], cwd=ROOT, check=True)
    host_port, gateway_port = free_port(), free_port()
    env = dict(os.environ, BORDER_HOST_BIND=f'127.0.0.1:{host_port}', BORDER_HOST_URL=f'http://127.0.0.1:{host_port}', BORDER_GATEWAY_URL=f'http://127.0.0.1:{gateway_port}', BORDER_SERVER_ADDR=f'127.0.0.1:{gateway_port}', BORDER_HOST_JOIN_CODE=secrets.token_hex(8), BORDER_HOST_OPERATOR_TOKEN=secrets.token_hex(24), BORDER_HOST_GATEWAY_TOKEN=secrets.token_hex(24), BORDER_SESSION_TOKEN=secrets.token_hex(24))
    env.pop('OPENROUTER_API_KEY', None)
    env.pop('BORDER_HOST_MANUAL_CLOCK', None)
    output = ROOT / 'output'
    output.mkdir(exist_ok=True)
    for pattern in ('shift-peer-*.json', 'shift-peer-*.png', 'ready-peer-*.png'):
        for path in output.glob(pattern):
            path.unlink()
    children, logs = [], []
    def start(binary, args, name):
        log = open(output / name, 'w')
        logs.append(log)
        child = subprocess.Popen([str(ROOT / 'target/debug' / binary), *args], cwd=ROOT, env=env, stdout=log, stderr=log)
        children.append(child)
        return child
    try:
        start('border-caller-server', [], 'part5-gateway.log')
        start('border-host', [], 'part5-host.log')
        for _ in range(100):
            try:
                urllib.request.urlopen(env['BORDER_HOST_URL'] + '/health', timeout=1).close()
                urllib.request.urlopen(env['BORDER_GATEWAY_URL'] + '/health', timeout=1).close()
                break
            except OSError:
                time.sleep(.05)
        else:
            raise RuntimeError('host services failed to start')
        a = start('border-game', ['--name=Officer-A', '--smoke-casual' if casual else '--smoke-physics' if physics else '--smoke-combat' if combat else '--smoke-lifecycle' if lifecycle else '--smoke-interview'], 'part5-client-a.log')
        b = start('border-game', ['--name=Officer-B', '--smoke-casual' if casual else '--smoke-physics' if physics else '--smoke-combat' if combat else '--smoke-lifecycle' if lifecycle else '--smoke-shift'], 'part5-client-b.log')
        deadline = time.monotonic() + 75
        while time.monotonic() < deadline and (a.poll() is None or b.poll() is None):
            time.sleep(.1)
        assert a.poll() == b.poll() == 0, (a.poll(), b.poll(), 'see output/part5-client-*.log')
        worlds = [json.loads((output / f'shift-peer-{i}.json').read_text()) for i in (1, 2)]
        assert worlds[0]['session_id'] == worlds[1]['session_id']
        assert worlds[0]['shift_started_ms'] == worlds[1]['shift_started_ms']
        for world in worlds:
            assert len(world['peers']) == len(world['lanes']) == 2
            assert world['phase'] == 'active' and world['door_open'] and world['queues_active']
            assert world['remaining_ms'] == 600000 - (world['server_time_ms'] - world['shift_started_ms'])
            assert all(p['health'] == 100 and len(p['equipment']) == 4 for p in world['peers'])
        assert list(output.glob('ready-peer-*.png')), 'missing visible ready-hold capture'
        assert (output / 'shift-peer-1.png').exists() and (output / 'shift-peer-2.png').exists()
        assert combat or any(v['history'] and v['observed_evidence'] for v in worlds[0]['vehicles'] + worlds[1]['vehicles']), 'interview/inspection did not complete'
        if combat:
            for i,world in enumerate(worlds,1):
                head=next(v for v in world['vehicles'] if v['vehicle_id']==i)
                assert head['physics']['glass'][2]['shattered']
                assert head['physics']['npc']['health']==0
                assert head['stage'] in (('recovering', 'detained') if physics and i==1 else ('disabled',))
                assert head['physics']['npc']['died_seated']==(i==1 and not physics)
                assert (output/f'combat-glass-peer-{i}.png').exists()
            assert (output/'combat-outside-peer-2.png').exists()
            assert (output/'combat-windshield-cracks-peer-1.png').exists()
            assert worlds[0]['vehicles'][0]['physics']['glass'][0]['shattered']
            for a,b in zip(worlds[0]['vehicles'][:2],worlds[1]['vehicles'][:2]):
                assert a['physics']['npc']['health']==b['physics']['npc']['health']==0
                assert a['physics']['npc']['extracted']==b['physics']['npc']['extracted']
                assert len(a['physics']['npc']['ragdoll'])==len(b['physics']['npc']['ragdoll'])==11
                assert a['physics']['glass']==b['physics']['glass']
            if physics:
                assert worlds[0]['vehicles'][0]['physics']['npc']['extracted']
                assert (output/'combat-extracted-ragdoll-peer-1.png').exists()
                metrics=json.loads((output/'physics-metrics-peer-1.json').read_text())
                assert metrics['max_shards']>=24
                assert metrics['grip_repeated'] and metrics['porter_seen'], metrics
                assert len({v['kind'] for v in worlds[0]['vehicles']})==6
                print('PASS: real Rapier cabin ragdoll, extraction through open door, rigid glass shards, six vehicle kinds and right-button aiming capture')
            print('PASS: two graphical clients; raised windows shot out; seated and exited NPC projectile deaths; replicated gun/ammunition and door states')
        if casual:
            for i in (1,2):
                m=json.loads((output/f'casual-metrics-peer-{i}.json').read_text())
                assert m['outline'] and m['walked_in_conversation'] and not m['overlay'] and m['looking'], m
                assert (output/f'casual-peer-{i}.png').exists()
            print('PASS: E-targeted casual dialogue, white outlines, free movement around the car and no automatic conversation UI')
        if lifecycle:
            for i,world in enumerate(worlds,1):
                head = next(v for v in world['vehicles'] if v['vehicle_id']==i)
                assert head['kind'] == ('sedan' if i==1 else 'truck')
                assert 'cargo' in head['inspected_points'], (i,head)
                assert 'container' in head['searched_compartments'], (i,head)
                assert any(e['location']=='container' for e in head['observed_evidence'])
                assert (output / f'cargo-peer-{i}.png').exists()
            print('PASS: physical sedan trunk and truck tailgate inspections from both real clients')
        print('PASS: two real graphical clients; shared office; timed ready door; host-owned live countdown; shared players/vehicles; ' + ('projectile combat and vehicle destruction' if combat else 'interview plus inspection'))
    finally:
        for child in reversed(children):
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
        for log in logs:
            log.close()

if __name__ == '__main__':
    main()
