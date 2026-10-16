import json,struct,glob,sys,math,bisect
import numpy as np
root=glob.glob(sys.argv[1]+'/*/')[0]
G0=9.80665
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    return np.frombuffer(body[:n*8],dtype='<f8').reshape(-1,nc)
M=readbin(root+'stream/motion.bin'); G=readbin(root+'stream/raw_gyro.bin'); A=readbin(root+'stream/raw_accel.bin')
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
blocks=[e for e in ev if e['type']=='block_start']
def block_at(t): return [b['block'] for b in blocks if b['t']<=t][-1]
imps=[e for e in ev if e['type']=='impact']; arms=sorted(e['t'] for e in ev if e['type'] in('armed','readdressed'))
# quaternion helpers (w,x,y,z), vectorised where useful
def qmul(a,b):
    w1,x1,y1,z1=a; w2,x2,y2,z2=b
    return np.array([w1*w2-x1*x2-y1*y2-z1*z2, w1*x2+x1*w2+y1*z2-z1*y2, w1*y2-x1*z2+y1*w2+z1*x2, w1*z2+x1*y2-y1*x2+z1*w2])
def qconj(q): return q*np.array([1,-1,-1,-1])
def qrot(q,v): return qmul(qmul(q,np.r_[0,v]),qconj(q))[1:]
def arc(a,b):
    a=a/np.linalg.norm(a); b=b/np.linalg.norm(b); c=np.cross(a,b); q=np.r_[1+a@b,c]; return q/np.linalg.norm(q)
def interp_rows(T,X,t): return np.stack([np.interp(t,T,X[:,k]) for k in range(X.shape[1])],1)
def bias_at(t):
    m=(G[:,0]>t-2)&(G[:,0]<t); g=G[m]; f=interp_rows(M[:,0],M[:,11:14],g[:,0]); quiet=np.linalg.norm(g[:,1:4],axis=1)<0.5
    return np.median(g[quiet,1:4]-f[quiet],axis=0)
def orientation(ta, t_end, bias):
    """raw-gyro orientation from address (tilt anchored to the raw accelerometer), at the gyro's timestamps"""
    i0=np.searchsorted(M[:,0],ta-0.3); i1=np.searchsorted(M[:,0],ta)
    qs=M[i0:i1,1:5].copy(); qs[(qs@qs[0])<0]*=-1; q0=qs.mean(0); q0/=np.linalg.norm(q0)
    gm=A[(A[:,0]>ta-0.3)&(A[:,0]<ta),1:4].mean(0); gp=qrot(qconj(q0),np.array([0,0,-1.0]))
    q=qmul(q0,arc(gm,gp)); m=(G[:,0]>=ta)&(G[:,0]<=t_end); T=G[m,0]; W=G[m,1:4]-bias
    Q=[q]; 
    for k in range(1,len(T)):
        w=W[k]; th=np.linalg.norm(w)*(T[k]-T[k-1])
        if th>0: q=qmul(q,np.r_[math.cos(th/2),w/np.linalg.norm(w)*math.sin(th/2)]); q/=np.linalg.norm(q)
        Q.append(q)
    return T,np.array(Q)
def fit_window(e):
    ta=max(x for x in arms if x<=e['backswing_start']); ti=e['t']
    bias=bias_at(ta); T,Q=orientation(ta,ti+0.35,bias)
    m=(A[:,0]>=max(e['top']-0.25, e['backswing_start'])) & (A[:,0]<=ti+0.25)
    t=A[m,0]; raw=A[m,1:4]
    w=interp_rows(G[:,0],G[:,1:4],t)-bias
    dw=np.gradient(w,t,axis=0)
    # gravity in the device frame from the raw-gyro orientation
    qi=np.stack([np.interp(t,T,Q[:,k]) for k in range(4)],1); qi/=np.linalg.norm(qi,axis=1,keepdims=True)
    gdev=np.array([qrot(qconj(q),np.array([0,0,-1.0])) for q in qi])
    return dict(t=t,raw=raw,w=w,dw=dw,g=gdev,ti=ti,bias=bias)
def design(w,dw):
    """rows: the model's rotational acceleration (in g) is linear in r: alpha x r + w x (w x r)"""
    n=len(w); X=np.zeros((3*n,6))
    for k in range(n):
        ax,ay,az=dw[k]; wx,wy,wz=w[k]
        Acr=np.array([[0,-az,ay],[az,0,-ax],[-ay,ax,0]])       # alpha x
        W=np.array([[0,-wz,wy],[wz,0,-wx],[-wy,wx,0]])        # w x
        X[3*k:3*k+3,0:3]=(Acr+W@W)/G0
        X[3*k:3*k+3,3:6]=np.eye(3)                            # a slowly varying rest: hand (pivot) acceleration, bias
    return X
def fit(d, ok, sign):
    X=design(d['w'],d['dw']); y=(d['raw']-d['g']).reshape(-1)*sign
    keep=np.repeat(ok,3); sol,*_=np.linalg.lstsq(X[keep],y[keep],rcond=None)
    pred=(X@sol).reshape(-1,3)*sign+d['g']; return sol,pred
