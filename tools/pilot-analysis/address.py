import json,struct,glob,sys,math,bisect
root=glob.glob(sys.argv[1]+'/*/')[0]
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    v=struct.unpack('<%dd'%n, body[:n*8]); return [v[i:i+nc] for i in range(0,n,nc)]
M=readbin(root+'stream/motion.bin'); T=[r[0] for r in M]
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
def down(r):
    g=math.sqrt(r[8]**2+r[9]**2+r[10]**2); return math.degrees(math.acos(max(-1,min(1,r[9]/g))))
W=[math.sqrt(r[11]**2+r[12]**2+r[13]**2) for r in M]; A=[math.sqrt(r[5]**2+r[6]**2+r[7]**2) for r in M]; D=[down(r) for r in M]
# previous "end of activity": the impact's follow-through end (swing_saved) or capture start, or a disarm
ends=sorted([e['t'] for e in ev if e['type'] in ('swing_saved','disarmed','capture_start','swing_deleted')])
rows=[]
for e in ev:
    if e['type']!='armed': continue
    ta=e['t']; k=bisect.bisect_left(T,ta)
    # the continuous stretch before arming with the phone within 45 deg of down
    j=k
    while j>0 and D[j-1]<45: j-=1
    prev_end=max([x for x in ends if x<ta] or [T[0]])
    start=max(T[j],prev_end)
    i0=bisect.bisect_left(T,start)
    seg=range(i0,k)
    n=max(1,len(seg))
    fw=sum(1 for i in seg if W[i]>=0.35)/n; fa=sum(1 for i in seg if A[i]>=0.06)/n; fboth=sum(1 for i in seg if W[i]<0.35 and A[i]<0.06)/n
    sst=[x['t'] for x in ev if x['type']=='settle_start' and start<=x['t']<=ta]; sbr=[x['t'] for x in ev if x['type']=='settle_broken' and start<=x['t']<=ta]
    rows.append((ta, ta-start, fw, fa, fboth, len(sst), len(sbr), start, k))
t0=ev[0]['t']
print('each arm: time at address (down within 45 deg, since the previous swing ended) before it armed; share of those samples with')
print('rotation >= 0.35 rad/s, acceleration >= 0.06 g, both under; settle starts / breaks in that time')
for ta,dt,fw,fa,fb,ns,nb,st,k in rows:
    flag='  <-- long' if dt>1.6 else ''
    print(f"  {int((ta-t0)//60):2d}:{(ta-t0)%60:05.2f}  at address {dt:5.2f} s   w>=0.35 {fw*100:4.0f}%  acc>=0.06 {fa*100:4.0f}%  still {fb*100:4.0f}%   settles {ns} breaks {nb}{flag}")
long=[r for r in rows if r[1]>1.6]
print(f'\n{len(long)} of {len(rows)} arms took more than 1.6 s at address')
# for the long ones: the last 3 s before arming, in 0.1 s bins: down angle, max w, max acc
for ta,dt,fw,fa,fb,ns,nb,st,k in long[:5]:
    print(f"\narm at {int((ta-t0)//60)}:{(ta-t0)%60:05.2f}: last {min(dt,4):.1f} s before arming, 0.1 s bins (down deg / max rot rad/s / max acc g); * = a still sample")
    i0=bisect.bisect_left(T,ta-min(dt,4)); line=[]
    i=i0
    while i<k:
        j=min(k,i+10)
        line.append(f"{max(D[i:j]):3.0f}/{max(W[i:j]):4.2f}/{max(A[i:j]):4.3f}{'*' if any(W[x]<0.35 and A[x]<0.06 for x in range(i,j)) else ' '}")
        i=j
    for c in range(0,len(line),6): print('   '+'  '.join(line[c:c+6]))
