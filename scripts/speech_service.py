#!/usr/bin/env python3
"""Persistent, authenticated, loopback-only GPU speech service. No paid API keys required."""
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
MAX_AUDIO = 2_000_000

class Speech:
    def __init__(self):
        import torch
        import numpy as np
        import soundfile as sf
        from faster_whisper import WhisperModel
        from kokoro import KPipeline
        if not torch.cuda.is_available():
            raise RuntimeError('CUDA GPU unavailable. GPU speech requires a compatible NVIDIA driver; use the lightweight speech profile otherwise.')
        torch.set_num_threads(PROFILE['speech_cpu_threads'])
        torch.set_num_interop_threads(1)
        self.torch, self.np, self.sf = torch, np, sf
        self.lock = threading.Lock()
        self.before_used = torch.cuda.mem_get_info()[1]-torch.cuda.mem_get_info()[0]
        self.peak_delta = 0
        self.stt_name = os.environ.get('BORDER_STT_MODEL', PROFILE['stt_model'])
        print('Loading quantized Whisper Large-v3 Turbo…',file=sys.stderr,flush=True)
        self.asr = WhisperModel(self.stt_name, device='cuda', compute_type='int8_float16', cpu_threads=4, num_workers=1)
        print('Loading Kokoro neural voice model…',file=sys.stderr,flush=True)
        self.tts = KPipeline(lang_code='a', repo_id=PROFILE['tts_model'], device='cuda')
        self.voices = {False: os.environ.get('BORDER_TTS_VOICE','am_michael'), True: os.environ.get('BORDER_TTS_VOICE_ALTERNATE','af_heart')}
        for voice in self.voices.values():
            self.tts.load_voice(voice)
        print('Warming speech models…',file=sys.stderr,flush=True)
        # Initialize kernels before the first real interview.
        self.synthesize('Good evening, officer.',False)
        self.synthesize('Hello. How can I help you?',True)
        silence = io.BytesIO(); sf.write(silence,np.zeros(16000,dtype=np.float32),16000,format='WAV'); self.transcribe(silence.getvalue())
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
        return dict(ok=True,gpu_process_mib=gpu_process_mib,stt=self.stt_name,compute_type='int8_float16',tts=PROFILE['tts_model'],device=self.torch.cuda.get_device_name(),speech_vram_delta_mib=self.measure(),observed_peak_delta_mib=round(self.peak_delta/1024**2,1),rss_mib=round(rss,1) if rss else None,reference=PROFILE)

    def transcribe(self,data):
        started=time.monotonic()
        with self.lock:
            audio,sr=self.sf.read(io.BytesIO(data),dtype='float32')
            if audio.ndim>1: audio=audio.mean(axis=1)
            if not 0 < len(audio)/sr <= 15:
                raise ValueError('Audio must contain at most 15 seconds')
            if sr != 16000:
                import scipy.signal
                import math
                factor=math.gcd(sr,16000)
                audio=scipy.signal.resample_poly(audio,16000//factor,sr//factor).astype('float32')
            segments,info=self.asr.transcribe(audio,language='en',beam_size=PROFILE['stt_beam_size'],condition_on_previous_text=False,vad_filter=True,vad_parameters=dict(min_silence_duration_ms=300),temperature=0)
            text=' '.join(segment.text.strip() for segment in segments).strip()[:500]
            self.measure()
        return dict(text=text,latency_ms=round((time.monotonic()-started)*1000))

    def synthesize(self,text,alternate):
        if not isinstance(text,str) or not text.strip() or len(text)>1000:
            raise ValueError('Speech text must contain 1–1000 characters')
        with self.lock, self.torch.inference_mode():
            chunks=[audio.detach().cpu().numpy() for _,_,audio in self.tts(text,voice=self.voices[bool(alternate)],speed=1.0,split_pattern=r'\n+') if audio is not None]
            if not chunks: raise ValueError('No speech generated')
            audio=self.np.concatenate(chunks)
            output=io.BytesIO();self.sf.write(output,audio,24000,format='WAV',subtype='PCM_16')
            self.measure()
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
                    req=json.loads(body);self.send(200,backend.synthesize(req['text'],req.get('alternate',False)),'audio/wav')
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
            raise SystemExit('Speech exceeded the 3 GiB reference budget; keep the lightweight profile until tuned.')
        (ROOT/'.tools/gpu-speech-ready.json').write_text(json.dumps(report,indent=2))
    else:
        token=os.environ.get('BORDER_SPEECH_TOKEN')
        if not token:raise SystemExit('BORDER_SPEECH_TOKEN is required')
        serve(backend,args.port,token)
