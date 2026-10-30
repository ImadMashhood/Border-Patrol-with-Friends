"""HTTP boundary checks without downloading models or recording a microphone."""
import json
import threading
import unittest
import urllib.error
import urllib.request
from speech_service import create_server

class FakeSpeech:
    def health(self): return dict(ok=True)
    def transcribe(self,data): return dict(text='What is your name?')
    def synthesize(self,text,alternate,scenario_id=None,emotion="cooperative"):
        self.last=(scenario_id,emotion)
        return b'RIFF'+b'\0'*44

class BoundaryTests(unittest.TestCase):
    def setUp(self):
        self.backend=FakeSpeech()
        self.server=create_server(self.backend,0,'test-token')
        self.thread=threading.Thread(target=self.server.serve_forever,daemon=True);self.thread.start()
        self.url=f'http://127.0.0.1:{self.server.server_port}'
    def tearDown(self): self.server.shutdown();self.server.server_close();self.thread.join()
    def request(self,path,data,token='test-token'):
        return urllib.request.urlopen(urllib.request.Request(self.url+path,data=data,headers={'Authorization':'Bearer '+token}),timeout=2)
    def test_authentication_and_audio_response(self):
        with self.assertRaises(urllib.error.HTTPError) as e:self.request('/stt',b'audio','wrong-token')
        self.assertEqual(e.exception.code,401);e.exception.close()
        with self.request('/stt',b'audio') as response:self.assertEqual(json.load(response)['text'],'What is your name?')
        with self.request('/tts',json.dumps(dict(text='Hello')).encode()) as response:self.assertEqual(response.read()[:4],b'RIFF')
    def test_character_and_emotion_are_forwarded_to_the_voice_backend(self):
        with self.request('/tts',json.dumps(dict(text='Hey! Back off!',scenario_id='09_cal',emotion='angry')).encode()) as response:
            self.assertEqual(response.read()[:4],b'RIFF')
        self.assertEqual(self.backend.last,('09_cal','angry'))
    def test_empty_unknown_and_malformed_requests(self):
        for path,body,code in [('/stt',b'',400),('/tts',b'not json',400),('/unknown',b'body',404)]:
            with self.assertRaises(urllib.error.HTTPError) as e:self.request(path,body)
            self.assertEqual(e.exception.code,code);e.exception.close()

if __name__=='__main__':unittest.main()
