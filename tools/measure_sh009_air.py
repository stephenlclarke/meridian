"""Independent measured fit, using generic exact-certificate math from the accepted audit."""
import json
from pathlib import Path
import numpy as np
import trimesh
import measure_goshawk_surface_fit as fit

root=Path(__file__).resolve().parents[1]/'assets/fleet/sh009'
raw=(root/'source.vertices').read_bytes()
vert=np.frombuffer(raw,dtype='<f8',offset=16).reshape(-1,3)
faces=np.frombuffer((root/'source-triangles.bin').read_bytes(),dtype='<u4').reshape(-1,3)
assignment=np.frombuffer((root/'source-piece-membership.bin').read_bytes(),dtype='u1')
names=json.loads((root/'SHIELD-PARTITION.json').read_bytes())['pieces']
meshes={
 'detailed':(0,trimesh.Trimesh(vert[:2104],faces[:3532],process=False)),
 'nativeCandidate':(2104,trimesh.Trimesh(vert[2104:],faces[3532:]-2104,process=False)),
}
models=[]
for i in range(len(names)):
    ids=np.flatnonzero(assignment==i)
    unique,first=np.unique(vert[ids],axis=0,return_index=True)
    piece=fit.audit_fan(unique)
    piece['sourceIds']=ids[first]
    models.append(piece)
_,scale,exact=fit.integer_source(vert,meshes)
targets={'detailed':600,'nativeCandidate':1140}
rows=[]
witnesses={k:[] for k in meshes}
for name,piece in zip(names,models,strict=True):
    samples=fit.exposed_samples(piece,models)
    comparisons={}
    for kind,(_,mesh) in meshes.items():
        record,i=fit.worst_sample(mesh,samples[0])
        if record is not None:
            witnesses[kind].append((record,i,samples,piece))
        comparisons[kind]={
            'largestExposedSample':record,
            'fullPieceSurfaceUpperMillimetres':fit.certified_upper(mesh,piece,targets[kind]),
        }
    rows.append({'piece':name,'exposedSamples':len(samples[0]),'sourceComparisons':comparisons})
summary={}
for kind,candidates in witnesses.items():
    first=sorted(candidates,key=lambda r:-r[0]['metres'])[:2]
    for row,i,samples,piece in first:
        fit.certify_exposed_sample(row,i,samples,piece,models,(exact[kind],scale))
    summary[kind]={
        'largestSample':first[0][0],
        'fullUpperMillimetres':max(row['sourceComparisons'][kind]['fullPieceSurfaceUpperMillimetres'] for row in rows),
    }
print(json.dumps({'hull':'SH009','nominalPaddingMetres':.02,'summary':summary,'pieces':rows},indent=2))
