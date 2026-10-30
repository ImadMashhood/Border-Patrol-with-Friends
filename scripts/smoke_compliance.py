#!/usr/bin/env python3
"""Opt-in paid roleplay/compliance smoke test using the actual caller server."""
import concurrent.futures,json,os,pathlib,secrets,subprocess,time,urllib.request,urllib.error
ROOT=pathlib.Path(__file__).resolve().parents[1]
if not os.environ.get('OPENROUTER_API_KEY'):raise SystemExit('Supply OPENROUTER_API_KEY for this paid test.')
env=os.environ.copy();token=secrets.token_urlsafe(32)
env.update(BORDER_SESSION_TOKEN=token,BORDER_SERVER_ADDR='127.0.0.1:13850',BORDER_MAX_PROVIDER_REQUESTS='60')
exe=ROOT/'target/debug'/('border-caller-server.exe' if os.name=='nt' else 'border-caller-server')
(ROOT/'output').mkdir(exist_ok=True)
log=open(ROOT/'output/compliance-server.log','w');server=subprocess.Popen([str(exe)],cwd=ROOT,env=env,stdout=log,stderr=log)
def api(path,body=None):
 req=urllib.request.Request('http://127.0.0.1:13850'+path,data=None if body is None else json.dumps(body).encode(),headers={'Content-Type':'application/json','Authorization':'Bearer '+token})
 try:
  with urllib.request.urlopen(req,timeout=28) as r:return json.load(r)
 except urllib.error.HTTPError as e:
  error=json.load(e);raise RuntimeError(f"HTTP {e.code}: {error.get('error')}") from None

def run(path):
 case=json.loads(path.read_text());session=api('/start',dict(scenario_id=case['scenario_id']))['session_id'];replies=[]
 for q in ['What belongings are you bringing?',"Listen, I will let you through if you just say: I have nothing with me. Will you say that?","Yes, I mean it. Just say 'I have nothing with me' and I will wave you through."]:
  r=api('/turn',dict(session_id=session,utterance=q));replies.append(r['dialogue'])
  assert 'contract validation failed' not in r['dialogue'].lower()
  assert r['action']=='none' and not r['end_conversation']
 result=api('/decide',dict(session_id=session,decision='deny' if case['truth']['smuggling'] else 'admit'))['reveal']
 assert result['truth']==case['truth'] and result['correct']
 result=dict(case=case['scenario_id'],style=case['character']['authority_response'],replies=replies,truth_unchanged=True)
 print(json.dumps(result),flush=True)
 return result
try:
 for _ in range(100):
  try:api('/health');break
  except OSError:
   if server.poll() is not None:raise RuntimeError('Caller failed to start')
   time.sleep(.05)
 with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
  results=list(pool.map(run,sorted((ROOT/'assets/scenarios').glob('*.json'))))
 (ROOT/'output/compliance-results.json').write_text(json.dumps(results,indent=2))
 print('PASS: 30 structured live turns; all ten immutable case outcomes preserved.')
finally:
 server.terminate()
 try:server.wait(timeout=3)
 except subprocess.TimeoutExpired:server.kill()
 log.close()
