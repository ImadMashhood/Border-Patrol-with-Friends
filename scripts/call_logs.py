#!/usr/bin/env python3
"""Read the latest play session's dialogue and speech diagnostics."""
import argparse,datetime,json,pathlib,time,sys
ROOT=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--follow','-f',action='store_true');p.add_argument('--json',action='store_true');p.add_argument('--directory',type=pathlib.Path);args=p.parse_args()
pointer=ROOT/'output/latest-call-log-dir.txt'
directory=args.directory or (pathlib.Path(pointer.read_text().strip()) if pointer.exists() else ROOT/'output/call-logs')
print('Call logs: '+str(directory),file=sys.stderr if args.json else sys.stdout,flush=True);seen=set()
try:
 while True:
  rows=[]
  for file in directory.glob('calls-*.jsonl*'):
   try:
    for i,line in enumerate(file.read_text().splitlines()):
     key=(str(file),i,line)
     if key in seen:continue
     try:row=json.loads(line)
     except ValueError:continue
     seen.add(key);rows.append(row)
   except OSError:continue
  for row in sorted(rows,key=lambda r:r.get('timestamp_ms',0)):
   if args.json:print(json.dumps(row),flush=True);continue
   d=row.get('details',{});stage=row['stage'];when=datetime.datetime.fromtimestamp(row['timestamp_ms']/1000).strftime('%H:%M:%S')
   text=d.get('officer') or d.get('traveler') or d.get('opening') or d.get('text') or d.get('message') or d.get('error') or ''
   info=' '.join(f'{k}={d[k]}' for k in ['status','latency_ms','elapsed_ms','retry_after_seconds','model','fallback'] if d.get(k) is not None)
   print(f'{when} [{stage}] {text} {info}'.rstrip(),flush=True)
  if not args.follow:break
  time.sleep(.4)
except KeyboardInterrupt:pass
