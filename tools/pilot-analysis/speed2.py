import sys
_src=open(sys.argv[1]).read(); OUT=sys.argv[3] if len(sys.argv)>3 else None; sys.argv=[sys.argv[0]]+sys.argv[2:3]; exec(_src.split("full_like=lambda")[0])
full_like=lambda b: b in('practice','full','hard','three_quarter','fade','draw')
def local_fit_rebuild(d, ok, miss_mask, win=0.12):
    """rebuild the missing samples: the rigid-body shape (CoreMotion's sign: raw - gravity = -(a_point)), with a
    hand-acceleration term linear in time, fitted on good samples within `win` of the gap; then the leftover
    (measured - model) is interpolated across the gap and added back"""
    t=d['t']; out=d['raw'].copy()
    idx=np.where(miss_mask)[0]
    if len(idx)==0: return out
    # split into stretches
    groups=np.split(idx, np.where(np.diff(idx)>1)[0]+1)
    for g in groups:
        t0,t1=t[g[0]],t[g[-1]]
        near=(t>=t0-win)&(t<=t1+win)
        fitm=near&ok
        if fitm.sum()<6: 
            for k in range(3): out[g,k]=np.interp(t[g],t[ok],d['raw'][ok,k])
            continue
        X=design(d['w'][near],d['dw'][near])
        tt=np.repeat(t[near]-t0,3)
        X=np.hstack([X, X[:,3:6]*tt[:,None]])
        y=-(d['raw'][near]-d['g'][near]).reshape(-1)
        keep=np.repeat(fitm[near],3)
        sol,*_=np.linalg.lstsq(X[keep],y[keep],rcond=None)
        pred=-(X@sol).reshape(-1,3)+d['g'][near]
        resid=d['raw'][near]-pred
        tn=t[near]; okn=fitm[near]; gn=np.isin(np.where(near)[0],g)
        for k in range(3):
            r_i=np.interp(tn[gn],tn[okn],resid[okn,k])
            out[np.where(near)[0][gn],k]=pred[gn,k]+r_i
    return out
print('1. REPAIR TEST (again): clip at 6 g, rebuild the samples measured between 6 and 7.8 g')
em=[];el=[];ep=[]
for e in imps:
    if not full_like(block_at(e['t'])): continue
    d=fit_window(e); mx=np.abs(d['raw']).max(1)
    held=(mx>=6)&(mx<7.8); ok=mx<6
    if held.sum()<3: continue
    reb=local_fit_rebuild(d,ok,~ok)
    lin=d['raw'].copy()
    for k in range(3): lin[~ok,k]=np.interp(d['t'][~ok],d['t'][ok],d['raw'][ok,k])
    em+=list(np.abs(reb[held]-d['raw'][held]).max(1)); el+=list(np.abs(lin[held]-d['raw'][held]).max(1))
    # the peak: the largest component in the held-out stretch
    ep.append((np.abs(reb[held]).max()-np.abs(d['raw'][held]).max(), np.abs(lin[held]).max()-np.abs(d['raw'][held]).max()))
ep=np.array(ep)
print(f"  samples {len(em)}: rigid-body shape + interpolated leftover: median {np.median(em):.2f} g, 90th {np.percentile(em,90):.2f} g | straight line: median {np.median(el):.2f} g, 90th {np.percentile(el,90):.2f} g")
print(f"  peak height error: model median {np.median(ep[:,0]):+.2f} g | straight line median {np.median(ep[:,1]):+.2f} g (the straight line cuts the peak off)")

