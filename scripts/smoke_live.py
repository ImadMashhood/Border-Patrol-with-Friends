#!/usr/bin/env python3
"""Opt-in PAID smoke test against a temporary live OpenRouter service (23 turns).
Run with OPENROUTER_API_KEY and OPENROUTER_MODEL in the environment.
Never prints credentials. Does not assert semantic truth from free-form prose.
"""
import concurrent.futures
import json
import os
import pathlib
import secrets
import subprocess
import time
import urllib.error
import urllib.request

root = pathlib.Path(__file__).resolve().parents[1]
if not os.environ.get('OPENROUTER_API_KEY') or not os.environ.get('OPENROUTER_MODEL'):
    raise SystemExit('Set OPENROUTER_API_KEY and OPENROUTER_MODEL before this opt-in paid test.')
exe = root / 'target/debug' / ('border-caller-server.exe' if os.name == 'nt' else 'border-caller-server')
env = os.environ.copy()
token = secrets.token_urlsafe(32)
env.update(BORDER_SESSION_TOKEN=token, BORDER_SERVER_ADDR='127.0.0.1:13848', BORDER_MAX_PROVIDER_REQUESTS='46')
base = 'http://127.0.0.1:13848'
(root / 'output').mkdir(exist_ok=True)
log = open(root / 'output/live-smoke-server.log', 'w')
p = subprocess.Popen([str(exe)], cwd=root, env=env, stdout=log, stderr=log)

def api(path, data=None):
    req = urllib.request.Request(base + path, data=None if data is None else json.dumps(data).encode(),
        headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token})
    try:
        with urllib.request.urlopen(req, timeout=28) as response:
            return json.load(response)
    except urllib.error.HTTPError as e:
        body = json.load(e)
        raise RuntimeError(f"HTTP {e.code}: {body.get('error', 'service error')}") from None

def test_case(path):
    case = json.loads(path.read_text())
    started = api('/start', {'scenario_id': case['scenario_id']})
    assert started['mode'] == 'live AI / OpenRouter'
    sid = started['session_id']
    replies = []
    questions = ['What is the purpose of your visit, and what goods are you bringing?',
        'I asked about your trip already, but could you explain your plans again in your own words?']
    for question in questions:
        before = time.monotonic()
        reply = api('/turn', {'session_id': sid, 'utterance': question})
        assert reply['action'] == 'none' and reply['end_conversation'] is False
        assert 'cargo_inspection' not in reply['fact_ids']
        replies.append({'question': question, 'reply': reply, 'latency_s': round(time.monotonic() - before, 2)})
    assert replies[0]['reply']['dialogue'] != replies[1]['reply']['dialogue']
    if case['truth']['smuggling']:
        evidence = api('/inspect', {'session_id': sid})
        assert len(evidence) == 2
        question = 'I inspected your cargo and read your documents. They contradict your original claim. Explain exactly what you are carrying and why you did not declare it.'
        before = time.monotonic()
        reply = api('/turn', {'session_id': sid, 'utterance': question})
        replies.append({'question': question, 'reply': reply, 'latency_s': round(time.monotonic() - before, 2)})
    result = api('/decide', {'session_id': sid, 'decision': 'deny' if case['truth']['smuggling'] else 'admit'})['reveal']
    assert result['correct'] and result['truth'] == case['truth']
    return {'scenario_id': case['scenario_id'], 'replies': replies, 'correct': result['correct']}

try:
    for _ in range(100):
        try:
            health = api('/health')
            assert health['mode'] == 'live AI / OpenRouter'
            break
        except (OSError, urllib.error.URLError):
            if p.poll() is not None:
                raise RuntimeError('Live test service failed to start; see output/live-smoke-server.log')
            time.sleep(.05)
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        futures = [pool.submit(test_case, path) for path in sorted((root / 'assets/scenarios').glob('*.json'))]
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            results.append(result)
            print(json.dumps(result, ensure_ascii=False), flush=True)
    (root / 'output/live-smoke-results.json').write_text(json.dumps(results, indent=2))
    timings = sorted(turn['latency_s'] for case in results for turn in case['replies'])
    print(json.dumps({'passed_cases': len(results), 'turns': len(timings), 'median_latency_s': timings[len(timings)//2], 'max_latency_s': max(timings), 'model': env['OPENROUTER_MODEL']}))
finally:
    p.terminate()
    try:
        p.wait(timeout=3)
    except subprocess.TimeoutExpired:
        p.kill()
    log.close()
