"""Bound both SH039 support unions and the separate derived native surface."""
import argparse
from fractions import Fraction
import hashlib
import json
import math
from math import isqrt
import struct
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_vulture_mk3_studio import ASSETS, DETAIL, NATIVE, NAMES, require

MANIFEST_SHA='5c98a65f9a7a21ed571ed58a6fe7900681fdbc0bfb23c04bcc2fa4ce274f8020'


def load(folder=ASSETS):
    raw_manifest=(folder/'SOURCE-GEOMETRY.json').read_bytes()
    require(hashlib.sha256(raw_manifest).hexdigest()==MANIFEST_SHA,'Changed SH039 source manifest')
    manifest=json.loads(raw_manifest)
    def data(name):
        raw=(folder/name).read_bytes(); pin=manifest['provenance']['derivedHashes'][name]
        require(len(raw)==pin['bytes'] and hashlib.sha256(raw).hexdigest()==pin['sha256'], f'Changed SH039 fit input {name}')
        return raw
    source=data('source.vertices'); dv,df=DETAIL[:2]; nv,nf=NATIVE[:2]
    require(source[:16]==struct.pack('<8sII',b'MSH039V1',dv,nv),'Wrong SH039 coordinate stream')
    points=np.frombuffer(source,dtype='<f8',offset=16).reshape(-1,3)
    faces=np.frombuffer(data('source-triangles.bin'),dtype='<u4').reshape(-1,3)
    ids=np.frombuffer(data('source-piece-membership.bin'),dtype='u1')
    candidate=json.loads(data('DERIVED-NATIVE.json'))
    require(points.shape==(dv+nv,3) and faces.shape==(df+nf,3) and len(ids)==len(points) and np.max(np.abs(points))<32,'Wrong original input shape')
    require(np.all(faces[:df]<dv) and np.all(faces[df:]>=dv) and np.all(ids[faces]==ids[faces[:,:1]]),'Wrong original group assignment')
    candidate_points=points[dv+np.array(candidate['sourceVertexIndices'])]
    require(len(candidate['triangles'])==508 and len(candidate_points)==len(candidate['groups']),'Wrong derived topology')
    meshes={
        'detailed':trimesh.Trimesh(points[:dv],faces[:df],process=False),
        'archivedNative':trimesh.Trimesh(points[dv:],faces[df:]-dv,process=False),
        'derivedNative':trimesh.Trimesh(candidate_points,candidate['triangles'],process=False),
    }
    unions={'original':(points,ids),'candidate':(np.concatenate((points[:dv],candidate_points)),np.concatenate((ids[:dv],candidate['groups'])))}
    return meshes,unions


def make_pieces(points, ids):
    result=[]
    for slot in sorted(set(ids)):
        indices=np.flatnonzero(ids==slot)
        unique, first=np.unique(points[indices],axis=0,return_index=True)
        try: piece=fit.audit_fan(unique)
        except AssertionError as error: raise ValueError(f'SH039 support {NAMES[slot]} radial fan not certified') from error
        piece['sourceIds']=indices[first]; piece['name']=NAMES[slot]; result.append(piece)
    return result


def cutoff_for_union(pieces, meshes):
    result={name:0. for name in meshes}
    for piece in pieces:
        tris=fit.split(fit.split(piece['triangles']))
        samples=np.concatenate((tris.mean(axis=1),piece['vertices']))
        for name,mesh in meshes.items():
            _,d,_=fit.nearest(mesh,samples)
            result[name]=max(result[name],float(d.max()))
    return {name:math.ceil(d*1000)+45 for name,d in result.items()}


def union_fit(points, membership, meshes, samples_only=False):
    pieces=make_pieces(points,membership)
    targets=cutoff_for_union(pieces,meshes)
    exact={}
    for name,mesh in meshes.items():
        xyz,scale=fit.dyadic(mesh.vertices)
        exact[name]=([tuple(xyz[int(i)] for i in f) for f in mesh.faces],scale)
    candidates={name:[] for name in meshes}; rows=[]
    for piece in pieces:
        samples=fit.exposed_samples(piece,pieces)
        row={'piece':piece['name'],'exposedSamples':len(samples[0]),'comparisons':{}}
        for name,mesh in meshes.items():
            sample,index=fit.worst_sample(mesh,samples[0])
            if sample is not None:candidates[name].append((sample,index,samples,piece))
            result={'largestSample':sample}
            if not samples_only:result['fullUpperMillimetres']=fit.certified_upper(mesh,piece,targets[name])
            row['comparisons'][name]=result
        rows.append(row)
    summary={}
    for name,values in candidates.items():
        require(bool(values),'No exposed sample')
        sample,index,samples,piece=max(values,key=lambda row:row[0]['metres'])
        fit.certify_exposed_sample(sample,index,samples,piece,pieces,exact[name])
        summary[name]={'piece':piece['name'],'largestCertifiedExposedSample':sample}
        if not samples_only:summary[name]['fullUpperMillimetres']=max(row['comparisons'][name]['fullUpperMillimetres'] for row in rows)
    return {'groups':len(pieces),'refinementCutoffMillimetres':targets,'summary':summary,'pieces':rows}