print('\n2. HAND SPEED, by integrating the acceleration from the still address to impact (raw-gyro orientation; clipped samples rebuilt)')
L={'practice':0.95,'full':0.95,'hard':0.95,'three_quarter':0.95,'fade':0.95,'draw':0.95,'chip10':0.9,'chip25':0.9,'chip40':0.9,'putt1':0.85,'putt4':0.85,'putt10':0.85}
res={}
for e in imps:
    b=block_at(e['t']); ta=max(x for x in arms if x<=e['backswing_start']); ti=e['t']
    bias=bias_at(ta); T,Q=orientation(ta,ti+0.05,bias)
    m=(A[:,0]>=ta)&(A[:,0]<=ti); t=A[m,0]; raw=A[m,1:4].copy()
    # rebuild clipped samples with the local model
    d=dict(t=t,raw=raw); d['w']=interp_rows(G[:,0],G[:,1:4],t)-bias; d['dw']=np.gradient(d['w'],t,axis=0)
    qi=np.stack([np.interp(t,T,Q[:,k]) for k in range(4)],1); qi/=np.linalg.norm(qi,axis=1,keepdims=True)
    d['g']=np.array([qrot(qconj(q),np.array([0,0,-1.0])) for q in qi])
    ok=np.abs(raw).max(1)<7.8
    raw=local_fit_rebuild(d,ok,~ok) if (~ok).any() else raw
    # accelerometer offset from the still address: raw - gravity there should be zero
    a0=(raw[:20]-d['g'][:20]).mean(0)
    # world linear acceleration (m/s^2): a = -(raw - g - offset), rotated to the world
    a_dev=-(raw-d['g']-a0)*G0
    a_w=np.array([qrot(q,v) for q,v in zip(qi,a_dev)])
    v=np.zeros(3); V=[v]
    for k in range(1,len(t)): v=v+0.5*(a_w[k]+a_w[k-1])*(t[k]-t[k-1]); V.append(v)
    vh=V[-1]
    # the club: along the phone's +y, rotating with the phone
    w=interp_rows(G[:,0],G[:,1:4],np.array([ti]))[0]-bias
    qimp=qi[-1]
    club_dev=np.cross(w,np.array([0,L[b],0])); club_w=qrot(qimp,club_dev)
    head=vh+club_w
    res.setdefault(b,[]).append((np.linalg.norm(vh),np.linalg.norm(head),np.linalg.norm(club_w), ti-ta, int((~ok).sum())))
print('  block          n   hands     club rotation   head      lost without the hands   (integration time, clipped)')
for b,v in res.items():
    v=np.array(v); lost=1-v[:,2]/v[:,1]
    print(f"  {b:13} {len(v):2}  {np.median(v[:,0]):5.1f} m/s   {np.median(v[:,2]):5.1f} m/s     {np.median(v[:,1]):5.1f} m/s ({np.median(v[:,1])/0.44704:5.1f} mph)   {np.median(lost)*100:5.1f}% (range {lost.min()*100:5.1f} to {lost.max()*100:5.1f})   ({np.median(v[:,3]):.1f} s, {int(np.median(v[:,4]))})")

print('\n3. GYRO BRIDGE (again): clip at 28 rad/s per axis; rebuild from the other two axes and the steady rotation axis')
pe_c=[];pe_a=[];ae_c=[];ae_a=[]
for e in imps:
    if not full_like(block_at(e['t'])): continue
    m=(G[:,0]>=e['backswing_start'])&(G[:,0]<=e['t']+0.3); t=G[m,0]; w=G[m,1:4]
    bad=np.abs(w)>=28
    if not bad.any(): continue
    rows_bad=bad.any(1); idx=np.where(rows_bad)[0]
    for g in np.split(idx,np.where(np.diff(idx)>1)[0]+1):
        lo=max(0,g[0]-6); hi=min(len(t),g[-1]+7)
        nb=[i for i in range(lo,hi) if not rows_bad[i]]
        n_hat=np.mean([w[i]/np.linalg.norm(w[i]) for i in nb],0); n_hat/=np.linalg.norm(n_hat)
        for i in g:
            k_bad=np.where(bad[i])[0]; k_ok=[k for k in range(3) if k not in k_bad]
            # |w| from the unclipped components and the axis: w_ok = |w| n_ok
            mag=np.linalg.norm(w[i,k_ok])/max(1e-6,np.linalg.norm(n_hat[k_ok]))
            est=w[i].copy()
            for k in k_bad: est[k]=np.sign(w[i,k])*max(28,abs(mag*n_hat[k]))
            pe_a.append(np.linalg.norm(est)-np.linalg.norm(w[i]))
        # cubic, per axis, for comparison
        for k in range(3):
            if not bad[g,k].any(): continue
            sel=[i for i in range(lo,hi) if not bad[i,k]]
            c=np.polyfit(t[sel]-t[g[0]],w[sel,k],3); est=np.sign(w[g,k])*np.maximum(np.abs(np.polyval(c,t[g]-t[g[0]])),28)
            pe_c.append(np.abs(est).max()-np.abs(w[g,k]).max())
