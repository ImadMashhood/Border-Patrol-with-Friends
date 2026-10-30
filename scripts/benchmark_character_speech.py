#!/usr/bin/env python3
"""Opt-in local speech/acting check: generated samples only, no microphone or paid API."""
import json,os,pathlib,secrets,subprocess,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[1]
SAMPLES=[('09_cal','cooperative',"C'mon, man. That's one hell of a way to ask."),('09_cal','angry','Hey! Back off! Put that gun down!'),('07_amir','panicked','Wait! Please! I do not want to get hurt!'),('01_mara','cooperative','Oh, lovely. Do you want the camera or the bag?'),('08_rosa','irritated',"Buddy, I heard you. I just don't like the question."),('10_priya','cooperative',"Right. Was that a question, or are you workshopping a joke?"),('09_cal','panicked','AAAH!')]
def main():
 env=dict(os.environ,BORDER_SPEECH_TOKEN=secrets.token_hex(24));env.pop('OPENROUTER_API_KEY',None)
 url='http://127.0.0.1:13857';rows=[]
 with open(ROOT/'output/character-speech-service.log','w') as log:
  child=subprocess.Popen([str(ROOT/'scripts/gpu-speech.sh'),'--port','13857'],cwd=ROOT,env=env,stdout=log,stderr=log)
  def post(path,data,kind):
   req=urllib.request.Request(url+path,data=data,headers={'Authorization':'Bearer '+env['BORDER_SPEECH_TOKEN'],'Content-Type':kind})
   with urllib.request.urlopen(req,timeout=120) as response:return response.read()
  try:
   for _ in range(900):
    try:
     with urllib.request.urlopen(url+'/health',timeout=1) as r:health=json.load(r)
     break
    except OSError:
     if child.poll() is not None:raise RuntimeError('Speech service failed; see output/character-speech-service.log')
     time.sleep(.2)
   else:raise RuntimeError('Speech service startup timed out')
   for i,(case,emotion,text) in enumerate(SAMPLES):
    started=time.monotonic();audio=post('/tts',json.dumps(dict(text=text,scenario_id=case,emotion=emotion)).encode(),'application/json');elapsed=time.monotonic()-started
    path=ROOT/f'output/character-voice-{case}-{emotion}.wav';path.write_bytes(audio)
    stt_started=time.monotonic();transcript=json.loads(post('/stt',audio,'audio/wav'));stt=time.monotonic()-stt_started
    rows.append(dict(scenario=case,emotion=emotion,text=text,path=path.name,tts_s=round(elapsed,3),stt_s=round(stt,3),transcript=transcript['text']))
    print(json.dumps(rows[-1]),flush=True)
   with urllib.request.urlopen(url+'/health',timeout=1) as r:health=json.load(r)
   result=dict(health=health,samples=rows)
   (ROOT/'output/character-speech-validation.json').write_text(json.dumps(result,indent=2)+'\n')
  finally:
   child.terminate()
   try:child.wait(timeout=4)
   except subprocess.TimeoutExpired:child.kill();child.wait()
if __name__=='__main__':main()
