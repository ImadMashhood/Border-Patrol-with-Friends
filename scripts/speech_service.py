#!/usr/bin/env python3
"""Persistent, authenticated, loopback-only local speech service. No paid API keys required."""
import argparse
import hmac
import io
import json
import os
import pathlib
import sys
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = pathlib.Path(__file__).resolve().parents[1]
PROFILE = json.loads((ROOT/'assets/settings/reference_hardware.json').read_text())
VOICES = json.loads((ROOT/'assets/settings/traveler_voices.json').read_text())
MAX_AUDIO = 2_000_000

class Speech:
    def __init__(self):
        import torch
        import numpy as np
        import soundfile as sf
        from faster_whisper import WhisperModel
        if not torch.cuda.is_available():
            raise RuntimeError('CUDA GPU unavailable. GPU speech requires a compatible NVIDIA driver; use the lightweight speech profile otherwise.')
        torch.set_num_threads(PROFILE['speech_cpu_threads'])
        torch.set_num_interop_threads(1)
        self.torch, self.np, self.sf = torch, np, sf
        self.lock = threading.Lock()
        self.before_used = torch.cuda.mem_get_info()[1]-torch.cuda.mem_get_info()[0]
        self.peak_delta = 0
        self.stt_name = os.environ.get('BORDER_STT_MODEL', PROFILE['stt_model'])
        print('Loading Whisper '+self.stt_name+'…',file=sys.stderr,flush=True)
        self.stt_device=os.environ.get('BORDER_STT_DEVICE',PROFILE.get('stt_device','cpu'))
        self.stt_compute='int8_float16' if self.stt_device=='cuda' else 'int8'
        self.asr = WhisperModel(self.stt_name,device=self.stt_device,compute_type=self.stt_compute,cpu_threads=4,num_workers=1)
        self.engine=os.environ.get('BORDER_TTS_ENGINE',PROFILE.get('tts_engine','pocket'))
        if self.engine=='pocket':
            from pocket_tts import TTSModel
            print('Loading Pocket TTS on CPU…',file=sys.stderr,flush=True)
            self.tts_name='kyutai/pocket-tts-without-voice-cloning:english_2026-10'
            self.tts=TTSModel.load_model(language='english_2026-10')
            # Public preset states need no voice-cloning credentials or audio samples.
            self.pocket_voices={voice:self.tts.get_state_for_audio_prompt(voice) for voice in sorted({p['pocket_voice'] for p in VOICES.values()})}
        elif self.engine=='qwen':
            from qwen_tts import Qwen3TTSModel
            print('Loading expressive Qwen3-TTS with CPU text embeddings and audio decoder…',file=sys.stderr,flush=True)
            self.tts_name='Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice'
            self.tts=Qwen3TTSModel.from_pretrained(self.tts_name,device_map='cuda:0',dtype=torch.float16,attn_implementation='sdpa')
            # Keep full-precision acoustic weights. Only text lookup and final waveform
            # decoding use CPU; this saves GPU memory without lossy weight compression.
            class CPUTextEmbedding(torch.nn.Module):
                def __init__(self,embedding):
                    super().__init__()
                    self.register_buffer('weight',embedding.weight.detach().cpu())
                def forward(self,ids):
                    return torch.nn.functional.embedding(ids.cpu(),self.weight).to(ids.device)
            self.tts.model.talker.model.text_embedding=CPUTextEmbedding(self.tts.model.talker.model.text_embedding)
            tokenizer=self.tts.model.speech_tokenizer
            tokenizer.model.to('cpu',dtype=torch.float32)
            tokenizer.device=torch.device('cpu')
            torch.cuda.empty_cache()
        elif self.engine=='kokoro':
            from kokoro import KPipeline
            self.tts_name='hexgrad/Kokoro-82M'
            self.tts=KPipeline(lang_code='a',repo_id=self.tts_name,device='cuda')
            self.voices={False:os.environ.get('BORDER_TTS_VOICE','am_michael'),True:os.environ.get('BORDER_TTS_VOICE_ALTERNATE','af_heart')}
            for voice in self.voices.values():self.tts.load_voice(voice)
        else:raise ValueError('BORDER_TTS_ENGINE must be pocket, qwen or kokoro')
        print('Warming speech models…',file=sys.stderr,flush=True)
        # Initialize kernels before the first real interview.
        self.synthesize('Good evening, officer.',False)
        self.synthesize('Hello. How can I help you?',True)
        silence = io.BytesIO(); sf.write(silence,np.zeros(16000,dtype=np.float32),16000,format='WAV'); self.transcribe(silence.getvalue())
        self.torch.cuda.empty_cache()
        self.measure()
        self.ready = True

    def measure(self):
        free,total = self.torch.cuda.mem_get_info()
        delta = max(0,total-free-self.before_used)
        self.peak_delta = max(self.peak_delta,delta)
        return round(delta/1024**2,1)

    def health(self):
        rss = None
        try:
            rss = int(next(l.split()[1] for l in pathlib.Path('/proc/self/status').read_text().splitlines() if l.startswith('VmRSS:')))/1024
        except (OSError,StopIteration):
            pass
        gpu_process_mib = None
        try:
            rows = subprocess.check_output(['nvidia-smi','--query-compute-apps=pid,used_gpu_memory','--format=csv,noheader,nounits'],text=True,timeout=2).splitlines()
            gpu_process_mib = next(float(row.split(',')[1].strip()) for row in rows if int(row.split(',')[0].strip()) == os.getpid())
        except (OSError,ValueError,StopIteration,subprocess.SubprocessError):
            pass
        return dict(ok=True,gpu_process_mib=gpu_process_mib,stt=self.stt_name,compute_type=self.stt_compute,stt_device=self.stt_device,tts=self.tts_name,tts_engine=self.engine,tts_device='cpu' if self.engine=='pocket' else 'cuda',tts_instruction_control=self.engine=='qwen',voice_profiles=len(VOICES),device=self.torch.cuda.get_device_name(),speech_vram_delta_mib=self.measure(),observed_peak_delta_mib=round(self.peak_delta/1024**2,1),rss_mib=round(rss,1) if rss else None,reference=PROFILE)

    def transcribe(self,data):
        started=time.monotonic()
        with self.lock:
            audio,sr=self.sf.read(io.BytesIO(data),dtype='float32')
            if audio.ndim>1: audio=audio.mean(axis=1)
            if not 0 < len(audio)/sr <= 22:
                raise ValueError('Audio must contain at most 22 seconds (20 second utterance plus capture padding)')
            if sr != 16000:
                import scipy.signal
                import math
                factor=math.gcd(sr,16000)
                audio=scipy.signal.resample_poly(audio,16000//factor,sr//factor).astype('float32')
            segments,info=self.asr.transcribe(audio,language='en',beam_size=PROFILE['stt_beam_size'],condition_on_previous_text=False,vad_filter=True,vad_parameters=dict(min_silence_duration_ms=300),temperature=0)
            text=' '.join(segment.text.strip() for segment in segments).strip()[:500]
            self.measure()
        return dict(text=text,latency_ms=round((time.monotonic()-started)*1000))

    def synthesize(self,text,alternate=False,scenario_id=None,emotion='cooperative'):
        if not isinstance(text,str) or not text.strip() or len(text)>1000:
            raise ValueError('Speech text must contain 1–1000 characters')
        acting={
            'cooperative':'Natural conversational expression. Warmth, dry humor or sarcasm follows the character.',
            'confused':'Hesitant, puzzled, small pauses and questioning inflection.',
            'suspicious':'Wary, guarded, probing and skeptical.',
            'irritated':'Impatient, clipped, audibly annoyed; bite on stressed words.',
            'defensive':'Defensive and tense, quick explanations and stressed denials.',
            'angry':'An angry raised voice, forceful sharp bursts; shout the emphatic words.',
            'disengaged':'Tired and dismissive, low effort without sounding robotic.',
            'afraid':'Frightened, shaky breath, urgent pleading, trying to stay composed.',
            'panicked':'Frightened, urgent, raised voice. Pronounce every word clearly.',
            'desperate':'Desperate, urgent pleading with emotional stress; keep every word clear.',
        }
        if emotion not in acting:raise ValueError('Unsupported speech emotion')
        profile=VOICES.get(scenario_id or '',VOICES['01_mara' if alternate else '09_cal'])
        started=time.monotonic()
        with self.lock,(self.torch.no_grad() if self.engine=='pocket' else self.torch.inference_mode()):
            if self.engine=='pocket':
                # Never mutate the cached speaker state across turns/characters.
                # Pocket infers prosody from words and punctuation; it has no
                # natural-language accent/emotion instruction API.
                self.torch.manual_seed(profile['seed'])
                audio=self.tts.generate_audio(self.pocket_voices[profile['pocket_voice']],text,copy_state=True).detach().cpu().numpy()
                sr=self.tts.sample_rate
            elif self.engine=='qwen':
                self.torch.manual_seed(profile['seed'])
                cry=text.strip().upper() in ('AAAH!','AAAAH!','AHHH!','AAAH','AAAAH','AHHH')
                instruction=profile['description']+' '+acting[emotion]+(' A real alarmed scream, not the spelled letters.' if cry else '')+' Speak only the given words, at a conversational pace with short pauses and no prolonged silences.'
                waves,sr=self.tts.generate_custom_voice(text=text,language='English',speaker=profile['speaker'],instruct=instruction,max_new_tokens=min(900,max(64,len(text.split())*6+24)),temperature=0.7,top_p=0.9,repetition_penalty=1.05)
                audio=waves[0]
            else:
                chunks=[a.detach().cpu().numpy() for _,_,a in self.tts(text,voice=self.voices[bool(alternate)],speed=1.0,split_pattern=r'\n+') if a is not None]
                if not chunks:raise ValueError('No speech generated')
                audio=self.np.concatenate(chunks);sr=24000
            # Remove only long leading/trailing silence; preserve expressive internal pauses.
            audio=self.np.asarray(audio,dtype=self.np.float32)
            audible=self.np.flatnonzero(self.np.abs(audio)>0.006)
            if len(audible):audio=audio[max(0,audible[0]-int(sr*.08)):min(len(audio),audible[-1]+int(sr*.16))]
            peak=float(self.np.max(self.np.abs(audio))) if len(audio) else 0
            if peak>0.98:audio=audio*(0.98/peak)
            output=io.BytesIO();self.sf.write(output,audio,sr,format='WAV',subtype='PCM_16')
            self.torch.cuda.empty_cache()
            self.measure()
        print(json.dumps(dict(event='tts',engine=self.engine,scenario=scenario_id,emotion=emotion,latency_s=round(time.monotonic()-started,3),audio_s=round(len(audio)/sr,3))),file=sys.stderr,flush=True)
        return output.getvalue()

def create_server(backend,port,token):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self,*args): pass
        def send(self,code,body,kind='application/json'):
            if kind=='application/json': body=json.dumps(body).encode()
            self.send_response(code);self.send_header('Content-Type',kind);self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
        def do_GET(self):
            if self.path=='/health':self.send(200,backend.health())
            else:self.send(404,{'error':'Unknown endpoint'})
        def do_POST(self):
            if not hmac.compare_digest(self.headers.get('Authorization',''),'Bearer '+token):
                self.send(401,{'error':'Invalid local speech token'});return
            try:
                length=int(self.headers.get('Content-Length','0'))
                if not 0<length<=MAX_AUDIO:raise ValueError('Request body too large or empty')
                body=self.rfile.read(length)
                if self.path=='/stt':self.send(200,backend.transcribe(body))
                elif self.path=='/tts':
                    req=json.loads(body);self.send(200,backend.synthesize(req['text'],req.get('alternate',False),scenario_id=req.get('scenario_id'),emotion=req.get('emotion','cooperative')),'audio/wav')
                else:self.send(404,{'error':'Unknown endpoint'})
            except (ValueError,KeyError) as exc:self.send(400,{'error':str(exc)})
            except Exception as exc:
                print('Speech inference error: '+type(exc).__name__,file=sys.stderr,flush=True)
                self.send(503,{'error':'Speech inference failed; see speech-service.log'})
    return ThreadingHTTPServer(('127.0.0.1',port),Handler)

def serve(backend,port,token):
    server=create_server(backend,port,token)
    print(json.dumps(backend.health()),flush=True)
    server.serve_forever()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--port',type=int,default=3848);p.add_argument('--check',action='store_true');args=p.parse_args()
    backend=Speech()
    if args.check:
        report=backend.health()
        print(json.dumps(report),flush=True)
        used=report['gpu_process_mib'] or report['observed_peak_delta_mib']+512
        if used > PROFILE['speech_vram_budget_mib']:
            raise SystemExit('Speech exceeded the configured reference budget; use BORDER_TTS_ENGINE=kokoro or the lightweight profile.')
        (ROOT/'.tools/gpu-speech-ready.json').write_text(json.dumps(report,indent=2))
    else:
        token=os.environ.get('BORDER_SPEECH_TOKEN')
        if not token:raise SystemExit('BORDER_SPEECH_TOKEN is required')
        serve(backend,args.port,token)
