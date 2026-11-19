"""Exact original SH041/SH028 fixed-yaw swept lip controls."""
from fractions import Fraction
import hashlib
import json
import struct
import sys

from prepare_serval_studio import DETAIL, NATIVE, NAMES, ROOT, require

MANIFEST='a953baee13f80d97e490b609650d548b03751a4fdb0468020b6f0b2df4623dc9'
STATION='cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5'
GUARD=21


def points(raw):return [tuple(Fraction(v) for v in p) for p in struct.iter_unpack('<3d',raw)]


def bounds(shape):return tuple(tuple(fn(axis) for axis in zip(*shape,strict=True)) for fn in (min,max))


def clearance(groups,station,x,z):
    a,b=sorted(map(Fraction,z)); x=Fraction(x)
    swept=[((x-hi[0],lo[1],a-hi[2]),(x-lo[0],hi[1],b-lo[2])) for lo,hi in map(bounds,groups)]
    distance,i,j=min((max(max(c-bb,aa-d) for aa,bb,c,d in zip(lo,hi,low,high,strict=True)),i,j) for i,(lo,hi) in enumerate(swept) for j,(low,high) in enumerate(station))
    return {'netMillimetres':distance*1000//1-GUARD,'limitingSupport':i,'limitingStationFace':j}


def intersection(triangle,vertex,x,z):
    depth=triangle[0][2]; require(all(p[2]==depth for p in triangle),'Wrong lip plane'); start,end=map(Fraction,z); t=(depth+vertex[2]-start)/(end-start); x,y=Fraction(x)-vertex[0],vertex[1]
    (ax,ay),(bx,by),(cx,cy)=[p[:2] for p in triangle]; det=(bx-ax)*(cy-ay)-(by-ay)*(cx-ax); require(det!=0,'Degenerate lip triangle')
    u=((x-ax)*(cy-ay)-(y-ay)*(cx-ax))/det; v=((bx-ax)*(y-ay)-(by-ay)*(x-ax))/det; weights=(1-u-v,u,v); require(0<t<1 and min(weights)>0,'No strict original native crossing')
    return {'fraction':float(t),'actorCentreZMetres':float(start+t*(end-start)),'pointMetres':list(map(float,(x,y,depth))),'barycentric':list(map(float,weights))}


def verify(root=ROOT):
    folder=root/'assets/fleet/sh041'; raw=(folder/'SOURCE-GEOMETRY.json').read_bytes(); require(hashlib.sha256(raw).hexdigest()==MANIFEST,'Changed original manifest'); manifest=json.loads(raw)
    def blob(name):
        data=(folder/name).read_bytes(); pin=manifest['provenance']['derivedHashes'][name]; require(len(data)==pin['bytes'] and hashlib.sha256(data).hexdigest()==pin['sha256'],f'Changed source input {name}'); return data
    stream=blob('source.vertices'); dv,nv=DETAIL[0],NATIVE[0]; require(stream[:16]==struct.pack('<8sII',b'MSH041V1',dv,nv),'Wrong hull stream')
    xyz=points(stream[16:]); membership=blob('source-piece-membership.bin'); require(len(xyz)==len(membership)==dv+nv and set(membership)==set(range(len(NAMES))),'Wrong support membership')
    groups=[[] for _ in NAMES]
    for p,g in zip(xyz,membership,strict=True):groups[g].append(p)
    raw=(root/'assets/legacy/sh028.physics').read_bytes(); require(hashlib.sha256(raw).hexdigest()==STATION,'Changed original SH028 station'); magic,vc,fc=struct.unpack_from('<8sII',raw); require((magic,vc,fc)==(b'MPHYS001',272,488),'Wrong station geometry')
    stop=16+24*vc; vertices=points(raw[16:stop]); triangles=[[vertices[i] for i in f] for f in struct.iter_unpack('<3I',raw[stop:])]; boxes=list(map(bounds,triangles)); aligned=[150,116]; offset=[150,147]; x=69.9; lip=[p for t in triangles[480:488] for p in t]; low,high=bounds(lip); vlo,vhi=bounds(xyz); guard=Fraction(GUARD,1000); lead=Fraction(aligned[0])-vhi[2]-guard; rear=Fraction(aligned[1])-vlo[2]+guard
    require(lead>high[2] and rear<low[2],'Whole hull does not traverse lip band')
    clear=clearance(groups,boxes,0,aligned); before=clearance(groups,boxes,x,[offset[0]]*2); require(clear['netMillimetres']>0 and before['netMillimetres']>0,'Aligned or initial offset pose not clear'); vertex=26
    return {'hull':'SH041','station':'SH028','stationSha256':STATION,'method':'Exact original binary64 rational fixed half-turn swept-support boxes, with 20 mm rounding and 1 mm guard; actual native vertex crosses original station triangle interior.','alignedActorCentreZMetres':aligned,'guardedLeadingTrailingEdgesMetres':[float(lead),float(rear)],'lipBandZMetres':[float(low[2]),float(high[2])],'aligned':clear,'offsetActorXMetres':x,'offsetActorCentreZMetres':offset,'offsetStart':before,'nativeLipCrossing':{'nativeLocalVertex':vertex,'nativeGlobalVertex':dv+vertex,'stationFace':482,**intersection(triangles[482],xyz[dv+vertex],x,offset)}}


if __name__=='__main__':sys.stdout.buffer.write((json.dumps(verify(),indent=2)+'\n').encode('utf-8'))
