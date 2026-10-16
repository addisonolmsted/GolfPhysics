import json,struct,glob,sys,math,bisect,statistics as st
root=glob.glob(sys.argv[1]+'/*/')[0]
def readbin(p):
    d=open(p,'rb').read(); L=struct.unpack('<I',d[4:8])[0]; h=json.loads(d[8:8+L]); body=d[8+L:]
    nc=len(h['columns']); n=len(body)//8//nc*nc
    v=struct.unpack('<%dd'%n, body[:n*8]); return [v[i:i+nc] for i in range(0,n,nc)]
M=readbin(root+'stream/motion.bin'); T=[r[0] for r in M]
ev=[json.loads(l) for l in open(root+'stream/events.jsonl')]
ends=sorted([e['t'] for e in ev if e['type'] in ('swing_saved','disarmed','capture_start','swing_deleted')])
arms=[e['t'] for e in ev if e['type']=='armed']
def down(r):
    g=math.sqrt(r[8]**2+r[9]**2+r[10]**2); return math.degrees(math.acos(max(-1,min(1,r[9]/g))))
def acc_fused(r): return math.sqrt(r[5]**2+r[6]**2+r[7]**2)
def acc_total(r): return abs(math.sqrt((r[5]+r[8])**2+(r[6]+r[9])**2+(r[7]+r[10])**2)-1)
def simulate(i0, i1, accf, window=1.0):
    """the detector's idle -> settling -> armed, from sample i0; the arm time or None"""
    phase='idle'; since=None; ew=ea=None; lt=None
    for i in range(i0,i1):
        r=M[i]; w=math.sqrt(r[11]**2+r[12]**2+r[13]**2); a=accf(r)
        if ew is None: ew,ea,lt=w,a,r[0]
        else:
            k=1-math.exp(-max(0,min(0.1,r[0]-lt))/0.15); ew+=k*(w-ew); ea+=k*(a-ea); lt=r[0]
        if phase=='idle':
            if down(r)<45 and w<0.35 and a<0.06: phase='settling'; since=r[0]; ew,ea=w,a
        else:
            if not(down(r)<55 and ew<0.7 and ea<0.12): phase='idle'; continue
            if r[0]-since>=window: return r[0]
    return None
old=[];new=[]
print('arm (actual) | replayed old rule | new rule (total acceleration vs 1 g) | seconds saved')
for ta in arms:
    pe=max([x for x in ends if x<ta] or [T[0]])
    i0=bisect.bisect_left(T,pe); i1=bisect.bisect_left(T,ta+0.5)
    a_old=simulate(i0,i1,acc_fused); a_new=simulate(i0,i1,acc_total)
    if a_old is None or a_new is None: continue
    old.append(a_old-pe); new.append(a_new-pe)
    if a_old-a_new>0.3: print(f"  {ta-ev[0]['t']:7.2f}  old {a_old-pe:5.2f} s after the last swing ended   new {a_new-pe:5.2f} s   saved {a_old-a_new:4.2f} s")
print(f"\nreplayed over {len(old)} arms: time from the end of the previous swing (or a disarm) to arming")
print(f"  old rule: median {st.median(old):.2f} s, 90th percentile {sorted(old)[int(len(old)*0.9)]:.2f} s, max {max(old):.2f} s")
print(f"  new rule: median {st.median(new):.2f} s, 90th percentile {sorted(new)[int(len(new)*0.9)]:.2f} s, max {max(new):.2f} s")
# false arming check: does the new rule arm while the phone is actually moving? count arms in swing windows (backswing..impact)
sw=[(e['backswing_start'],e['t']) for e in ev if e['type']=='impact']

# the felt delay: how long the phone had been down and truly still (rotation < 0.35 rad/s, total acceleration within
# 0.06 g of 1 g, continuously) when the old rule finally armed
felt=[]
for ta in arms:
    k=bisect.bisect_left(T,ta)
    j=k
    while j>0 and down(M[j-1])<45 and math.sqrt(M[j-1][11]**2+M[j-1][12]**2+M[j-1][13]**2)<0.35 and acc_total(M[j-1])<0.06: j-=1
    felt.append(ta-T[j])
fs=sorted(felt)
print(f"\nactual arms: how long the phone had already been still at address when it armed (the new rule: always 1.0 s)")
print(f"  median {st.median(fs):.2f} s, 75th {fs[int(len(fs)*0.75)]:.2f} s, 90th {fs[int(len(fs)*0.9)]:.2f} s, max {fs[-1]:.2f} s; over 1.5 s: {sum(1 for x in fs if x>1.5)} of {len(fs)}")
