#!/usr/bin/env python3
"""Render the checkpoint at 1080p beside resident speech; sample memory on Linux/NVIDIA."""
import json,pathlib,subprocess,time,struct
ROOT=pathlib.Path(__file__).resolve().parents[1]
OUT=ROOT/'output';OUT.mkdir(exist_ok=True)
def gpu_used():return int(subprocess.check_output(['nvidia-smi','--query-gpu=memory.used','--format=csv,noheader,nounits'],text=True).splitlines()[0])
def children(pid):
    found=[pid]
    try:
        for child in pathlib.Path(f'/proc/{pid}/task/{pid}/children').read_text().split():found+=children(int(child))
    except OSError:pass
    return found
baseline=gpu_used();samples=[]
with open(OUT/'reference-runtime.log','w') as log:
    proc=subprocess.Popen([str(ROOT/'play.sh'),'--smoke-interview','--reference-hardware'],cwd=ROOT,stdout=log,stderr=log)
    while proc.poll() is None:
        pids=children(proc.pid);rss=0;game=False
        for pid in pids:
            try:
                name=pathlib.Path(f'/proc/{pid}/comm').read_text().strip()
                game|=name=='border-game'
                status=pathlib.Path(f'/proc/{pid}/status').read_text()
                rss+=int(next(l.split()[1] for l in status.splitlines() if l.startswith('VmRSS:')))/1024
            except (OSError,StopIteration):pass
        if game:samples.append(dict(combined_rss_mib=round(rss,1),device_used_mib=gpu_used()))
        time.sleep(.25)
    if proc.returncode:raise SystemExit('Reference runtime failed; see output/reference-runtime.log')
assert samples,'No game runtime samples captured'
report=dict(reference=json.loads((ROOT/'assets/settings/reference_hardware.json').read_text()),samples=len(samples),baseline_device_mib=baseline,whole_device_peak_mib=max(s['device_used_mib'] for s in samples),incremental_device_peak_mib=max(s['device_used_mib'] for s in samples)-baseline,combined_process_peak_rss_mib=max(s['combined_rss_mib'] for s in samples),note='Measured on RTX 3080, not an RTX 2060 certification or a 60 FPS measurement.')
report['actual_capture_resolution']=list(struct.unpack('>II',(OUT/'interview.png').read_bytes()[16:24]))
(OUT/'reference-runtime.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
