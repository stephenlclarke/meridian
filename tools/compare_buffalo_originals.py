import json
from pathlib import Path
import numpy as np
import trimesh
import measure_goshawk_surface_fit as fit
root=Path(__file__).resolve().parents[1]/'assets/fleet/sh013'
v=np.frombuffer((root/'source.vertices').read_bytes(),dtype='<f8',offset=16).reshape(-1,3)
f=np.frombuffer((root/'source-triangles.bin').read_bytes(),dtype='<u4').reshape(-1,3)
models={'detailed':(0,trimesh.Trimesh(v[:2056],f[:3460],process=False)),
        'native':(2056,trimesh.Trimesh(v[2056:],f[3460:]-2056,process=False))}
exact,scale,triangles=fit.integer_source(v,models)
report={}
for against,vertex in [('detailed',2099),('native',1581)]:
    _,dist,face=fit.nearest(models[against][1],v[vertex:vertex+1])
    strict=int(dist[0]*1000)-1
    fit.verify_lower(exact[vertex],triangles[against],scale,strict)
    report[against]={'mandatoryOppositeSourceVertex':vertex,'nearestFace':int(face[0]),
                     'sampledMetres':float(dist[0]),'strictLowerMillimetres':strict}
print(json.dumps(report,indent=2))
