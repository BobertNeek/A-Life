"""Independent fixed finite-support algebra oracle. No native imports or state."""
import math,itertools,json
from pathlib import Path

def feature(group,fear,pain,l,p):
    return (1/(1+math.exp(-group)))*l*p*(1-max(max(0,min(1,(fear-.25)/.75)),max(0,min(1,(pain-.1)/.9))))
def geo(l,m): return (-.2*m, .3*l*m, .1*l*(1-m))
def dist(group,fear,pain,t,w,star):
    rows=[]
    for r,rp in enumerate((.2,.3,.5)):
        ls=[(1,1.)] if r==1 else [(0,.4),(1,.6)]
        ps=[(1,1.)] if r==2 else [(0,.7),(1,.3)]
        if star:
            pairs=[]
            for l,lp in ls:
                qs=[(p,pp*math.exp(w[3]*feature(group,fear,pain,l,p)/t)) for p,pp in ps]
                zp=sum(q for _,q in qs)
                for m,mp in [(0,.8),(1,.2)]:
                    phi=geo(l,m)
                    pairs.append((l,m,lp*mp*math.exp(sum(a*b for a,b in zip(w,phi))/t)*zp,qs,zp))
            zpair=sum(row[2] for row in pairs)
            for l,m,pairmass,qs,zp in pairs:
                for p,pm in qs:
                    rows.append(((r,l,m,p),rp*zpair*(pairmass/zpair)*(pm/zp),(*geo(l,m),feature(group,fear,pain,l,p))))
        else:
            for (l,lp),(m,mp),(p,pp) in itertools.product(ls,[(0,.8),(1,.2)],ps):
                phi=(*geo(l,m),feature(group,fear,pain,l,p))
                rows.append(((r,l,m,p),rp*lp*mp*pp*math.exp(sum(a*b for a,b in zip(w,phi))/t),phi))
    z=sum(p for _,p,_ in rows)
    return {key:(p/z,phi) for key,p,phi in rows}

checks=0;worst=0
for group,fear,pain,t in itertools.product((-2.,0.,2.),(0.,.8,1.),(0.,.8,1.),(.73,1.)):
    w=[-4.,1.,-1.,-2.]
    a=dist(group,fear,pain,t,w,True);b=dist(group,fear,pain,t,w,False)
    assert abs(sum(x[0] for x in a.values())-1)<1e-12
    assert all(p>0 for p,_ in a.values())
    assert max(abs(a[k][0]-b[k][0]) for k in a)<1e-12
    for j in range(4):
        mean=sum(p*phi[j] for p,phi in a.values());h=1e-5
        wp=w[:];wm=w[:];wp[j]+=h;wm[j]-=h
        plus=dist(group,fear,pain,t,wp,True);minus=dist(group,fear,pain,t,wm,True)
        physical={}
        for k,(p,phi) in a.items():
            derivative=(phi[j]-mean)/t
            err=abs((math.log(plus[k][0])-math.log(minus[k][0]))/(2*h)-derivative)
            worst=max(worst,err);assert err<1e-8
            # Marginalize representative and manipulation; both operations
            # contribute separate physical posture outcomes with fixed .25/.75.
            for op,q in ((0,.25),(1,.75)) if k[3] else ((None,1.),):
                v=physical.setdefault((k[1],k[3],op),[0.,0.,0.,0.])
                for i,x in enumerate((p,p*derivative,plus[k][0],minus[k][0])):v[i]+=q*x
        for v in physical.values():
            err=abs((math.log(v[2])-math.log(v[3]))/(2*h)-v[1]/v[0]);worst=max(worst,err);assert err<1e-8
    z=dist(group,fear,pain,t,[-4.,1.,-1.,0.],True)
    old=dist(group,fear,pain,t,[-4.,1.,-1.,0.],False)
    assert max(abs(z[k][0]-old[k][0]) for k in z)<1e-12
    if fear==1. or pain==1.: assert max(abs(a[k][0]-z[k][0]) for k in a)<1e-12
    checks+=1
for g in (-2.,0.,2.):
    assert feature(g,0,0,0,1)==0 and feature(g,0,0,1,0)==0
assert feature(2,0,0,1,1)>feature(-2,0,0,1,1)
result={'fixed_contexts':checks,'normalization_cartesian_support_zero_weight_escape':'PASS','path_and_marginal_derivative_axes':4,'worst_derivative_error':worst,'native_execution':False,'world_intervals':0,'learning_updates':0}
Path(__file__).with_name('analytic-results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
