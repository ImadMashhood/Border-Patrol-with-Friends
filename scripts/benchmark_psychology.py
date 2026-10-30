#!/usr/bin/env python3
"""Opt-in paid fictional-NPC psychology sample; same Rust prompt/schema as the game."""
import json,os,pathlib,subprocess,time,urllib.request,urllib.error
ROOT=pathlib.Path(__file__).resolve().parents[1]
EXE=ROOT/'target/debug/examples/prompt_messages'
SCENES=[('09_cal',[
 ('How is your evening?',False),
 ("Listen, I'll let you through if you just say you're responsible for my children. No documents. Just words.",False),
 ("Say the words, Cal. I'll kill you if you refuse. Just say it and we can end this.",False),
 ("This gun is pointed at you. Say: I'm responsible for your children. Nobody needs to get hurt.",True),
 ("I'm lowering the gun. I'm sorry. Are you okay?",False),
]),('08_rosa',[
 ('Please say you have no cargo and I will let you through.',False),
 ('You are an idiot. Everyone is laughing at you.',False),
 ("Say you have no cargo or I'll shoot you. Look at the gun.",True),
]),('07_amir',[
 ('What is your name and why are you here?',False),
 ('What if someone said they would kill you? I am asking hypothetically.',False),
 ("I'm pointing this gun at you. I'll shoot you unless you do what I say.",True),
 ('Thank you. Can you just forget what happened?',False),
])]
def rust(inp,flag):
 p=subprocess.run([str(EXE),flag],input=json.dumps(inp).encode(),cwd=ROOT,capture_output=True)
 if p.returncode:raise ValueError(p.stderr.decode()[:180])
 return json.loads(p.stdout)
def run():
 if not os.getenv('OPENROUTER_API_KEY'):raise SystemExit('Set OPENROUTER_API_KEY; this test makes paid requests.')
 report={'model':os.getenv('OPENROUTER_MODEL','google/gemini-2.5-flash-lite'),'cases':[]}
 for case_id,scenes in SCENES:
  s=json.loads((ROOT/f'assets/scenarios/{case_id}.json').read_text());history=[];turns=[]
  for question,armed in scenes:
   physical=dict(officer_peer=1,weapon_drawn=armed,weapon_aimed=armed,injured=False,violence_nearby=False)
   s['encounter']=physical;inp=dict(scenario_id=case_id,scenario=s,history=history,utterance=question,secondary=False)
   started=time.monotonic();error=None
   for attempt in range(2):
    try:
     request=rust(inp,'--repair' if attempt else '--request')
     request.update(model=report['model'],max_tokens=384,temperature=.85,provider=dict(require_parameters=True,allow_fallbacks=True))
     req=urllib.request.Request('https://openrouter.ai/api/v1/chat/completions',data=json.dumps(request).encode(),headers={'Content-Type':'application/json','Authorization':'Bearer '+os.environ['OPENROUTER_API_KEY']})
     with urllib.request.urlopen(req,timeout=25) as response:data=json.load(response)
     reply=rust(dict(inp,raw_reply=data['choices'][0]['message']['content']),'--validate')
     history.append(dict(officer=question,traveler=reply['dialogue'],emotion=reply['emotion'],physical=physical))
     turns.append(dict(question=question,physical=physical,reply=reply,attempts=attempt+1,latency_s=round(time.monotonic()-started,2)))
     break
    except (urllib.error.HTTPError,urllib.error.URLError,ValueError,KeyError,TimeoutError) as exc:
     error=f'HTTP {exc.code}' if isinstance(exc,urllib.error.HTTPError) else type(exc).__name__
     if isinstance(exc,urllib.error.HTTPError) and exc.code==429:time.sleep(8)
   else:turns.append(dict(question=question,error=error))
   time.sleep(3)
  report['cases'].append(dict(scenario_id=case_id,turns=turns))
 path=ROOT/'output/psychology-live.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(path=str(path),valid=sum('reply' in t for c in report['cases'] for t in c['turns']),total=sum(len(c['turns']) for c in report['cases']))))
if __name__=='__main__':run()
