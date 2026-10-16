import json,struct,glob,sys,math,bisect,statistics as st
root=glob.glob(sys.argv[1]+'/*/')[0]
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    v=struct.unpack('<%dd'%n, body[:n*8]); return [v[i:i+nc] for i in range(0,n,nc)]
M=readbin(root+'stream/motion.bin'); R=readbin(root+'stream/raw_accel.bin'); TR=[r[0] for r in R]
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
imps=[e['t'] for e in ev if e['type']=='impact']; blocks=[e for e in ev if e['type']=='block_start']
def block_at(t): return [b['block'] for b in blocks if b['t']<=t][-1]
def clipped_before(t):
    # was the last swing before t a clipping one (full-type)?
    p=[x for x in imps if x<t]; return p[-1] if p else None
# still samples by rotation (< 0.1 rad/s), grouped by time since the last impact
bins={}
for r in M:
    w=math.sqrt(r[11]**2+r[12]**2+r[13]**2)
    if w>=0.1: continue
    li=clipped_before(r[0])
    if li is None: key='before any swing'
    else:
        dt=r[0]-li
        if dt<1.0: continue
        b=block_at(li); fam='full-type' if b in('practice','full','hard','three_quarter','fade','draw') else 'chip/putt'
        key=(fam, '1-2 s' if dt<2 else '2-3 s' if dt<3 else '3-5 s' if dt<5 else '5-10 s' if dt<10 else '10+ s')
    ua=math.sqrt(r[5]**2+r[6]**2+r[7]**2)
    i=bisect.bisect_left(TR,r[0]); i=min(i,len(R)-1); q=R[i]
    rawdev=abs(math.sqrt(q[1]**2+q[2]**2+q[3]**2)-1)
    bins.setdefault(key,[]).append((ua,rawdev))
print('samples where the phone is NOT rotating (< 0.1 rad/s), by time since the last swing:')
print('  fused "user acceleration" (what the detector uses)  vs  raw accelerometer |a| - 1 g (no attitude estimate needed)')
order=['before any swing']+[(f,b) for f in ('full-type','chip/putt') for b in ('1-2 s','2-3 s','3-5 s','5-10 s','10+ s')]
for k in order:
    v=bins.get(k)
    if not v: continue
    ua=sorted(x[0] for x in v); rd=sorted(x[1] for x in v)
    print(f"  {str(k):28} n={len(v):5}  user acc median {st.median(ua):.3f} g (share >= 0.06: {sum(1 for x in ua if x>=0.06)/len(v)*100:3.0f}%)   raw |a|-1 median {st.median(rd):.3f} g (share >= 0.06: {sum(1 for x in rd if x>=0.06)/len(v)*100:3.0f}%)")
