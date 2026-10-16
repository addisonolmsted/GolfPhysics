import json,struct,glob,sys,math,bisect,statistics as st
root=glob.glob(sys.argv[1]+'/*/')[0]
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    v=struct.unpack('<%dd'%n, body[:n*8]); return [v[i:i+nc] for i in range(0,n,nc)]
M=readbin(root+'stream/motion.bin'); TM=[r[0] for r in M]
G=readbin(root+'stream/raw_gyro.bin'); TG=[r[0] for r in G]
A=readbin(root+'stream/raw_accel.bin'); TA=[r[0] for r in A]
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
blocks=[e for e in ev if e['type']=='block_start']
def block_at(t): return [b['block'] for b in blocks if b['t']<=t][-1]
def mul(a,b):
    w1,x1,y1,z1=a; w2,x2,y2,z2=b
    return (w1*w2-x1*x2-y1*y2-z1*z2, w1*x2+x1*w2+y1*z2-z1*y2, w1*y2-x1*z2+y1*w2+z1*x2, w1*z2+x1*y2-y1*x2+z1*w2)
def conj(q): return (q[0],-q[1],-q[2],-q[3])
def norm(q): n=math.sqrt(sum(x*x for x in q)); return tuple(x/n for x in q)
def rot(q,v): return mul(mul(q,(0,)+tuple(v)),conj(q))[1:]
def ang(a,b): d=abs(sum(x*y for x,y in zip(a,b))); return math.degrees(2*math.acos(min(1,d)))
def vang(a,b):
    na=math.sqrt(sum(x*x for x in a)); nb=math.sqrt(sum(x*x for x in b)); return math.degrees(math.acos(max(-1,min(1,sum(x*y for x,y in zip(a,b))/(na*nb)))))
def qM(i): return norm(M[i][1:5])
def q_at(t):
    i=max(1,min(len(TM)-1,bisect.bisect_left(TM,t))); a,b=qM(i-1),qM(i); f=(t-TM[i-1])/(TM[i]-TM[i-1])
    if sum(x*y for x,y in zip(a,b))<0: b=tuple(-x for x in b)
    return norm(tuple(x+(y-x)*f for x,y in zip(a,b)))
def avg_q(t0,t1):
    i0,i1=bisect.bisect_left(TM,t0),bisect.bisect_left(TM,t1); f=qM(i0); s=[0,0,0,0]
    for i in range(i0,i1):
        q=qM(i)
        if sum(x*y for x,y in zip(q,f))<0: q=tuple(-x for x in q)
        s=[x+y for x,y in zip(s,q)]
    return norm(s)
# convention check: CoreMotion's gravity (device frame) vs its attitude's down, at still moments
chk=[vang(M[i][8:11], rot(conj(qM(i)),(0,0,-1))) for i in range(0,len(M),500)]
print('convention check: gravity vs attitude-derived down, median deg', round(st.median(chk),3))
def integrate(q0, t0, t1, bias, sign=1):
    """the raw gyro (device-frame rates, bias removed) from orientation q0 at t0; yields (t, q)"""
    i=bisect.bisect_left(TG,t0); q=q0; t=t0; out=[(t,q)]
    while i<len(G) and G[i][0]<=t1:
        w=[G[i][k+1]-bias[k] for k in range(3)]; dt=G[i][0]-t; t=G[i][0]
        th=math.sqrt(sum(x*x for x in w))*dt
        if th>0:
            ax=[x/math.sqrt(sum(y*y for y in w)) for x in w]; dq=(math.cos(th/2),)+tuple(sign*a*math.sin(th/2) for a in ax)
            q=norm(mul(q,dq))
        out.append((t,q)); i+=1
    return out
def still_after(t, dur=0.25):
    """the first moment after t the phone is still for dur (raw gyro < 0.08 rad/s, raw |a| within 0.02 g of 1)"""
    i=bisect.bisect_left(TG,t); run=None
    while i<len(G):
        r=G[i]; w=math.sqrt(r[1]**2+r[2]**2+r[3]**2)
        j=min(len(A)-1,bisect.bisect_left(TA,r[0])); a=A[j]; dev=abs(math.sqrt(a[1]**2+a[2]**2+a[3]**2)-1)
        if w<0.08 and dev<0.02:
            run=run or r[0]
            if r[0]-run>=dur: return r[0]
        else: run=None
        i+=1
    return None
def raw_gravity(t0,t1):
    i0,i1=bisect.bisect_left(TA,t0),bisect.bisect_left(TA,t1); s=[0,0,0]
    for i in range(i0,i1): s=[x+y for x,y in zip(s,A[i][1:4])]
    return s
imps=[e for e in ev if e['type']=='impact']
arms=sorted(e['t'] for e in ev if e['type'] in ('armed','readdressed'))
swings=[]
for sign in (1,-1):
    e=imps[30]; ta=max(x for x in arms if x<=e['backswing_start']); q0=avg_q(ta-0.3,ta)
    i0,i1=bisect.bisect_left(TG,ta-0.6),bisect.bisect_left(TG,ta)
    bias=[st.mean(G[i][k] for i in range(i0,i1)) for k in (1,2,3)]
    tr=integrate(q0,ta,e['t'],bias,sign); print('sign',sign,'diff at impact deg', round(ang(tr[-1][1], q_at(e['t'])),2))

