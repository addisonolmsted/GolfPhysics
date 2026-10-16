import sys,io,contextlib,numpy as np
from itertools import combinations
S='/private/tmp/claude-501/-Users-davisolmsted/8a9bbb64-64ac-47ea-82ae-1f684be93b9d/scratchpad'
sys.argv=['x',S+'/speed.py',S+'/pilot']
buf=io.StringIO()
with contextlib.redirect_stdout(buf): exec(open(S+'/speed2.py').read())
L={'chip10':0.9,'chip25':0.9,'chip40':0.9,'putt1':0.85,'putt4':0.85,'putt10':0.85,'practice':0.95,'full':0.95,'hard':0.95,'three_quarter':0.95,'fade':0.95,'draw':0.95}
def hands(e, start):
    """phone velocity at impact by integrating the linear acceleration from `start` (velocity taken as zero there)"""
    b=block_at(e['t']); ta=max(x for x in arms if x<=e['backswing_start']); ti=e['t']
    bias=bias_at(ta); T,Q=orientation(ta,ti+0.05,bias)
    t0=ta if start=='address' else e['top']
    m=(A[:,0]>=t0)&(A[:,0]<=ti); t=A[m,0]; raw=A[m,1:4]
    clipped=int((np.abs(raw).max(1)>=7.8).sum())
    qi=np.stack([np.interp(t,T,Q[:,k]) for k in range(4)],1); qi/=np.linalg.norm(qi,axis=1,keepdims=True)
    g=np.array([qrot(qconj(q),np.array([0,0,-1.0])) for q in qi])
    ia=(A[:,0]>ta-0.3)&(A[:,0]<ta); qa=np.stack([np.interp(A[ia,0],T,Q[:,k]) for k in range(4)],1) if False else None
    a0=np.zeros(3)
    a_w=np.array([qrot(q,v) for q,v in zip(qi,-(raw-g)*G0)])
    v=np.zeros(3)
    for k in range(1,len(t)): v=v+0.5*(a_w[k]+a_w[k-1])*(t[k]-t[k-1])
    w=interp_rows(G[:,0],G[:,1:4],np.array([ti]))[0]-bias
    club=qrot(qi[-1],np.cross(w,np.array([0,L[b],0])))
    return np.linalg.norm(v), np.linalg.norm(v+club), np.linalg.norm(club), clipped, ti-t0
groups={'putts':['putt1','putt4','putt10'],'10-yd chips':['chip10'],'25-40 yd pitches':['chip25','chip40'],'full-type':['practice','full','hard','three_quarter','fade','draw']}
for name,bl in groups.items():
    es=[e for e in imps if block_at(e['t']) in bl]
    A_=np.array([hands(e,'address') for e in es]); T_=np.array([hands(e,'top') for e in es])
    for lab,X in (('from address',A_),('from the top',T_)):
        rot,head,hand=X[:,2],X[:,1],X[:,0]
        k=(rot@head)/(rot@rot); err=(k*rot-head)/head*100
        pairs=list(combinations(range(len(X)),2)); agree=sum(1 for i,j in pairs if (rot[i]-rot[j])*(head[i]-head[j])>0)
        print(f"{name:17} {lab:13} n={len(X):2}  hands {np.median(hand):4.2f} m/s, head {np.median(head):5.2f}, hand share {np.median(1-rot/head)*100:4.0f}%  | rotation-only x{k:.2f}: rms error {np.sqrt((err**2).mean()):4.1f}% (worst {np.abs(err).max():4.1f}%), same ranking {agree}/{len(pairs)}  | clipped samples {int(np.median(X[:,3]))}, integration {np.median(X[:,4]):.2f} s")
# the putts' speed spread per target distance: does the head estimate separate the 1.5, 4 and 10 m putts?
print()
for b in ['putt1','putt4','putt10','chip10','chip25','chip40']:
    es=[e for e in imps if block_at(e['t'])==b]; X=np.array([hands(e,'top') for e in es])
    print(f"  {b:7} head speed from the top: {' '.join(f'{x:.2f}' for x in X[:,1])} m/s   rotation only: {' '.join(f'{x:.2f}' for x in X[:,2])} m/s")
