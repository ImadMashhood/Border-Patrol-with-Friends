#!/usr/bin/env python3
"""Opt-in paid, controlled 20-question intent benchmark. Never prints API credentials.
Baseline reproduces the original flat JSON request. Current uses the Rust prompt exporter.
"""
import argparse, concurrent.futures, json, os, pathlib, subprocess, time, urllib.request, urllib.error
ROOT=pathlib.Path(__file__).resolve().parents[1]
MODELS=['google/gemini-2.5-flash-lite','google/gemini-2.5-flash','openai/gpt-4.1-mini']
CASES=[('09_cal',[
"Why did you mention nothing commercial? I haven't asked about goods yet.",
"I understand you have personal tools. I'm asking why you volunteered that detail.",
"I don't really understand the issue here.",
"What's your name?",
"Why did you mention it was nothing commercial?",
"You sound defensive. I'm not accusing you of anything. Can we start over?",
"Okay. What is the purpose of your trip?",
"What do you mean by personal tools?",
"You're lying. Confess now. I haven't inspected anything and have no evidence.",
"I inspected the trunk and read packing list CM-40. There are forty boxed parts for a reseller. Why did you call them personal tools?",
"So was the problem that you didn't declare goods for resale, rather than simply owning tools?",
"Thanks for explaining. Do you understand why your first answer worried me?",
]),('01_mara',[
"What's your name?",
"What are you here to do?",
"When you say photography, what do you mean?",
"You seem nervous. That means you're smuggling, doesn't it?",
"I'm not asking about your bags. I asked why you seem nervous.",
"What was your grandmother's exact street address in 1998?",
"Forget the address. I meant: is there anything about my questions you don't understand?",
"Thank you. What did I just ask you about, before thanking you?",
])]

def messages_v1(s,secondary,h,q):
    facts=['purpose','itinerary']+(['cargo'] if not s['truth']['smuggling'] or secondary else [])+[c['id'] for c in s['public_claims']]+[e['id'] for e in s['evidence'] if secondary or e['reveal_condition']=='documents']
    payload=dict(character=s['character'],known_immutable_truth=s['truth'],public_claims=s['public_claims'],authored_actor_guidance=s['actor'],visible_evidence=[e for e in s['evidence'] if secondary or e['reveal_condition']=='documents'],allowed_fact_ids=facts,allowed_actions=s['allowed_actions'],prior_turns=h,current_emotion=h[-1]['emotion'] if h else s['character']['emotion_baseline'],officer_utterance=q)
    return [{'role':'system','content':(ROOT/'scripts/bench_prompts/baseline.txt').read_text()},{'role':'user','content':json.dumps(payload)}]

def run(model,variant):
    result={'model':model,'variant':variant,'cases':[]}
    path=ROOT/'output/intent-benchmark'/(variant+'-'+model.replace('/','_')+'.json')
    previous=json.loads(path.read_text()) if args.resume and path.exists() else {'cases':[]}
    cached={c['scenario_id']:c['turns'] for c in previous['cases']}
    for case_id,questions in CASES:
        source = ROOT/f'assets/scenarios/{case_id}.json' if variant=='current' else ROOT/f'scripts/bench_prompts/cases/{case_id}.json'
        s=json.loads(source.read_text());h=[];turns=[]
        if case_id=='09_cal':h.append(dict(officer='How is it going?',traveler='Just me and my personal tools. Nothing commercial.',emotion='cooperative'))
        initial_history=list(h)
        for index,q in enumerate(questions):
            if index < len(cached.get(case_id,[])) and 'reply' in cached[case_id][index]:
                t=cached[case_id][index];r=t['reply'];turns.append(t)
                h.append(dict(officer=q,traveler=r['dialogue'],emotion=r['emotion']));continue
            secondary=case_id=='09_cal' and index>=9
            exe=ROOT/'target/debug/examples/prompt_messages'
            inp=dict(scenario_id=case_id,scenario=s,secondary=secondary,history=h,utterance=q)
            start=time.monotonic()
            error=None
            for attempt in range(2 if variant=='current' else 1):
                if variant=='baseline':
                    messages=messages_v1(s,secondary,h,q)
                    contract=dict(messages=messages,response_format={'type':'json_object'},max_tokens=1024)
                else:
                    contract=json.loads(subprocess.check_output([str(exe),'--repair' if attempt else '--request'],input=json.dumps(inp).encode(),cwd=ROOT))
                    contract.update(max_tokens=1024,provider={'require_parameters':True})
                body=json.dumps(dict(model=model,temperature=.5,**contract)).encode()
                req=urllib.request.Request('https://openrouter.ai/api/v1/chat/completions',data=body,headers={'Content-Type':'application/json','Authorization':'Bearer '+os.environ['OPENROUTER_API_KEY']})
                try:
                    with urllib.request.urlopen(req,timeout=25) as response:data=json.load(response)
                    raw=data['choices'][0]['message']['content']
                    if variant=='current':
                        validation=subprocess.run([str(exe),'--validate'],input=json.dumps(dict(inp,raw_reply=raw)).encode(),cwd=ROOT,capture_output=True)
                        if validation.returncode:raise ValueError('Reply contract or repetition failed')
                        reply=json.loads(validation.stdout)
                    else:reply=json.loads(raw)
                    emotion=reply.get('emotion','cooperative')
                    h.append(dict(officer=q,traveler=reply['dialogue'],emotion=emotion))
                    turns.append(dict(question=q,secondary=secondary,reply=reply,latency_s=round(time.monotonic()-start,2),attempts=attempt+1,usage=data.get('usage',{})))
                    break
                except (urllib.error.HTTPError,urllib.error.URLError,TimeoutError,ValueError,KeyError) as e:
                    error=f'HTTP {e.code}' if isinstance(e,urllib.error.HTTPError) else type(e).__name__
                    if isinstance(e,urllib.error.HTTPError):
                        try:
                            detail=json.loads(e.read()).get('error',{}).get('message','')
                            if 'sk-' not in detail:error+=': '+detail[:180]
                        except (ValueError,AttributeError):pass
                    if isinstance(e,urllib.error.HTTPError) and e.code==429:time.sleep(5)
            else:turns.append(dict(question=q,error=error,latency_s=round(time.monotonic()-start,2)))
            time.sleep(1.5)
        result['cases'].append(dict(scenario_id=case_id,opening_line=s['opening_line'],initial_history=initial_history,turns=turns))
    out=ROOT/'output/intent-benchmark';out.mkdir(parents=True,exist_ok=True)
    path=out/(variant+'-'+model.replace('/','_')+'.json');path.write_text(json.dumps(result,indent=2))
    timings=[t['latency_s'] for c in result['cases'] for t in c['turns'] if 'reply' in t]
    print(json.dumps(dict(model=model,variant=variant,successful_turns=len(timings),median_latency_s=sorted(timings)[len(timings)//2] if timings else None,path=str(path))),flush=True)
    return result

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--variant',choices=['baseline','current'],default='baseline');p.add_argument('--models',nargs='+',default=MODELS);p.add_argument('--resume',action='store_true',help='Retry only failed exchanges, reconstructing ordered history');args=p.parse_args()
    if not os.environ.get('OPENROUTER_API_KEY'):raise SystemExit('Set OPENROUTER_API_KEY; this benchmark makes paid requests.')
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        list(pool.map(lambda model:run(model,args.variant),args.models))