def fused_rot(t):
    i=max(1,min(len(TM)-1,bisect.bisect_left(TM,t))); a,b=M[i-1],M[i]; f=(t-a[0])/(b[0]-a[0])
    return [a[11+k]+(b[11+k]-a[11+k])*f for k in range(3)]
def bias_at(t, span=2.0):
    # the raw gyro's offset: raw minus CoreMotion's bias-corrected rate, averaged over quiet samples (< 0.5 rad/s)
    # in the span before t
    i0,i1=bisect.bisect_left(TG,t-span),bisect.bisect_left(TG,t); d=[[],[],[]]
    for i in range(i0,i1):
        r=G[i]
        if math.sqrt(r[1]**2+r[2]**2+r[3]**2)>0.5: continue
        f=fused_rot(r[0])
        for k in range(3): d[k].append(r[k+1]-f[k])
    return [st.median(x) if x else 0 for x in d]
def arc(a,b):
    """the shortest rotation taking unit vector a to b"""
    na=math.sqrt(sum(x*x for x in a)); nb=math.sqrt(sum(x*x for x in b)); a=[x/na for x in a]; b=[x/nb for x in b]
    c=[a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]; d=sum(x*y for x,y in zip(a,b))
    return norm((1+d,)+tuple(c))
def twist(q0,q):
    r=mul(conj(q0),q); t=math.degrees(2*math.atan2(r[2],r[0]))
    return (t+180)%360-180
out=[]
for n,e in enumerate(imps):
    ta=max(x for x in arms if x<=e['backswing_start']); ti=e['t']
    q0=avg_q(ta-0.3,ta)
    bias=bias_at(ta)
    gm=raw_gravity(ta-0.3,ta); gp=rot(conj(q0),(0,0,-1))
    addr_err=vang(gm,gp)
    q0c=norm(mul(q0,arc(gm,gp)))   # the address tilted so its down matches the raw accelerometer's
    ts=still_after(ti+0.5); ts = ts if ts and ts-ti<20 else None
    tend=(ts if ts else ti+1.0)
    tr_cm=integrate(q0,ta,tend,bias); tr_c=integrate(q0c,ta,tend,bias)
    Tr=[x[0] for x in tr_cm]
    def at(tr,t): return tr[min(len(tr)-1,bisect.bisect_left(Tr,t))][1]
    peak=max(math.sqrt(r[1]**2+r[2]**2+r[3]**2) for r in G[bisect.bisect_left(TG,e['backswing_start']):bisect.bisect_left(TG,ti+0.3)])
    gclip=sum(1 for r in G[bisect.bisect_left(TG,e['backswing_start']):bisect.bisect_left(TG,ti+0.5)] if max(abs(r[1]),abs(r[2]),abs(r[3]))>=34.0)
    aclip=sum(1 for r in A[bisect.bisect_left(TA,e['backswing_start']):bisect.bisect_left(TA,ti+0.5)] if max(abs(r[1]),abs(r[2]),abs(r[3]))>=7.8)
    rec_bias=[round(x,4) for x in bias]
    rec={'bias':rec_bias,'n':n,'block':block_at(ti),'peak':round(peak,2),'gyro_clip':gclip,'acc_clip':aclip,'backswing':round(e['backswing_deg']),
         'addr_err':round(addr_err,2),
         'impact_diff':round(ang(at(tr_cm,ti),q_at(ti)),2),          # raw gyro (from CoreMotion's address) vs CoreMotion, at impact
         'twist_cm':round(twist(q0,q_at(ti)),1),'twist_raw':round(twist(q0,at(tr_cm,ti)),1),
         'still_after':round(ts-ti,2) if ts else None}
    if ts:
        gt=raw_gravity(ts-0.25,ts)
        rec['truth_cm']=round(vang(gt, rot(conj(q_at(ts)),(0,0,-1))),2)
        rec['truth_raw']=round(vang(gt, rot(conj(at(tr_c,ts)),(0,0,-1))),2)
    # the trace for the explorer: every 10 ms from 0.3 s before the backswing to 0.8 s after impact
    tr=[]; t=e['backswing_start']-0.3
    while t<=min(tend,ti+0.8):
        qc=q_at(t); qr=at(tr_cm,t)
        j=min(len(G)-1,bisect.bisect_left(TG,t)); w=math.sqrt(G[j][1]**2+G[j][2]**2+G[j][3]**2)
        tr.append([round(t-ti,3), round(ang(qr,qc),2), round(w,2), round(twist(q0,qc),1), round(twist(q0,qr),1)])
        t+=0.01
    rec['trace']=tr
    out.append(rec)
json.dump({'swings':out,'device':'iPhone 12 (iPhone13,2)','session':'P-MMFY, 2026-10-16'},open(sys.argv[2],'w'))
for r in out:
    print(f"{r['n']:2} {r['block']:13} peak {r['peak']:5.1f}  gclip {r['gyro_clip']:2} aclip {r['acc_clip']:2}  addr err {r['addr_err']:4.1f}  impact diff {r['impact_diff']:5.2f}  twist cm {r['twist_cm']:+6.1f} raw {r['twist_raw']:+6.1f}  still after {r['still_after']}  truth cm {r.get('truth_cm')} raw {r.get('truth_raw')}")
