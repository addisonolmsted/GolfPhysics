import json,struct,glob,os,sys,collections,math
root=glob.glob(sys.argv[1]+'/*/')[0]
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    v=struct.unpack('<%dd'%n, body[:n*8]); return [v[i:i+nc] for i in range(0,n,nc)]
acc=readbin(root+'stream/raw_accel.bin'); gyro=readbin(root+'stream/raw_gyro.bin'); motion=readbin(root+'stream/motion.bin')
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
imp=[e for e in ev if e['type']=='impact']; blocks=[e for e in ev if e['type']=='block_start']
def block_at(t): return [b['block'] for b in blocks if b['t']<=t][-1]
print('per swing: accel samples >= 7.8 g, gyro samples >= 33 rad/s (1890 deg/s), fused rotation-rate peak, raw gyro peak')
rows=collections.defaultdict(list)
for e in imp:
    a=e['backswing_start']-0.2; b=e['t']+0.4
    ac=sum(1 for r in acc if a<=r[0]<=b and max(abs(r[1]),abs(r[2]),abs(r[3]))>=7.8)
    gy=[math.sqrt(r[1]**2+r[2]**2+r[3]**2) for r in gyro if a<=r[0]<=b]
    gc=sum(1 for r in gyro if a<=r[0]<=b and max(abs(r[1]),abs(r[2]),abs(r[3]))>=33)
    fm=max(math.sqrt(r[11]**2+r[12]**2+r[13]**2) for r in motion if a<=r[0]<=b)
    rows[block_at(e['t'])].append((ac,gc,fm,max(gy)))
for k,v in rows.items():
    print(f"  {k:14} n={len(v):2}  accel-clipped samples/swing {sum(x[0] for x in v)/len(v):5.1f} (swings with any {sum(1 for x in v if x[0])}/{len(v)})  gyro >=33 rad/s {sum(x[1] for x in v)/len(v):4.1f}/swing  peak |w| fused {max(x[2] for x in v):5.1f} raw {max(x[3] for x in v):5.1f} rad/s")
print('\nunarmed_swing events: peak rad/s, block')
for e in ev:
    if e['type']=='unarmed_swing': print(f"  {e['peak_omega']:5.1f}  {block_at(e['t'])}")
# per-axis raw gyro max: which axis hits the limit
mx=[max(abs(r[i]) for r in gyro) for i in (1,2,3)]; print('\nraw gyro max per axis (rad/s)', [round(x,2) for x in mx])
