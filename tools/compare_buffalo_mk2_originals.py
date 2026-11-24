import json
from pathlib import Path
import numpy as np
import trimesh
import measure_goshawk_surface_fit as fit
root=Path(__file__).resolve().parents[1]/'assets/fleet/sh014'
v=np.frombuffer((root/'source.vertices').read_bytes(),dtype='<f8',offset=16).reshape(-1,3)
f=np.frombuffer((root/'source-triangles.bin').read_bytes(),dtype='<u4').reshape(-1,3)
models={'detailed':(0,trimesh.Trimesh(v[:2056],f[:3460],process=False)),
        'native':(2056,trimesh.Trimesh(v[2056:],f[3460:]-2056,process=False))}
exact,scale,triangles=fit.integer_source(v,models)
report={}
for against, offsets in [('detailed', range(2056, 2336)), ('native', range(2056))]:
    ids = np.asarray(list(offsets))
    _,dist,face=fit.nearest(models[against][1],v[ids])
    i=int(dist.argmax())
    vertex=int(ids[i])
    strict=int(dist[i]*1000)-1
    fit.verify_lower(exact[vertex],triangles[against],scale,strict)
    report[against]={'mandatoryOppositeSourceVertex':vertex,'nearestFace':int(face[i]),
                     'sampledMetres':float(dist[i]),'strictLowerMillimetres':strict}
print(json.dumps(report,indent=2))
