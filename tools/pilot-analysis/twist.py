import json,struct,glob,sys,math,statistics as st
root=glob.glob(sys.argv[1]+'/*/')[0]
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    v=struct.unpack('<%dd'%n, body[:n*8]); return [v[i:i+nc] for i in range(0,n,nc)]
M=readbin(root+'stream/motion.bin'); T=[r[0] for r in M]
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
blocks=[e for e in ev if e['type']=='block_start']
def block_at(t): return [b['block'] for b in blocks if b['t']<=t][-1]
import bisect
def q_at(t):
    i=bisect.bisect_left(T,t); a,b=M[i-1],M[i]; f=(t-a[0])/(b[0]-a[0])
    qa=a[1:5]; qb=b[1:5]
    if sum(x*y for x,y in zip(qa,qb))<0: qb=[-x for x in qb]
    q=[x+(y-x)*f for x,y in zip(qa,qb)]; n=math.sqrt(sum(x*x for x in q)); return [x/n for x in q]
def avg_q(t0,t1):
    qs=[r[1:5] for r in M if t0<=r[0]<=t1]; f=qs[0]; s=[0,0,0,0]
    for q in qs:
        if sum(x*y for x,y in zip(q,f))<0: q=[-x for x in q]
        s=[x+y for x,y in zip(s,q)]
    n=math.sqrt(sum(x*x for x in s)); return [x/n for x in s]
def mul(a,b):
    w1,x1,y1,z1=a; w2,x2,y2,z2=b
    return [w1*w2-x1*x2-y1*y2-z1*z2, w1*x2+x1*w2+y1*z2-z1*y2, w1*y2-x1*z2+y1*w2+z1*x2, w1*z2+x1*y2-y1*x2+z1*w2]
def conj(q): return [q[0],-q[1],-q[2],-q[3]]
def rot(q,v):
    p=mul(mul(q,[0]+list(v)),conj(q)); return p[1:]
res={}
armed=[e['t'] for e in ev if e['type'] in('armed','readdressed')]
for e in ev:
    if e['type']!='impact': continue
    b=block_at(e['t'])
    ta=max(x for x in armed if x<=e['backswing_start'])
    qa=avg_q(ta-0.3,ta); qi=q_at(e['t'])
    rel=mul(conj(qa),qi)   # rotation from address to impact, in the device's address frame
    twist=math.degrees(2*math.atan2(rel[2],rel[0]))  # about the phone's long axis (+y): the face opening or closing
    if twist>180: twist-=360
    if twist<-180: twist+=360
    res.setdefault(b,[]).append(twist)
print('twist of the phone about its long axis at impact, relative to address (degrees; the sign convention is not yet tied to open/closed):')
for b in ['practice','full','hard','three_quarter','fade','draw','chip25','chip40','putt4','putt10']:
    v=res.get(b,[])
    if v: print(f"  {b:14} n={len(v):2}  mean {st.mean(v):+6.1f}  sd {st.pstdev(v):5.1f}   {' '.join(f'{x:+.0f}' for x in v)}")
