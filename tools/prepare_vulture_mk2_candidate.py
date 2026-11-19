"""Separate new <=512-face SH038 approximation; the original archive stays intact."""

import argparse
import hashlib
import json
import math

import numpy as np
from scipy.spatial import ConvexHull

from import_goshawk_envelope_source import components
from prepare_vulture_mk2_source import ASSETS, OUTPUT, authenticated, definition as archival_definition, require


CANDIDATE = OUTPUT.with_name('DERIVED-NATIVE-CANDIDATE.json')
NATIVE_PARTS = {0:0,1:1,2:2,3:3,4:4,29:29,34:78,35:79,79:252,104:277,109:326,110:327,154:500}
SECTORS = 8
OPEN_ROUTES = [
    {'name':f'{side} inner side opening','start':[x,2,-9],'end':[x,2,-4],'radiusMetres':.1}
    for side,x in [('Port',-7.75),('Starboard',7.75)]
] + [
    {'name':f'{side} collar shoulder','start':[x,2,8],'end':[x,2,13],'radiusMetres':.1}
    for side,x in [('Port',-8.75),('Starboard',8.75)]
]


def local_label(part, face, vertices):
    if part not in (2,3,4):
        return (part, 0)
    # Split original annular glazing and both rails by source face centre.
    x,y,_ = [sum(axis)/3 for axis in zip(*(vertices[i] for i in face),strict=True)]
    angle = math.atan2(y-.09,x)
    return part, min(SECTORS-1,int((angle+math.pi)/(2*math.pi)*SECTORS))


def source_groups(model):
    return {vertex: i for i, group in enumerate(components(model)) for vertex in group}


def detailed_mesh(model):
    labels = source_groups(model)
    vertices, triangles, memberships, remap = [], [], [], {}
    for face in model['faces']:
        part = labels[face[0]]
        require(all(labels[i]==part for i in face),'Original detail crosses components')
        group = local_label(part,face,model['vertices'])
        new = []
        for i in face:
            key = (i,group)
            if key not in remap:
                remap[key] = len(vertices)
                vertices.append(model['vertices'][i])
                memberships.append(group)
            new.append(remap[key])
        triangles.append(new)
    require({i for i,_ in remap}==set(range(len(model['vertices']))),'Lost original detailed vertex')
    require([[vertices[i] for i in f] for f in triangles]==[[model['vertices'][i] for i in f] for f in model['faces']],
            'Changed original detailed triangle positions')
    return vertices, triangles, memberships


def reduced_piece(points, limit):
    """Convex inscribed source-vertex hull; no fabricated native coordinates."""
    xyz = np.asarray(sorted(set(map(tuple,points))),dtype=np.float64)
    outer = ConvexHull(xyz)
    xyz = xyz[np.sort(outer.vertices)]
    if len(xyz)>limit:
        a = 0
        b = int(np.argmax(np.linalg.norm(xyz-xyz[a],axis=1)))
        c = int(np.argmax(np.linalg.norm(np.cross(xyz[b]-xyz[a],xyz-xyz[a]),axis=1)))
        normal = np.cross(xyz[b]-xyz[a],xyz[c]-xyz[a])
        d = int(np.argmax(np.abs((xyz-xyz[a])@normal)))
        selection = [a,b,c,d]
        require(len(set(selection))==4,'Degenerate source subdivision')
        while len(selection)<limit:
            distances = np.min(np.linalg.norm(xyz[:,None,:]-xyz[selection][None,:,:],axis=2),axis=1)
            selection.append(int(np.argmax(distances)))
        xyz = xyz[sorted(selection)]
    hull = ConvexHull(xyz)
    used = sorted(map(int,hull.vertices)); remap = {old:i for i,old in enumerate(used)}
    centre = xyz[used].mean(axis=0)
    faces = []
    for tri in hull.simplices:
        a,b,c = map(int,tri)
        if np.dot(np.cross(xyz[b]-xyz[a],xyz[c]-xyz[a]),xyz[a]-centre)<0:
            b,c = c,b
        face = [remap[i] for i in (a,b,c)]
        i = face.index(min(face)); faces.append(face[i:]+face[:i])
    return xyz[used].tolist(), sorted(faces)


def native_mesh(model):
    labels = source_groups(model); groups = {}
    for face in model['faces']:
        part = labels[face[0]]
        if part not in NATIVE_PARTS:
            continue
        group = local_label(NATIVE_PARTS[part],face,model['vertices'])
        groups.setdefault(group,[]).extend(model['vertices'][i] for i in face)
    vertices, triangles, memberships = [], [], []
    for group, points in sorted(groups.items()):
        # Spend the remaining faces on the broad primary surface rather than
        # merging independent parts or sacrificing the hollow collar split.
        budget = 32 if group[0]==0 else 8 if group[0] in (2,3,4) else 12
        reduced, faces = reduced_piece(points,budget)
        triangles.extend([[i+len(vertices) for i in face] for face in faces])
        vertices.extend(reduced); memberships.extend([group]*len(reduced))
    require(len(triangles)<=512,'Derived native exceeds unchanged contract')
    require(set(group[0] for group in groups)==set(NATIVE_PARTS.values()),'Omitted distinct major part')
    return vertices, triangles, memberships


def definition():
    source,_,_ = authenticated(ASSETS)
    archived = archival_definition()
    detail, native = detailed_mesh(source['design']), native_mesh(source['nativeDesign'])
    names = sorted(set(detail[2]) | set(native[2])); slots={name:i for i,name in enumerate(names)}
    def mesh(value):
        vertices,triangles,groups=value
        return {'vertices':vertices,'triangles':triangles,'groups':[slots[g] for g in groups]}
    archived['detail'],archived['native']=mesh(detail),mesh(native)
    archived['clearances']=OPEN_ROUTES
    archived['provenance'].update(archivalCandidateOnly=False,derivedNativeApproximation=True,
                                  originalNativeFaces=3016,derivedNativeFaces=len(native[1]),
                                  groupNames=[f'part-{part:03}-{sector}' for part,sector in names],
                                  originalArchiveSHA256=hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),
                                  scope='Derived candidate; native is a new inscribed approximation of separate original major components; not exact native approval')
    return archived


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    raw=(json.dumps(definition(),allow_nan=False,separators=(',',':'))+'\n').encode()
    if args.check:
        require(CANDIDATE.read_bytes()==raw,'Changed derived native candidate')
    else:
        CANDIDATE.write_bytes(raw)


if __name__=='__main__':
    main()
