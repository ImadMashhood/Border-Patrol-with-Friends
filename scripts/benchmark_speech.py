#!/usr/bin/env python3
"""Local synthetic speech regression/latency benchmark. No paid API requests or room capture."""
import io
import argparse
import json
import pathlib
import re
import subprocess
import time
import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from speech_service import Speech

ROOT=pathlib.Path(__file__).resolve().parents[1]
OUT=ROOT/'output/speech-benchmark';OUT.mkdir(parents=True,exist_ok=True)
PHRASES=[
'Why would you mention commercial goods before I asked?',
'What is your name, and where are you traveling?',
'Are those forty boxed automotive parts intended for resale?',
'I asked about your answer, not the contents of your trunk.',
'Does the invoice match the automotive parts in your car?',
'You seem nervous, but that does not mean you are smuggling.'
]

def words(text):
    text=re.sub(r"\bmr\b", "mister", text.lower())
    return re.findall(r"[a-z]+",text.replace('40','forty').replace("you're",'you are').replace("doesn't",'does not').replace("what's",'what is'))
def distance(a,b):
    row=list(range(len(b)+1))
    for i,x in enumerate(a,1):
        new=[i]
        for j,y in enumerate(b,1):new.append(min(new[-1]+1,row[j]+1,row[j-1]+(x!=y)))
        row=new
    return row[-1]

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--human-parquet');args=parser.parse_args()
    speech=Speech();results=[];rng=np.random.default_rng(42)
    cases=[]
    if args.human_parquet:
        import pyarrow.parquet as pq
        OUT=OUT/'human';OUT.mkdir(exist_ok=True)
        for row in pq.read_table(args.human_parquet).to_pylist():
            wave=row['audio']['bytes'];audio,sr=sf.read(io.BytesIO(wave),dtype='float32')
            if 1<len(audio)/sr<=12:
                cases.append((row['text'],audio,sr))
            if len(cases)==6:break
        assert len(cases)==6
    else:cases=[(text,None,None) for text in PHRASES]
    for i,(text,provided_audio,provided_sr) in enumerate(cases):
        if provided_audio is None:
            start=time.monotonic();wave=speech.synthesize(text,i%2==1);tts_s=time.monotonic()-start
            audio,sr=sf.read(io.BytesIO(wave),dtype='float32');sf.write(OUT/f'voice-{i}.wav',audio,sr)
        else:audio,sr,tts_s=provided_audio,provided_sr,0
        import math
        factor=math.gcd(sr,16000)
        audio=resample_poly(audio,16000//factor,sr//factor).astype('float32')
        if provided_audio is None and i>=3:
            rms=np.sqrt(np.mean(audio**2));audio=audio+rng.normal(0,rms/(10**(12/20)),len(audio)).astype('float32')
        path=OUT/f'input-{i}.wav';sf.write(path,audio,16000)
        start=time.monotonic();new=speech.transcribe(path.read_bytes());new_s=time.monotonic()-start
        row=dict(reference=text,condition='human LibriSpeech sample' if provided_audio is not None else ('12dB synthetic noise' if i>=3 else 'clean synthetic voice'),new_text=new['text'],stt_s=round(new_s,3),tts_s=round(tts_s,3),audio_s=round(len(audio)/16000,3),new_errors=distance(words(text),words(new['text'])),words=len(words(text)))
        legacy=ROOT/'.tools/whisper/build/bin/whisper-cli';model=ROOT/'.tools/whisper/models/ggml-tiny.en.bin'
        if legacy.exists() and model.exists():
            output=OUT/f'tiny-{i}';start=time.monotonic()
            subprocess.run([str(legacy),'-m',str(model),'-f',str(path),'-otxt','-of',str(output),'-nt','-l','en','-t','4'],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=30)
            old=pathlib.Path(str(output)+'.txt').read_text().strip();row.update(old_text=old,tiny_stt_s=round(time.monotonic()-start,3),tiny_errors=distance(words(text),words(old)))
        results.append(row);print(json.dumps(row),flush=True)
    total=sum(r['words'] for r in results)
    summary=dict(hardware=speech.health(),samples=results,new_wer=sum(r['new_errors'] for r in results)/total,tiny_wer=sum(r.get('tiny_errors',0) for r in results)/total)
    (OUT/'results.json').write_text(json.dumps(summary,indent=2));print(json.dumps({k:v for k,v in summary.items() if k!='samples'}),flush=True)
