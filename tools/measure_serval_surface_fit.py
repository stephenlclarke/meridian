"""Certified exposed SH041 rounded support fit to both original meshes."""
import argparse
import hashlib
import json
import math
import struct
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_serval_studio import ASSETS, DETAIL, NATIVE, NAMES, require

MANIFEST='a953baee13f80d97e490b609650d548b03751a4fdb0468020b6f0b2df4623dc9'
DEPTH=8


def load(folder=ASSETS):
    raw=(folder/'SOURCE-GEOMETRY.json').read_bytes(); require(hashlib.sha256(raw).hexdigest()==MANIFEST,'Changed original SH041 manifest'); manifest=json.loads(raw)
    def data(name):
        blob=(folder/name).read_bytes(); pin=manifest['provenance']['derivedHashes'][name]; require(len(blob)==pin['bytes'] and hashlib.sha256(blob).hexdigest()==pin['sha256'],f'Changed SH041 fit input {name}'); return blob
    stream=data('source.vertices'); dv,df=DETAIL[:2]; nv,nf=NATIVE[:2]; require(stream[:16]==struct.pack('<8sII',b'MSH041V1',dv,nv),'Wrong metre stream')
    points=np.frombuffer(stream,dtype='<f8',offset=16).reshape(-1,3); faces=np.frombuffer(data('source-triangles.bin'),dtype='<u4').reshape(-1,3); members=np.frombuffer(data('source-piece-membership.bin'),dtype='u1')
    require(points.shape==(dv+nv,3) and faces.shape==(df+nf,3) and len(members)==len(points) and set(members)==set(range(len(NAMES))) and np.max(np.abs(points))<32,'Wrong source input shape')
    require(np.all(faces[:df]<dv) and np.all(faces[df:]>=dv) and np.all(members[faces]==members[faces[:,:1]]),'Source triangle crosses groups or LODs')
    meshes={'detailed':(0,trimesh.Trimesh(points[:dv],faces[:df],process=False)),'native':(dv,trimesh.Trimesh(points[dv:],faces[df:]-dv,process=False))}
    return points,[np.flatnonzero(members==i) for i in range(len(NAMES))],meshes


def pieces_for(points,groups):
    pieces=[]
    for name,ids in zip(NAMES,groups,strict=True):
        xyz,first=np.unique(points[ids],axis=0,return_index=True)
        try:piece=fit.audit_fan(xyz)
        except AssertionError as error:raise ValueError(f'Cannot certify SH041 radial fan {name}') from error
        piece['sourceIds']=ids[first]; pieces.append(piece)
    return pieces


def targets(pieces,meshes):
    estimates={key:0. for key in meshes}
    for piece in pieces:
        samples=np.concatenate((fit.split(fit.split(piece['triangles'])).mean(axis=1),piece['vertices']))
        for key,(_,mesh) in meshes.items():
            _,distances,_=fit.nearest(mesh,samples); estimates[key]=max(estimates[key],float(distances.max()))
    return {key:math.ceil(value*1000)+45 for key,value in estimates.items()}


def bounded_upper(mesh,piece,cutoff):
    previous=fit.DEPTH
    try:
        fit.DEPTH=DEPTH; return fit.certified_upper(mesh,piece,cutoff)
    finally:fit.DEPTH=previous


def audit():
    points,groups,meshes=load(); pieces=pieces_for(points,groups); _,scale,exact=fit.integer_source(points,meshes); cutoff=targets(pieces,meshes); witnesses={key:[] for key in meshes}; rows=[]
    for name,piece in zip(NAMES,pieces,strict=True):
        samples=fit.exposed_samples(piece,pieces); comparisons={}
        for key,(_,mesh) in meshes.items():
            sample,index=fit.worst_sample(mesh,samples[0]); comparisons[key]={'largestSample':sample,'fullUpperMillimetres':bounded_upper(mesh,piece,cutoff[key])}
            if sample is not None:witnesses[key].append((sample,index,samples,piece,name))
        rows.append({'piece':name,'exposedSamples':len(samples[0]),'comparisons':comparisons})
    summary={}
    for key,candidates in witnesses.items():
        require(bool(candidates),'No exposed sample'); sample,index,samples,piece,name=max(candidates,key=lambda row:row[0]['metres']); fit.certify_exposed_sample(sample,index,samples,piece,pieces,(exact[key],scale))
        summary[key]={'piece':name,'largestCertifiedExposedSample':sample,'fullUpperMillimetres':max(row['comparisons'][key]['fullUpperMillimetres'] for row in rows)}
    return {'hull':'SH041','roundingMetres':.02,'refinementDepth':DEPTH,'refinementCutoffMillimetres':cutoff,'summary':summary,'pieces':rows}


def original_difference():
    points,_,meshes=load(); exact,scale,triangles=fit.integer_source(points,meshes); dv=DETAIL[0]; result={}
    for key,ids in (('detailed',range(dv,len(points))),('native',range(dv))):
        ids=np.asarray(ids); _,d,face=fit.nearest(meshes[key][1],points[ids]); index=int(d.argmax()); v=int(ids[index]); lower=int(d[index]*1000)-1; fit.verify_lower(exact[v],triangles[key],scale,lower)
        result[key]={'oppositeSourceVertex':v,'nearestFace':int(face[index]),'sampledMetres':float(d[index]),'strictLowerMillimetres':lower}
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--difference',action='store_true');args=parser.parse_args(); result=original_difference() if args.difference else audit(); sys.stdout.buffer.write((json.dumps(result,indent=2)+'\n').encode('utf-8'))


if __name__=='__main__':main()