def direct_lower(source,target):
    """Nominate a vertex or face centroid, then certify against every target triangle."""
    candidates=np.concatenate((source.vertices,source.triangles.mean(axis=1)))
    _,distances,faces=fit.nearest(target,candidates)
    index=int(np.argmax(distances)); lower=int(distances[index]*1000)-1
    exact,scale=fit.dyadic(np.concatenate((source.vertices,target.vertices)))
    count=len(source.vertices)
    if index<count:
        witness={'vertex':index}; point=exact[index]; den=1
    else:
        face=index-count; witness={'faceCentroid':face}; den=3
        point=tuple(map(sum,zip(*(exact[int(i)] for i in source.faces[face]))))
    triangles=[tuple(tuple(v*den for v in exact[count+int(i)]) for i in f) for f in target.faces]
    fit.verify_lower(point,triangles,scale*den,lower)
    return {**witness,'pointMetres':candidates[index].tolist(),'nearestTargetFace':int(faces[index]),'sampledMetres':float(distances[index]),'strictLowerMillimetres':lower}


def direct_upper(source,target,target_mm):
    """Bound all source triangle patches; do not add the envelope's rounding radius."""
    grid=fit.GRID
    vertices=np.rint(target.vertices*grid).astype(np.int64)
    patches=np.rint(source.triangles*grid).astype(np.int64)
    upper=0
    for depth in range(fit.DEPTH+1):
        if not len(patches):break
        squares=fit.patch_upper_squared(target,vertices,patches)
        margin=2*(depth+4)+2
        cutoff=target_mm*grid//1000-margin
        done=squares<=cutoff**2 if cutoff>=0 else np.zeros(len(squares),dtype=bool)
        if depth==fit.DEPTH:done[:]=True
        if np.any(done):upper=max(upper,isqrt(int(squares[done].max()))+1+margin)
        patches=fit.split(patches[~done],integer=True)
    return (upper*1000+grid-1)//grid


def direct_fit(meshes):
    result={}
    for key in ('detailed','archivedNative'):
        for a,b in ((key,'derivedNative'),('derivedNative',key)):
            source,target=meshes[a],meshes[b]
            lower=direct_lower(source,target)
            result[f'{a}To{b}']={'witness':lower,'fullUpperMillimetres':direct_upper(source,target,math.ceil(lower['sampledMetres']*1000)+25)}
    return result


def candidate_coverage_witness(meshes, unions):
    """Find an archived point strictly outside every rounded derived support, if any."""
    points, ids = unions['candidate']
    pieces = make_pieces(points, ids)
    archived = meshes['archivedNative'].vertices
    gaps, directions = [], []
    for piece in pieces:
        axes = np.concatenate((piece['normals'], np.eye(3), -np.eye(3)))
        projections = archived @ axes.T - (piece['vertices'] @ axes.T).max(axis=0)
        best = projections.argmax(axis=1)
        gaps.append(projections[np.arange(len(archived)),best] - fit.RADIUS)
        directions.append(axes[best])
    distances = np.asarray(gaps).min(axis=0)
    witness = int(distances.argmax())
    lower = int(distances[witness]*1000)-1
    if lower <= 0:
        return {'certifiedOutsideWitness':False,'note':'No positive separating-plane witness found; this is not a containment proof.'}
    for piece, axes in zip(pieces,directions,strict=True):
        axis=tuple(int(v) for v in np.rint(axes[witness]*fit.GRID))
        exact,scale=fit.dyadic(np.vstack((archived[witness],piece['vertices'])))
        gap=fit.dot(axis,exact[0])-max(fit.dot(axis,p) for p in exact[1:])
        threshold=(Fraction(lower,1000)+Fraction(1,50))*scale
        require(gap>0 and gap**2>threshold**2*fit.dot(axis,axis),'Archived outside witness not certified against every piece')
    return {'certifiedOutsideWitness':True,'archivedNativeVertex':witness,'pointMetres':archived[witness].tolist(),'strictLowerMillimetresFromRoundedCandidateUnion':lower,'method':'Exact integer separating axes against every candidate support; no inference from viewport.'}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--union',choices=('original','candidate'),help='Certificate against both original LODs')
    parser.add_argument('--samples-only',action='store_true',help='Diagnostic only; no full upper bound')
    parser.add_argument('--coverage-gap',action='store_true',help='Archived vertex outside candidate collision supports, if certified')
    args=parser.parse_args()
    meshes,unions=load()
    if args.coverage_gap:
        result={'hull':'SH039','candidateOriginalCoverage':candidate_coverage_witness(meshes,unions)}
    elif args.union:
        points,members=unions[args.union]
        result={'hull':'SH039','union':args.union,'samplesOnly':args.samples_only,'roundingMetres':.02,**union_fit(points,members,{key:meshes[key] for key in ('detailed','archivedNative')},args.samples_only)}
    else:result={'hull':'SH039','directedSurfaceFit':direct_fit(meshes)}
    sys.stdout.buffer.write((json.dumps(result,indent=2)+'\n').encode('utf-8'))


if __name__=='__main__':main()
