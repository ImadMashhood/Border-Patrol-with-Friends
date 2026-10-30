#!/usr/bin/env python3
"""Compare the three small Pocket models on six fixed local utterances; no paid API."""
import os,torch,time,json,gc,soundfile as sf
from pocket_tts import TTSModel
from faster_whisper import WhisperModel
from scipy.signal import resample_poly
from pathlib import Path
root=Path(__file__).resolve().parents[1]; (root/"output").mkdir(exist_ok=True); voices=json.loads((root/'assets/settings/traveler_voices.json').read_text()); samples=[('09_cal',"C'mon, man. That's one hell of a way to ask."),('09_cal','Hey! Back off! Put that gun down!'),('07_amir','Wait! Please! I do not want to get hurt!'),('01_mara','Oh, lovely. Do you want the camera or the bag?'),('08_rosa',"Buddy, I heard you. I just don't like the question."),('10_priya','Right. Was that a question, or are you workshopping a joke?')]
torch.set_num_threads(2);torch.set_num_interop_threads(1)
asr=WhisperModel('small',device='cuda',compute_type='int8_float16');results=[]
for variant in ['english_18m_2026-10','english_7m_2026-10','english_2m_2026-10']:
 m=TTSModel.load_model(language=variant);states={v:m.get_state_for_audio_prompt(v) for v in {p['pocket_voice'] for p in voices.values()}}
 m.generate_audio(states['michael'],'Good evening, officer.',copy_state=True)
 rows=[]
 for case,text in samples:
  torch.manual_seed(voices[case]['seed']);t=time.monotonic();a=m.generate_audio(states[voices[case]['pocket_voice']],text,copy_state=True).numpy();elapsed=time.monotonic()-t
  sf.write(root/f'output/{variant}-{case}-{len(rows)}.wav',a,m.sample_rate)
  segs,_=asr.transcribe(resample_poly(a,2,3),language='en',beam_size=3,vad_filter=True,condition_on_previous_text=False)
  row=dict(case=case,text=text,transcript=' '.join(x.text.strip() for x in segs),tts_s=round(elapsed,3),audio_s=round(len(a)/m.sample_rate,3));rows.append(row);print(json.dumps(dict(model=variant,**row)),flush=True)
 results.append(dict(model=variant,threads=2,samples=rows));(root/'output/pocket-small-models.json').write_text(json.dumps(results,indent=2)+'\n');del m,states;gc.collect()
