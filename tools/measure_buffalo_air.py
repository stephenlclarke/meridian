"""Independent measured fit, using generic exact-certificate math from the accepted audit."""
import json
from pathlib import Path
import numpy as np
import trimesh
import measure_goshawk_surface_fit as fit

root=Path(__file__).resolve().parents[1]/'assets/fleet/sh013'
raw=(root/'source.vertices').read_bytes()
vert=np.frombuffer(raw,dtype='<f8',offset=16).reshape(-1,3)
faces=np.frombuffer((root/'source-triangles.bin').read_bytes(),dtype='<u4').reshape(-1,3)
assignment=np.frombuffer((root/'source-piece-membership.bin').read_bytes(),dtype='u1')
names=json.loads((root/'PIECES.json').read_bytes())['pieces']
meshes={
 'detailed':(0,trimesh.Trimesh(vert[:2056],faces[:3460],process=False)),
 'nativeCandidate':(2056,trimesh.Trimesh(vert[2056:],faces[3460:]-2056,process=False)),
}
def exact_small_fan(vertices):
    # The tilted eight-point right bracket is nearly coplanar in floating Qhull.
    # Enumerate its exact binary64 support facets and verify their closed outward fan.
    from collections import Counter
    from itertools import combinations, pairwise
    points, _ = fit.dyadic(vertices)
    center = tuple(map(sum, zip(*points)))
    faces, normals = [], []
    edges = Counter()
    for a, b, c in combinations(range(len(points)), 3):
        normal = fit.cross(fit.sub(points[b], points[a]), fit.sub(points[c], points[a]))
        h = fit.dot(normal, points[a])
        offset = h * len(points) - fit.dot(normal, center)
        if not offset:
            continue
        if offset < 0:
            b, c = c, b
            normal = tuple(-x for x in normal)
            h = -h
        if max(fit.dot(normal, point) for point in points) != h:
            continue
        unit = np.asarray(normal, dtype=float)
        normals.append(unit / np.linalg.norm(unit))
        faces.append((a,b,c))
        for edge in pairwise((a,b,c,a)):
            edges[edge] += 1
    assert faces and all(count == edges[(b,a)] == 1 for (a,b),count in edges.items())
    normals = np.array(normals)
    return {
        'vertices':vertices, 'triangles':vertices[faces], 'supportFaces':list(zip(faces,normals)),
        'normals':normals, 'offsets':(vertices@normals.T).max(axis=0),
        'min':vertices.min(axis=0)-.02, 'max':vertices.max(axis=0)+.02,
    }

models=[]
for i in range(len(names)):
    ids=np.flatnonzero(assignment==i)
    unique,first=np.unique(vert[ids],axis=0,return_index=True)
    piece=(exact_small_fan(unique) if names[i]=='right-thruster-bracket' else fit.audit_fan(unique))
    piece['sourceIds']=ids[first]
    models.append(piece)
_,scale,exact=fit.integer_source(vert,meshes)
targets={'detailed':1800,'nativeCandidate':3000}
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
print(json.dumps({'hull':'SH013','nominalPaddingMetres':.02,'summary':summary,'pieces':rows},indent=2))
