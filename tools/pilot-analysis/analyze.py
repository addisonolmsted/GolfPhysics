import json,struct,glob,os,sys,collections,math
root=glob.glob(sys.argv[1]+'/*/')[0]
sess=json.load(open(root+'session.json'))
print('session', sess['participantCode'], sess['deviceModel'], sess['osVersion'], 'protocol', sess['protocolId'], sess['protocolVersion'], 'shapes', sess.get('shapeBlocks'), 'completed', sess['completed'])
print('survey', {k:v for k,v in sess['survey'].items() if v})
st=collections.Counter(w['status'] for w in sess['swings']); print('swings in session.json', dict(st))
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8
    vals=struct.unpack('<%dd'%n, body[:n*8]); rows=[vals[i*nc:(i+1)*nc] for i in range(n//nc)]
    return h,rows,n%nc
for name in ['motion','raw_accel','raw_gyro']:
    h,rows,rem=readbin(root+'stream/%s.bin'%name)
    ts=[r[0] for r in rows]; dts=[b-a for a,b in zip(ts,ts[1:])]
    nonmono=sum(1 for x in dts if x<=0)
    big=[(a,b) for a,b in zip(ts,ts[1:]) if b-a>0.1]
    dts_sorted=sorted(dts)
    print(f'{name:10} rows {len(rows):7d} rem {rem} span {ts[-1]-ts[0]:7.1f}s  median rate {1/dts_sorted[len(dts)//2]:6.1f} Hz  p99 dt {dts_sorted[int(len(dts)*0.99)]*1000:5.1f} ms  non-monotonic {nonmono}  gaps>0.1s {len(big)}')
    if name=='motion': motion=rows
    if name=='raw_gyro': gyro=rows
    if name=='raw_accel': acc=rows
gmax=max(max(abs(r[1]),abs(r[2]),abs(r[3])) for r in gyro); amax=max(max(abs(r[1]),abs(r[2]),abs(r[3])) for r in acc)
print(f'raw gyro peak component {gmax:.1f} rad/s ({math.degrees(gmax):.0f} deg/s); raw accel peak component {amax:.2f} g')
# clipping: count samples at the peak plateau
gclip=sum(1 for r in gyro if max(abs(r[1]),abs(r[2]),abs(r[3]))>=gmax*0.995); aclip=sum(1 for r in acc if max(abs(r[1]),abs(r[2]),abs(r[3]))>=7.8)
print('gyro samples within 0.5% of its peak', gclip, '| accel samples >= 7.8 g', aclip)
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
print('\nevents', len(ev), dict(collections.Counter(e['type'] for e in ev)))
print('disarm reasons', dict(collections.Counter(e.get('reason') for e in ev if e['type']=='disarmed')))
t0=ev[0]['t']
print('\ntimeline (minutes:seconds from capture start), compressed:')
last=None
for e in ev:
    ty=e['type']
    if ty in ('settle_start','settle_broken','backswing_start','wobble'): continue
    extra=''
    if ty=='block_start': extra=e['block']
    elif ty=='disarmed': extra=e['reason']+(f" (backswing {e['backswing_deg']:.0f} deg)" if 'backswing_deg' in e else '')
    elif ty=='impact': extra=f"back {e['backswing_deg']:.0f} deg, {e['omega']:.1f} rad/s, readdresses {e['readdresses']}"
    elif ty=='unarmed_swing': extra=f"peak {e['peak_omega']:.1f} rad/s, {e['end']-e['start']:.2f} s"
    elif ty=='gap': extra=f"{e['to']-e['from']:.1f} s"
    elif ty=='swing_saved': extra=f"{e['file']} [{e['status']}] {e['to']-e['from']:.1f} s"
    elif ty=='swing_deleted': extra=e['file']
    m=(e['t']-t0); print(f"  {int(m//60):2d}:{m%60:05.2f}  {ty:15} {extra}")
# settle stats
ss=[e for e in ev if e['type']=='settle_start']; sb=[e for e in ev if e['type']=='settle_broken']; ar=[e for e in ev if e['type']=='armed']
print('\nsettle starts', len(ss), 'broken', len(sb), 'armed', len(ar), 'readdressed', sum(1 for e in ev if e['type']=='readdressed'), 'wobbles', sum(1 for e in ev if e['type']=='wobble'))
# swing files vs events
files=sorted(os.listdir(root+'swings')); saved=[e for e in ev if e['type']=='swing_saved']
print('slab files', len(files), 'swing_saved events', len(saved), 'missing files', [e['file'] for e in saved if e['file'] not in files])
# slab check
bad=0
for f in files:
    d=open(root+'swings/'+f,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); n=(len(d)-8-L)//8
    if n!=h['motionRows']*17+h['rawRows']*4+h['gyroRows']*4 or h['formatVersion']!=2: bad+=1
print('slab files with a bad row count or format', bad)
