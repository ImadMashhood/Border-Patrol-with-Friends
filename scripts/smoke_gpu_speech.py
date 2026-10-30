#!/usr/bin/env python3
"""Verify the persistent service through the real Rust adapters. No microphone capture."""
import json,os,pathlib,secrets,subprocess,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[1]
env=os.environ.copy();env.pop('OPENROUTER_API_KEY',None);env.pop('OPENROUTER_MODEL',None)
env.update(BORDER_SPEECH_TOKEN=secrets.token_urlsafe(32),BORDER_SPEECH_URL='http://127.0.0.1:13849')
(ROOT/'output').mkdir(exist_ok=True)
with open(ROOT/'output/speech-adapter-check.log','w') as log:
    process=subprocess.Popen([str(ROOT/'scripts/gpu-speech.sh'),'--port','13849'],cwd=ROOT,env=env,stdout=log,stderr=log)
    try:
        for _ in range(600):
            try:
                with urllib.request.urlopen(env['BORDER_SPEECH_URL']+'/health',timeout=1) as r:health=json.load(r)
                break
            except OSError:
                if process.poll() is not None:raise RuntimeError('Speech service failed; see output/speech-adapter-check.log')
                time.sleep(.1)
        else:raise RuntimeError('GPU speech readiness timed out')
        sample=ROOT/'output/speech-benchmark/input-0.wav'
        text=subprocess.check_output([str(ROOT/'target/debug/examples/transcribe'),str(sample)],cwd=ROOT,env=env,text=True).strip()
        assert 'commercial' in text.lower() and 'asked' in text.lower(),text
        for case in ['09_cal','01_mara']:
            subprocess.run([str(ROOT/'target/debug/examples/synthesize'),str(ROOT/f'output/{case}-kokoro-preview.wav'),case,"I was trying to get ahead of the usual questions, officer. That is why I mentioned it. What would you like to know?"],cwd=ROOT,env=env,check=True)
        print(json.dumps(dict(transcript=text,health=health,rust_adapters='passed',voice_previews=2)))
    finally:
        process.terminate()
        try:process.wait(timeout=3)
        except subprocess.TimeoutExpired:process.kill()
