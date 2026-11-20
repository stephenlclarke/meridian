import json
from pathlib import Path
import numpy as np
import trimesh
import measure_goshawk_surface_fit as fit

root=Path(__file__).resolve().parents[1]/'assets/fleet/sh012'
v=np.frombuffer((root/'source.vertices').read_bytes(),dtype='<f8',offset=16).reshape(-1,3)
f=np.frombuffer((root/'source-triangles.bin').read_bytes(),dtype='<u4').reshape(-1,3)
meshes={'detailed':(0,trimesh.Trimesh(v[:2308],f[:4496],process=False)),
        'native':(2308,trimesh.Trimesh(v[2308:],f[4496:]-2308,process=False))}
exact,scale,triangles=fit.integer_source(v,meshes)
w=np.array([(a,b,12-a-b) for a in range(13) for b in range(13-a)])
result={}
for name,other,faces in [('detailed','native',f[:4496]),('native','detailed',f[4496:])]:
    mesh=meshes[other][1]
    # A fixed 12-unit barycentric lattice. Whole opposite mesh checks follow.
    # Keep only numerator 12 at vertices for the larger detailed input.
    weights=np.eye(3)*12 if name=='detailed' else w
    samples=np.einsum('wi,fij->fwj',weights,v[faces]).reshape(-1,3)/12
    _,d,near=fit.nearest(mesh,samples)
    j=int(np.argmax(d)); local,wid=divmod(j,len(weights))
    chosen=[int(x) for x in weights[wid]]
    ids=[int(x) for x in faces[local]]
    num=tuple(sum(chosen[k]*exact[ids[k]][i] for k in range(3)) for i in range(3))
    check_faces=[tuple(tuple(x*12 for x in point) for point in face) for face in triangles[other]]
    lower=int(d[j]*1000)-1
    fit.verify_lower(num,check_faces,scale*12,lower)
    result[name]={'sourceFace':local,'globalVertexIds':ids,'weightNumeratorsOver12':chosen,
                  'oppositeNearestFace':int(near[j]),'sampledMetres':float(d[j]),'strictLowerMillimetres':lower}
print(json.dumps(result,indent=2))