full_like=lambda b: b in('practice','full','hard','three_quarter','fade','draw')
# 1. the repair test: clip artificially at 6 g, rebuild the 6-7.8 g samples, compare with what was measured
print('1. REPAIR TEST: clip at 6 g, rebuild the samples the phone measured between 6 and 7.8 g')
errs_m=[];errs_lin=[];signs=[];R=[]
for e in imps:
    if not full_like(block_at(e['t'])): continue
    d=fit_window(e); mx=np.abs(d['raw']).max(1)
    truly=mx>=7.8; held=(mx>=6)&(~truly); ok=mx<6
    if held.sum()<3: continue
    best=None
    for sign in (1,-1):
        sol,pred=fit(d,ok,sign); res=np.abs(pred[ok]-d['raw'][ok]).mean()
        if best is None or res<best[0]: best=(res,sign,sol,pred)
    _,sign,sol,pred=best; signs.append(sign); R.append(np.linalg.norm(sol[:3]))
    e_m=np.abs(pred[held]-d['raw'][held]).max(1)
    # baseline: straight-line interpolation over the missing stretch, per axis
    lin=d['raw'].copy()
    for k in range(3): lin[~ok,k]=np.interp(d['t'][~ok],d['t'][ok],d['raw'][ok,k])
    e_l=np.abs(lin[held]-d['raw'][held]).max(1)
    errs_m+=list(e_m); errs_lin+=list(e_l)
print(f"  swings {len(signs)}, sign of the model {set(signs)}, fitted radius |r| median {np.median(R):.2f} m (range {min(R):.2f}-{max(R):.2f})")
print(f"  rebuilt samples {len(errs_m)}: model error median {np.median(errs_m):.2f} g, 90th {np.percentile(errs_m,90):.2f} g | straight-line interpolation median {np.median(errs_lin):.2f} g, 90th {np.percentile(errs_lin,90):.2f} g")
# 2. speed at impact: the virtual clubhead (phone + a club along the phone's +y) from the model's radius
L={'practice':0.95,'full':0.95,'hard':0.95,'three_quarter':0.95,'fade':0.95,'draw':0.95,'chip10':0.9,'chip25':0.9,'chip40':0.9,'putt1':0.85,'putt4':0.85,'putt10':0.85}
print('\n2. SPEED AT IMPACT: hands (w x r) and the virtual clubhead (w x (r + L y)); rotation only = w x L y')
rows={}
for e in imps:
    b=block_at(e['t']); d=fit_window(e); mx=np.abs(d['raw']).max(1); ok=mx<7.8
    best=None
    for sign in (1,-1):
        sol,pred=fit(d,ok,sign); res=np.abs(pred[ok]-d['raw'][ok]).mean()
        if best is None or res<best[0]: best=(res,sign,sol)
    res,sign,sol=best; r=sol[:3]
    w=interp_rows(G[:,0],G[:,1:4],np.array([e['t']]))[0]-d['bias']
    hand=np.cross(w,r); head=np.cross(w,r+np.array([0,L[b],0])); rot=np.cross(w,np.array([0,L[b],0]))
    rows.setdefault(b,[]).append((np.linalg.norm(hand),np.linalg.norm(head),np.linalg.norm(rot),np.linalg.norm(r),res,int((~ok).sum())))
for b,v in rows.items():
    v=np.array(v)
    lost=1-v[:,2]/v[:,1]
    print(f"  {b:13} n={len(v):2}  head {np.median(v[:,1]):5.1f} m/s ({np.median(v[:,1])/0.44704:5.1f} mph)  hands {np.median(v[:,0]):4.1f} m/s  rotation only {np.median(v[:,2]):5.1f} m/s  lost {np.median(lost)*100:4.1f}% (range {lost.min()*100:4.1f}-{lost.max()*100:4.1f})  |r| {np.median(v[:,3]):.2f} m  fit residual {np.median(v[:,4]):.3f} g  clipped samples {int(np.median(v[:,5]))}")
# 3. gyro clipping: clip the raw gyro at 28 rad/s on full swings, bridge, compare
print('\n3. GYRO BRIDGE TEST: clip the raw gyro at 28 rad/s per axis, bridge the gap, compare')
pk_err=[];int_err=[];gaps=[]
for e in imps:
    if not full_like(block_at(e['t'])): continue
    m=(G[:,0]>=e['backswing_start'])&(G[:,0]<=e['t']+0.3); t=G[m,0]; w=G[m,1:4].copy()
    for k in range(3):
        bad=np.abs(w[:,k])>=28
        if not bad.any(): continue
        gaps.append(int(bad.sum()))
        good=~bad; idx=np.where(bad)[0]
        # fit a cubic to 5 unclipped samples each side of each gap
        lo=max(0,idx[0]-5); hi=min(len(t),idx[-1]+6); sel=[i for i in range(lo,hi) if good[i]]
        c=np.polyfit(t[sel]-t[idx[0]],w[sel,k],3); est=np.polyval(c,t[idx]-t[idx[0]])
        # never below the clip level
        est=np.sign(w[idx,k])*np.maximum(np.abs(est),28)
        pk_err.append(abs(est).max()-abs(w[idx,k]).max())
        int_err.append(np.trapezoid(est-w[idx,k],t[idx]))
if gaps:
    print(f"  clipped stretches {len(gaps)}, length median {np.median(gaps):.0f} samples (max {max(gaps)})")
    print(f"  peak error median {np.median(pk_err):+.2f} rad/s (90th of |err| {np.percentile(np.abs(pk_err),90):.2f}); angle error over the gap median {np.degrees(np.median(np.abs(int_err))):.2f} deg (max {np.degrees(max(np.abs(int_err))):.2f})")
    clipped_only=[np.trapezoid(np.abs(G[(G[:,0]>=e['backswing_start'])&(G[:,0]<=e['t']+0.3),1:4]).clip(min=28)-28,G[(G[:,0]>=e['backswing_start'])&(G[:,0]<=e['t']+0.3),0],axis=0).max() for e in imps if full_like(block_at(e['t']))]
    print(f"  doing nothing (keeping the clipped values) loses up to {np.degrees(max(clipped_only)):.2f} deg of rotation per swing")
