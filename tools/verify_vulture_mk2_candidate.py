"""Independent SH038 derived-candidate contacts and exact original station witnesses."""

import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile

from prepare_vulture_mk2_source import ROOT, require
from prepare_vulture_mk2_candidate import CANDIDATE, definition


STATION_SHA='cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5'


def bounds(points):
    return tuple(tuple(f(axis) for axis in zip(*points,strict=True)) for f in (min,max))


def gap(a,b):
    lo,hi=a; low,high=b
    return max(max(c-d,e-f) for c,d,e,f in zip(low,hi,lo,high,strict=True))


def station_proof(groups,native,scale):
    blob=(ROOT/'assets/legacy/sh028.physics').read_bytes()
    require(hashlib.sha256(blob).hexdigest()==STATION_SHA,'Changed original SH028')
    magic,count,faces=struct.unpack_from('<8sII',blob); require((magic,count,faces)==(b'MPHYS001',272,488),'Wrong station topology')
    split=16+24*count
    points=[tuple(map(Fraction,p)) for p in struct.iter_unpack('<3d',blob[16:split])]
    triangles=[[points[i] for i in f] for f in struct.iter_unpack('<3I',blob[split:])]
    station=list(map(bounds,triangles)); piece_boxes=list(map(bounds,groups))
    def clearance(x,z):
        start,end=sorted(map(Fraction,z)); x=Fraction(x)
        swept=[((x-hi[0],lo[1],start-hi[2]),(x-lo[0],hi[1],end-lo[2])) for lo,hi in piece_boxes]
        return int(min(gap(a,b) for a in swept for b in station)*1000//1)-21
    index=native['vertices'].index([0,-.112,1.45]); witness=native['vertices'][index]
    vertex=tuple(Fraction(x*scale) for x in witness)
    triangle=triangles[482]; z=triangle[0][2]
    require(all(p[2]==z for p in triangle),'Changed lip plane')
    actor_x=69.5; crossing_z=float(z+vertex[2]); actor_z=[crossing_z+1,crossing_z-1]
    aligned=clearance(0,[137.32,135.32]); opening=clearance(actor_x,[actor_z[0]]*2)
    require(aligned>30000 and opening>970,'Missing swept station separation')
    x,y=Fraction(actor_x)-vertex[0],vertex[1]
    (ax,ay,_),(bx,by,_),(cx,cy,_)=triangle
    denominator=(bx-ax)*(cy-ay)-(by-ay)*(cx-ax)
    u=((x-ax)*(cy-ay)-(y-ay)*(cx-ax))/denominator
    v=((bx-ax)*(y-ay)-(by-ay)*(x-ax))/denominator
    bary=(1-u-v,u,v); t=(z+vertex[2]-Fraction(actor_z[0]))/(Fraction(actor_z[1])-Fraction(actor_z[0]))
    require(0<t<1 and min(bary)>0 and all(sum(w*p[i] for w,p in zip(bary,triangle,strict=True))==q for i,q in enumerate((x,y,z))),
            'No exact original triangle contact')
    return {'stationSHA256':STATION_SHA,'aligned':{'actorX':0,'actorZ':[137.32,135.32],'minimumSeparationMillimetres':aligned},
            'offsetOpening':{'actorX':actor_x,'actorZ':actor_z[0],'minimumSeparationMillimetres':opening},
            'offsetContact':{'actorX':actor_x,'actorZ':actor_z,'face':482,'nativeVertex':index,'pointMetres':list(map(float,vertex)),
                             'crossingFraction':float(t),'barycentrics':list(map(float,bary)),'stationPointMetres':list(map(float,(x,y,z)))},
            'scope':'Exact separation of all rounded support AABBs and 488 original station triangles; exact vertex lip crossing. No runtime game admission.'}


def verify(validator=None):
    package=definition(); require(package==json.loads(CANDIDATE.read_bytes()),'Changed candidate')
    scale=package['metresPerUnit']; names=package['provenance']['groupNames']; groups=[[] for _ in names]
    for mesh in (package['detail'],package['native']):
        for point,i in zip(mesh['vertices'],mesh['groups'],strict=True): groups[i].append(tuple(Fraction(x*scale) for x in point))
    boxes=list(map(bounds,groups)); open_paths=[]
    for route in package['clearances']:
        query=bounds([tuple(map(Fraction,p)) for p in (route['start'],route['end'])])
        lower=min(gap(query,box) for box in boxes)-Fraction(12,100)
        require(lower>0,f'No separated open route {route["name"]}')
        open_paths.append({**route,'lowerBoundMetres':float(lower)})
    solid=[]
    for name in ('part-000-0','part-001-0','part-078-0','part-326-0'):
        points=groups[names.index(name)]; centre=[float(sum(a)/len(points)) for a in zip(*points,strict=True)]
        solid.append({'name':name,'start':[centre[0]-1,*centre[1:]],'end':[centre[0]+1,*centre[1:]],'radiusMetres':.1})
    report={'hull':'SH038','candidateSHA256':hashlib.sha256(CANDIDATE.read_bytes()).hexdigest(),
            'openRoutes':open_paths,'solidRoutes':solid,'station':station_proof(groups,package['native'],scale)}
    if validator:
        validator=validator.resolve()
        with tempfile.TemporaryDirectory(prefix='meridian-sh038-derived-') as temp:
            folder=Path(temp); path=folder/'package.json'; output=folder/'report.json'
            cases=[('open',package,True)]+[(r['name'],{**package,'clearances':[r]},False) for r in solid]
            checked=[]
            for label,doc,expected in cases:
                output.unlink(missing_ok=True); path.write_text(json.dumps(doc,allow_nan=False,separators=(',',':')))
                result=subprocess.run([str(validator),'validate',str(path),'--report',str(output)],capture_output=True,check=False,timeout=180)
                require(output.is_file(),f'Missing real validator result {label}: {result.stderr!r}')
                data=json.loads(output.read_bytes())
                require(data['valid'] is expected and (result.returncode==0) is expected and
                        data['clearances']==[{'name':r['name'],'passed':expected} for r in doc['clearances']],f'Wrong validator outcome {label}')
                checked.append({'label':label,'exitCode':result.returncode,'clearances':data['clearances'],'diagnostics':data['diagnostics']})
        with validator.open('rb') as stream: sha=hashlib.file_digest(stream,'sha256').hexdigest()
        report['validator']={'sha256':sha,'cases':checked}
    return report


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('--validator',type=Path); args=parser.parse_args()
    print(json.dumps(verify(args.validator),indent=2))
