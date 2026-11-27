"""Exact closed-vessel contact, original SH028 witnesses and actual validator receipts."""

import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tempfile

from prepare_newt_escape_pod_studio import OUTPUT, RAW_TENSOR_OUTPUT, ROOT, definition, require


STATION_SHA='cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5'
FOLDER=OUTPUT.parent
SOLIDS=[
    {'name':'Through closed vessel from port','start':[-.8,0,0],'end':[.8,0,0],'radiusMetres':.1},
    {'name':'Through closed lid','start':[0,.7,0],'end':[0,0,0],'radiusMetres':.1},
    {'name':'Through forward end','start':[0,0,1.5],'end':[0,0,0],'radiusMetres':.1},
    {'name':'Through aft end','start':[0,0,-1.5],'end':[0,0,0],'radiusMetres':.1},
]


def bounds(points): return tuple(tuple(f(axis) for axis in zip(*points,strict=True)) for f in (min,max))


def gap(a,b):
    lo,hi=a; low,high=b
    return max(max(c-d,e-f) for c,d,e,f in zip(low,hi,lo,high,strict=True))


def station(vertices,witness):
    blob=(ROOT/'assets/legacy/sh028.physics').read_bytes()
    require(hashlib.sha256(blob).hexdigest()==STATION_SHA,'Changed original station geometry')
    magic,count,faces=struct.unpack_from('<8sII',blob); require((magic,count,faces)==(b'MPHYS001',272,488),'Wrong station topology')
    split=16+24*count
    xyz=[tuple(map(Fraction,p)) for p in struct.iter_unpack('<3d',blob[16:split])]
    triangles=[[xyz[i] for i in f] for f in struct.iter_unpack('<3I',blob[split:])]
    station_boxes=list(map(bounds,triangles)); lo,hi=bounds(vertices)
    def clearance(x,z):
        x=Fraction(x); start,end=sorted(map(Fraction,z))
        swept=((x-hi[0],lo[1],start-hi[2]),(x-lo[0],hi[1],end-lo[2]))
        return int(min(gap(swept,box) for box in station_boxes)*1000//1)-21
    triangle=triangles[482]; plane=triangle[0][2]
    require(all(p[2]==plane for p in triangle),'Wrong lip plane')
    actor_x=float(Fraction(139,2)+witness[0]); cross_z=float(plane+witness[2]); travel=[cross_z+1,cross_z-1]
    aligned=clearance(0,[137.32,135.32]); opening=clearance(actor_x,[travel[0]]*2)
    require(aligned>47000 and opening>970,'Missing station separation')
    x,y=Fraction(actor_x)-witness[0],witness[1]
    (ax,ay,_),(bx,by,_),(cx,cy,_)=triangle
    denominator=(bx-ax)*(cy-ay)-(by-ay)*(cx-ax)
    u=((x-ax)*(cy-ay)-(y-ay)*(cx-ax))/denominator; v=((bx-ax)*(y-ay)-(by-ay)*(x-ax))/denominator
    bary=(1-u-v,u,v); t=(plane+witness[2]-Fraction(travel[0]))/(Fraction(travel[1])-Fraction(travel[0]))
    require(0<t<1 and min(bary)>0 and all(sum(w*p[i] for w,p in zip(bary,triangle,strict=True))==q for i,q in enumerate((x,y,plane))), 'Missing exact native lip crossing')
    return {'stationSHA256':STATION_SHA,'aligned':{'actorX':0,'actorZ':[137.32,135.32],'lowerGapMillimetres':aligned},
            'offsetOpening':{'actorX':actor_x,'actorZ':travel[0],'lowerGapMillimetres':opening},
            'offsetContact':{'actorX':actor_x,'actorZ':travel,'nativeVertex':36,'stationFace':482,'fraction':float(t),
                             'barycentrics':list(map(float,bary)),'stationPointMetres':list(map(float,(x,y,plane)))},
            'scope':'Exact original station triangle separation with 21 mm outward guard, exact source vertex crossing (not first contact).'}


def geometry():
    package=definition(); require(package==json.loads(OUTPUT.read_bytes()),'Changed SH040 package')
    scale=package['metresPerUnit']; vertices=[tuple(Fraction(x*scale) for x in p) for mesh in (package['detail'],package['native']) for p in mesh['vertices']]
    box=bounds(vertices); open_paths=[]
    for route in package['clearances']:
        line=bounds([tuple(map(Fraction,p)) for p in (route['start'],route['end'])])
        lower=gap(line,box)-Fraction(12,100); require(lower>0,'Exterior route not separated')
        open_paths.append({**route,'lowerBoundMetres':float(lower)})
    witness=package['native']['vertices'][36]; require(witness==[.144,0,.38],'Changed native lip witness')
    return {'hull':'SH040','packageSHA256':hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),'openRoutes':open_paths,'solidRoutes':SOLIDS,
            'station':station(vertices,tuple(Fraction(x*scale) for x in witness)),'scope':'Closed capsule; external bypasses only; no through-opening or actual gameplay admission.'}