pe_a=np.array(pe_a); pe_c=np.array(pe_c)
print(f"  from the rotation axis: |w| error per sample median {np.median(pe_a):+.2f} rad/s, 90th of |err| {np.percentile(np.abs(pe_a),90):.2f} rad/s ({np.percentile(np.abs(pe_a),90)/30*100:.0f}% of a 30 rad/s peak)")
print(f"  cubic per axis:         peak error median {np.median(pe_c):+.2f} rad/s, 90th of |err| {np.percentile(np.abs(pe_c),90):.2f} rad/s")

if OUT:
    import json
    out={}
    # the repair: per-sample errors and one worked example
    em=[];el=[];peaks=[];example=None
    for e in imps:
        if not full_like(block_at(e['t'])): continue
        d=fit_window(e); mx=np.abs(d['raw']).max(1); held=(mx>=6)&(mx<7.8); ok=mx<6
        if held.sum()<3: continue
        reb=local_fit_rebuild(d,ok,~ok); lin=d['raw'].copy()
        for k in range(3): lin[~ok,k]=np.interp(d['t'][~ok],d['t'][ok],d['raw'][ok,k])
        em+=list(np.abs(reb[held]-d['raw'][held]).max(1)); el+=list(np.abs(lin[held]-d['raw'][held]).max(1))
        peaks.append([round(float(np.abs(d['raw'][held]).max()),2), round(float(np.abs(reb[held]).max()),2), round(float(np.abs(lin[held]).max()),2)])
        if example is None and held.sum()>=5:
            k=int(np.argmax(np.abs(d['raw']).max(0))); win=(d['t']>=d['ti']-0.35)&(d['t']<=d['ti']+0.2)
            example={'block':block_at(e['t']),'axis':'xyz'[k],'t':[round(float(x-d['ti']),3) for x in d['t'][win]],
                     'measured':[round(float(x),3) for x in d['raw'][win,k]],'model':[round(float(x),3) for x in reb[win,k]],
                     'line':[round(float(x),3) for x in lin[win,k]],'missing':[bool(x) for x in (~ok)[win]],'true_clip':[bool(x) for x in (mx>=7.8)[win]]}
    out['repair']={'model_med':round(float(np.median(em)),2),'model_p90':round(float(np.percentile(em,90)),2),'line_med':round(float(np.median(el)),2),'line_p90':round(float(np.percentile(el,90)),2),
                   'n':len(em),'swings':len(peaks),'peak_err_model':round(float(np.median([p[1]-p[0] for p in peaks])),2),'peak_err_line':round(float(np.median([p[2]-p[0] for p in peaks])),2),'example':example}
    # the hands
    rows=[]
    for b,v in res.items():
        v=np.array(v); lost=1-v[:,2]/v[:,1]
        rows.append({'block':b,'n':len(v),'hands':round(float(np.median(v[:,0])),1),'rotation':round(float(np.median(v[:,2])),1),'head':round(float(np.median(v[:,1])),1),
                     'lost':round(float(np.median(lost))*100,1),'lost_min':round(float(lost.min())*100,1),'lost_max':round(float(lost.max())*100,1),'clipped':int(np.median(v[:,4]))})
    out['hands']=rows
    out['bridge']={'axis_med':round(float(np.median(pe_a)),2),'axis_p90':round(float(np.percentile(np.abs(pe_a),90)),2),'cubic_med':round(float(np.median(pe_c)),2),'cubic_p90':round(float(np.percentile(np.abs(pe_c),90)),2)}
    json.dump(out,open(OUT,'w'))
    print(json.dumps({k:v for k,v in out.items() if k!='repair'})[:400]); print(out['repair']['example']['block'], len(out['repair']['example']['t']))
