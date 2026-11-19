"""Exact fixed-yaw SH039 physical sweep through the original SH028 lip."""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct
import sys

from prepare_vulture_mk3_studio import ASSETS, DETAIL, NATIVE, ROOT, require

STATION='cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5'
MANIFEST='5c98a65f9a7a21ed571ed58a6fe7900681fdbc0bfb23c04bcc2fa4ce274f8020'
GUARD=21


def points(raw):
    return [tuple(Fraction(value) for value in p) for p in struct.iter_unpack('<3d',raw)]


def bounds(shape):
    return tuple(tuple(fn(axis) for axis in zip(*shape,strict=True)) for fn in (min,max))


def groups(xyz, ids):
    result={}
    for point,index in zip(xyz,ids,strict=True):result.setdefault(index,[]).append(point)
    return result


def clearance(pieces,station,x,z):
    start,end=sorted(map(Fraction,z)); x=Fraction(x)
    swept=[(slot,((x-hi[0],lo[1],start-hi[2]),(x-lo[0],hi[1],end-lo[2]))) for slot,group in pieces.items() for lo,hi in [bounds(group)]]
    gap,piece,face=min((max(max(c-b,a-d) for a,b,c,d in zip(lo,hi,low,high,strict=True)),slot,j) for slot,(lo,hi) in swept for j,(low,high) in enumerate(station))
    return {'netMillimetres':gap*1000//1-GUARD,'limitingPiece':piece,'limitingStationFace':face}


def lip_hit(triangle,vertex,x,z):
    depth=triangle[0][2]; require(all(p[2]==depth for p in triangle),'Wrong lip face')
    start,end=map(Fraction,z); t=(depth+vertex[2]-start)/(end-start)
    x,y=Fraction(x)-vertex[0],vertex[1]
    (ax,ay),(bx,by),(cx,cy)=[p[:2] for p in triangle]
    det=(bx-ax)*(cy-ay)-(by-ay)*(cx-ax)
    require(det!=0,'Degenerate lip face')
    u=((x-ax)*(cy-ay)-(y-ay)*(cx-ax))/det; v=((bx-ax)*(y-ay)-(by-ay)*(x-ax))/det
    weights=(1-u-v,u,v)
    require(0<t<1 and min(weights)>0,'No strict original lip crossing')
    return {'fraction':float(t),'actorCentreZMetres':float(start+t*(end-start)),'pointMetres':list(map(float,(x,y,depth))),'barycentrics':list(map(float,weights))}


def verify(root=ROOT):
    folder=root/'assets/fleet/sh039'; raw=(folder/'SOURCE-GEOMETRY.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest()==MANIFEST,'Changed original manifest')
    manifest=json.loads(raw)
    def data(name):
        blob=(folder/name).read_bytes(); pin=manifest['provenance']['derivedHashes'][name]
        require(hashlib.sha256(blob).hexdigest()==pin['sha256'] and len(blob)==pin['bytes'],f'Changed station input {name}')
        return blob
    source=data('source.vertices'); dv,nv=DETAIL[0],NATIVE[0]
    require(source[:16]==struct.pack('<8sII',b'MSH039V1',dv,nv),'Wrong original source stream')
    xyz=points(source[16:]); ids=data('source-piece-membership.bin'); derived=json.loads(data('DERIVED-NATIVE.json'))
    require(len(xyz)==len(ids)==dv+nv,'Wrong original point count')
    selected=derived['sourceVertexIndices']; candidate=[xyz[dv+i] for i in selected]
    unions={'original':groups(xyz,ids),'candidate':groups(xyz[:dv]+candidate,list(ids[:dv])+derived['groups'])}
    raw=(root/'assets/legacy/sh028.physics').read_bytes(); require(hashlib.sha256(raw).hexdigest()==STATION,'Changed SH028 physics')
    magic,vc,fc=struct.unpack_from('<8sII',raw); require((magic,vc,fc)==(b'MPHYS001',272,488),'Wrong SH028 mesh')
    stop=16+24*vc; vertices=points(raw[16:stop]); triangles=[[vertices[i] for i in f] for f in struct.iter_unpack('<3I',raw[stop:])]; boxes=list(map(bounds,triangles))
    aligned=(152,117); offset=(152,149.5); x=72.6
    lip=[p for tri in triangles[480:488] for p in tri]; low,high=bounds(lip); hlo,hhi=bounds(xyz)
    lead=Fraction(aligned[0])-hhi[2]-Fraction(GUARD,1000); rear=Fraction(aligned[1])-hlo[2]+Fraction(GUARD,1000)
    require(lead>high[2] and rear<low[2],'Whole hull must traverse lip band')
    results={name:{'aligned':clearance(pieces,boxes,0,aligned),'offsetStart':clearance(pieces,boxes,x,[offset[0]]*2)} for name,pieces in unions.items()}
    require(all(row['aligned']['netMillimetres']>0 and row['offsetStart']['netMillimetres']>0 for row in results.values()),'Uncertified aligned/initial clearance')
    archived_index=120; derived_index=selected.index(archived_index)
    witness=xyz[dv+archived_index]; require(witness==candidate[derived_index],'Derived witness is not original native point')
    return {'hull':'SH039','station':'SH028','stationSha256':STATION,'method':'Exact binary64 rational fixed yaw half-turn swept support boxes; 20mm rounding plus 1mm guard; retained original native vertex crosses actual station face interior.', 'alignedActorCentreZMetres':list(aligned),'guardedLeadingTrailingEdgesMetres':[float(lead),float(rear)],'lipBandZMetres':[float(low[2]),float(high[2])],'offsetActorXMetres':x,'offsetActorCentreZMetres':list(offset),'unions':results,'lipCrossing':{'archivedNativeVertex':archived_index,'derivedNativeVertex':derived_index,'stationFace':482,**lip_hit(triangles[482],witness,x,offset)}}


if __name__=='__main__':sys.stdout.buffer.write((json.dumps(verify(),indent=2)+'\n').encode('utf-8'))