def check_group(report,package):
    expected=[tuple(v*package['metresPerUnit'] for v in p) for mesh in (package['detail'],package['native']) for p in mesh['vertices']]
    groups=report['supportGroups']; require(len(groups)==1 and groups[0]['id']==0 and groups[0]['roundingMetres']==.02,'Wrong physical support group')
    observed=[tuple(p) for p in groups[0]['points']]
    require(len(observed)==len(expected)==304,'Wrong support vertex count')
    require(all(all(math.isclose(a,b,rel_tol=1e-14,abs_tol=1e-14) for a,b in zip(x,y,strict=True))
                for x,y in zip(observed,expected,strict=True)),'Returned runtime support coordinates differ')


def validation(validator, write=False):
    package=json.loads(OUTPUT.read_bytes()); raw=json.loads(RAW_TENSOR_OUTPUT.read_bytes())
    cases=[('original-tensor',RAW_TENSOR_OUTPUT,FOLDER/'ORIGINAL-TENSOR-REFUSAL.json',raw,False,True),
           ('normalized',OUTPUT,FOLDER/'NORMALIZED-VALIDATOR-REPORT.json',package,True,True)]
    for i,route in enumerate(SOLIDS,1):
        doc={**package,'clearances':[route]}
        cases.append((f'solid-{i}',FOLDER/f'SOLID-{i}-INPUT.json',FOLDER/f'SOLID-{i}-REPORT.json',doc,False,False))
    receipts=[]
    with tempfile.TemporaryDirectory(prefix='meridian-sh040-') as tmp:
        folder=Path(tmp)
        for label,path,saved,doc,valid,clear in cases:
            encoded=(json.dumps(doc,allow_nan=False,separators=(',',':'))+'\n').encode()
            if write and not path.exists(): path.write_bytes(encoded)
            require(path.read_bytes()==encoded,'Changed actual validator input '+label)
            report_path=folder/f'{label}.json'
            result=subprocess.run([str(validator.resolve()),'validate',str(path),'--report',str(report_path)],capture_output=True,check=False,timeout=180)
            report=json.loads(report_path.read_bytes())
            require(report['valid'] is valid and (result.returncode==0) is valid,'Unexpected validator result '+label)
            require(report['clearances']==[{'name':r['name'],'passed':clear} for r in doc['clearances']],'Wrong route witness '+label)
            check_group(report,doc)
            if label=='original-tensor': require(any('symmetric' in r['message'] for r in report['diagnostics']),'Missing exact inertia refusal')
            if write and not saved.exists(): saved.write_bytes(report_path.read_bytes())
            require(saved.read_bytes()==report_path.read_bytes(),'Changed actual raw validator report '+label)
            receipts.append({'case':label,'input':path.name,'inputSHA256':hashlib.sha256(path.read_bytes()).hexdigest(),
                             'report':saved.name,'reportSHA256':hashlib.sha256(saved.read_bytes()).hexdigest(),'exitCode':result.returncode})
    with validator.open('rb') as stream: validator_sha=hashlib.file_digest(stream,'sha256').hexdigest()
    return {'validatorSHA256':validator_sha,'cases':receipts,'supportMembership':'Exact single group id 0 and all 304 original scaled inputs verified'}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('--validator',type=Path); parser.add_argument('--write-receipts',action='store_true'); args=parser.parse_args()
    result=geometry()
    if args.validator: result['validator']=validation(args.validator,args.write_receipts)
    print(json.dumps(result,indent=2))
