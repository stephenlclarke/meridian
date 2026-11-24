import json, struct
from pathlib import Path
import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit

root=Path(__file__).resolve().parents[1]/'assets/fleet/sh012'
vert=np.frombuffer((root/'source.vertices').read_bytes(),dtype='<f8',offset=16).reshape(-1,3)
faces=np.frombuffer((root/'source-triangles.bin').read_bytes(),dtype='<u4').reshape(-1,3)
part=(root/'piece-indices.bin').read_bytes()
layout=json.loads((root/'PIECES.json').read_text())
meshes={
 'detailed':(0,trimesh.Trimesh(vert[:2308],faces[:4496],process=False)),
 'native':(2308,trimesh.Trimesh(vert[2308:],faces[4496:]-2308,process=False)),
}
model=[]
offset=12
for r in layout:
    count=struct.unpack_from('<I',part,offset)[0]; offset+=4
    ids=np.frombuffer(part,dtype='<u4',offset=offset,count=count);offset+=4*count
    vertices, first=np.unique(vert[ids],axis=0,return_index=True)
    piece=fit.audit_fan(vertices); piece['sourceIds']=ids[first]
    model.append(piece)
assert offset==len(part)
_,scale,exact=fit.integer_source(vert,meshes)
targets={'detailed':1200,'native':1800}
report=[]
witnesses={kind:[] for kind in meshes}
for spec,piece in zip(layout,model,strict=True):
    sample=fit.exposed_samples(piece,model)
    data={}
    for kind,(_,mesh) in meshes.items():
        record,i=fit.worst_sample(mesh,sample[0])
        if record is not None:
            witnesses[kind].append((record,i,sample,piece,spec['piece']))
        data[kind]={'largestExposedSample':record,
                    'fullPieceSurfaceUpperMillimetres':fit.certified_upper(mesh,piece,targets[kind])}
    report.append({'piece':spec['piece'],'exposedSamples':len(sample[0]),'sourceComparisons':data})
summary={}
for kind,records in witnesses.items():
    samples=sorted(records,key=lambda r:-r[0]['metres'])
    certified=[]
    for row,i,sample,piece,name in samples[:6]:
        try:
            fit.certify_exposed_sample(row,i,sample,piece,model,(exact[kind],scale))
        except AssertionError:
            continue
        certified.append({'piece':name,'value':row})
        if len(certified)==2: break
    assert len(certified)==2
    summary[kind]={'largestSample':{'piece':samples[0][4],'value':samples[0][0]},
                   'independentExposedWitnesses':certified,
                   'fullUpperMillimetres':max(r['sourceComparisons'][kind]['fullPieceSurfaceUpperMillimetres'] for r in report)}
print(json.dumps({'hull':'SH012','nominalPaddingMetres':.02,'summary':summary,'pieces':report},indent=2))
