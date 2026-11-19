"""Authenticate SH039, preserve both originals, and author a separate 508-face LOD proposal."""
import argparse
from decimal import Decimal
from fractions import Fraction
import json
import math
from pathlib import Path
import struct
import tomllib

from import_goshawk_envelope_source import components, pin

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh039'
INTEGRATION = ROOT / 'integration/sh039-vulture-mk3'
LEGACY = '0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0'
SOURCE_SHA = '2b69475d2bc724889e10506f88e72df522dd054328c486d5cc67a36d209438c2'
SOURCE_PATH = 'replacement-hulls/SH039-design.toml'
SCALE = Fraction(6000, 611)
ROUNDING = .02
DETAIL = (5132, 8272, 501)
NATIVE = (1812, 3016, 155)
LIMIT = 512
SOURCE_PATHS = {
    'resource.toml': 'Sources/L337Core/Resources/Ships/SH039.toml',
    'native-export.json': 'Sources/L337Core/Resources/OriginalFleetShowroomModels/SH039.json',
    'contract.toml': 'docs/fleet-design-v2/ships/SH039.toml',
    'engineering-budget.json': 'Sources/L337Core/Resources/OriginalFleetPhysical/SH039.json',
    'creator-ownership.json': 'docs/milestones/fleet-physical/distinct-creator-ownership-records/SH039.json',
    'LICENSE': 'LICENSE',
}
PINS = {
    'resource.toml': 'b3c12cbaabfc4163507969fb9bb8c5610303c8637a1b6e1143a0d672152acd6f',
    'native-export.json': '0328e3cca704c0027e66a644a55c7c737b837cd479ca950ecebe08a643fc3c20',
    'contract.toml': 'a64233f42e9882bba36eaba7cb1d997bee73881dff3eb06e72cffc70d3ebba9e',
    'engineering-budget.json': '0e1f5f491676ac779cb7999d111bcf8e049d23358497a29baa588e7c36c26483',
    'creator-ownership.json': '35cb8f5f854620a8716ed47cc6ee0d2f386425e9c3475971816c91cd1b0811a0',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def encode(value):
    return (json.dumps(value, separators=(',', ':'), allow_nan=False) + '\n').encode()


def calibrated(points):
    return [[float(Fraction(value)*SCALE) for value in p] for p in points]


def authenticate(folder):
    raw = {name:(folder/name).read_bytes() for name in PINS}
    for name,data in raw.items():
        require(pin(data)['sha256']==PINS[name], f'Changed original SH039 {name}')
    resource = tomllib.loads(raw['resource.toml'].decode())
    exact = tomllib.loads(raw['resource.toml'].decode(), parse_float=Decimal)
    contract = tomllib.loads(raw['contract.toml'].decode())
    archive, physical, creator = (json.loads(raw[name]) for name in ('native-export.json','engineering-budget.json','creator-ownership.json'))
    require(all(x['id']=='SH039' for x in (resource,contract,archive,physical,creator)) and resource['name']==contract['name']=='Vulture Mk III', 'Wrong SH039 identity')
    require(archive['sourcePath']==contract['source']['geometryFile']==SOURCE_PATH and archive['sourceSHA256']==contract['source']['geometrySHA256']==physical['detailedSourceSHA256']==creator['geometrySHA256']==SOURCE_SHA, 'Wrong original variant')
    require(resource['nativeDesign']==archive['nativeDesign'], 'Native source differs from archive')
    require(physical['contractSHA256']==creator['contractSHA256']==PINS['contract.toml'] and physical['metresPerSourceUnitExact']=='6000/611' and physical['sourceZSpanExact']=='611/200' and physical['targetDetailedLengthMetres']==30, 'Wrong original calibration')
    z = [Fraction(p[2]) for p in exact['design']['vertices']]
    require((max(z)-min(z))*SCALE==30 and physical['dryMassKilograms']==64757 and physical['actualNativePopulation'].startswith('UNISSUED') and not physical['occupiedPeopleCabinsPaidEquipmentGranted'], 'Wrong source physical authority')
    require(contract['geometry']['nativeFaceLimit']==LIMIT and resource['game']['speed']==contract['performance']['maxSpeed']==52.5, 'Wrong native cap or authored speed')
    return resource,exact,raw


def label_components(detail):
    labels = ['fuselage','opaque-cockpit','glazing-band','aft-glazing-rim','fore-glazing-rim']
    for side in ('starboard','port'):
        labels.extend(f'{side}-window-{i}' for i in range(24))
        labels.append(f'{side}-wing')
        for i in range(12 if detail else 4):
            labels.extend([f'{side}-wing-{"detail" if detail else "native"}-panel-{i}']*(4 if detail else 1))
        labels.extend([f'{side}-drive']*2)
        for name, count in (('nacelle-panel',27),('body-panel',16)):
            for i in range(count):
                labels.extend([f'{side}-{name}-{i}']*(4 if detail else 1))
        labels.append(f'{side}-fore-tube')
    return labels


DETAIL_LABELS, NATIVE_LABELS = label_components(True), label_components(False)
NAMES = tuple(dict.fromkeys(DETAIL_LABELS+NATIVE_LABELS))


def source_model(mesh, counts, labels):
    verts, faces = mesh['vertices'], mesh['faces']
    require((len(verts),len(faces),len(components(mesh)))==counts, 'Changed original mesh counts')
    require(mesh['paletteName']=='Dark russet / graphite / silver' and mesh['license']==mesh['appearanceLicense']=='MIT' and 'textures' not in mesh, 'Wrong original material identity')
    require(all(len(p)==3 and all(math.isfinite(v) for v in p) for p in verts), 'Invalid source coordinate')
    require(all(len(f)==len(set(f))==3 and all(type(i) is int and 0<=i<len(verts) for i in f) for f in faces), 'Invalid original face')
    ids = [None]*len(verts)
    for part,label in zip(components(mesh),labels,strict=True):
        for vertex in part:
            ids[vertex]=NAMES.index(label)
    require(all(ids[a]==ids[b]==ids[c] for a,b,c in faces), 'Face crosses support groups')
    return ids


def signed_z(points, face):
    a,b,c=(points[i] for i in face)
    return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])


def reduced_rings(mesh, indices, angular, keep):
    """Rebuild each existing ring strip from a subset of original archived vertices."""
    start=indices[0]
    require(indices==list(range(start,start+len(indices))) and len(indices)%angular==0, 'Unexpected source ring topology')
    n=len(indices)//angular
    originals=[[v-start for v in f] for f in mesh['faces'] if start<=f[0]<start+len(indices)]
    patterns={}
    caps={}
    for face in originals:
        rings=tuple(sorted({v//angular for v in face}))
        if len(rings)==1:
            caps[rings[0]]=face
        elif all(v%angular in (0,1) for v in face):
            patterns.setdefault(rings,[]).append([(v//angular,v%angular) for v in face])
    require(all(len(pattern)==2 for pattern in patterns.values()) and bool(patterns), 'Unexpected source ring strip')
    kept=[start+r*angular+a for r in range(n) for a in keep]
    count=len(keep)
    faces=[]
    for pattern in patterns.values():
        for sector in range(count):
            faces.extend([[ring*count+(sector+step)%count for ring,step in face] for face in pattern])
    points=mesh['vertices']
    for ring,original in caps.items():
        want=signed_z(points,[start+v for v in original])
        require(want!=0,'Degenerate archived cap')
        for j in range(1,count-1):
            face=[ring*count,ring*count+j,ring*count+j+1]
            if signed_z(points,[kept[v] for v in face])*want<0:
                face.reverse()
            faces.append(face)
    return kept,faces


def derived_native(mesh, membership):
    parts=components(mesh)
    selected=(0,1,2,3,4,29,34,35,79,104,109,110,154)
    vertices,faces,groups,materials,source_ids,part_records=[],[],[],[],[],[]
    for part in selected:
        ids=parts[part]
        face_materials={mesh['faceMaterials'][i] for i,f in enumerate(mesh['faces']) if f[0] in ids}
        require(len(face_materials)==1,'Structural component has unexpected multiple materials')
        if part in (29,104):
            kept=ids
            local=[[v-ids[0] for v in f] for f in mesh['faces'] if f[0] in ids]
        else:
            angular=24 if part in (2,3,4) else 12
            keep=[0,2,3,4,6,8,9,10] if part==0 else list(range(0,angular,angular//6))
            kept,local=reduced_rings(mesh,ids,angular,keep)
        base=len(vertices)
        source_ids.extend(kept)
        vertices.extend(mesh['vertices'][v] for v in kept)
        groups.extend(membership[v] for v in kept)
        faces.extend([[v+base for v in f] for f in local])
        materials.extend([next(iter(face_materials))]*len(local))
        part_records.append({'archivedComponent':part,'sourceVertices':kept,'candidateFaceCount':len(local)})
    require(len(faces)==508 and len(faces)<=LIMIT and len(set(source_ids))==len(source_ids), 'Derived native does not meet face cap')
    require(set(v for face in faces for v in face)==set(range(len(vertices))), 'Unreferenced derived vertex')
    return {'vertices':vertices,'triangles':faces,'groups':groups,'faceMaterials':materials,'sourceVertexIndices':source_ids,'parts':part_records}


def bounds(points):
    return [[fn(axis) for axis in zip(*points,strict=True)] for fn in (min,max)]


def visual(points, faces, materials, face_materials):
    data=bytearray(struct.pack('<8sII',b'VUL3VIZ1',3*len(faces),56))
    for face,index in zip(faces,face_materials,strict=True):
        a,b,c=(points[i] for i in face)
        u,v=([j-i for i,j in zip(a,p,strict=True)] for p in (b,c))
        normal=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
        length=math.hypot(*normal)
        require(length>0 and math.isfinite(length),'Degenerate original or candidate face')
        material=materials[index]
        rgba=[(material['colour']>>shift&255)/255 for shift in (24,16,8,0)]
        for p in (a,b,c):
            data.extend(struct.pack('<12fII',*p,*(n/length for n in normal),*rgba,0,0,0xffffffff,int(material['emissive'])))
    return bytes(data)


def route(name,start,end,radius=.10):
    return {'name':name,'start':calibrated([[Decimal(str(v)) for v in start]])[0],'end':calibrated([[Decimal(str(v)) for v in end]])[0],'radiusMetres':radius}


CLEARANCES=(route('starboard-aft-channel',[.89,0,-1.8],[.89,0,-1.15]),route('port-aft-channel',[-.89,0,-1.8],[-.89,0,-1.15]),route('stern-cross-channel',[-.89,0,-1.58],[.89,0,-1.58]))
CONTACTS=(route('under-fuselage-contact',[.1,-.8,-.4],[.1,.1,-.4]),route('starboard-nacelle-contact',[1.9,-.03,-1.3],[1.15,-.03,-1.3]),route('port-nacelle-contact',[-1.9,-.03,-1.3],[-1.15,-.03,-1.3]))


def support_groups(points, ids):
    result={}
    for p,g in zip(points,ids,strict=True):
        result.setdefault(g,[]).append(p)
    return result


def clearance_bound(path,groups):
    lo,hi=bounds((path['start'],path['end']))
    margins=[]
    for points in groups.values():
        a,b=bounds(points)
        margins.append(max(max(u-z,x-v,0) for u,v,x,z in zip(a,b,lo,hi,strict=True))-ROUNDING-path['radiusMetres'])
    return math.floor((min(margins)-.001)*1000)/1000


def first_hit(points,faces,path):
    start,end=path['start'],path['end']
    axis,=[i for i in range(3) if start[i]!=end[i]]
    a,b=[i for i in range(3) if i!=axis]
    hits=[]
    for index,face in enumerate(faces):
        p,q,s=(points[i] for i in face)
        det=(q[a]-p[a])*(s[b]-p[b])-(q[b]-p[b])*(s[a]-p[a])
        if abs(det)<1e-10:continue
        u=((start[a]-p[a])*(s[b]-p[b])-(start[b]-p[b])*(s[a]-p[a]))/det
        v=((q[a]-p[a])*(start[b]-p[b])-(q[b]-p[b])*(start[a]-p[a]))/det
        if min(u,v,1-u-v)<0:continue
        t=(p[axis]+u*(q[axis]-p[axis])+v*(s[axis]-p[axis])-start[axis])/(end[axis]-start[axis])
        if 0<=t<=1:hits.append((t,index,u,v))
    require(bool(hits),'No original source contact')
    t,index,u,v=min(hits)
    require(min(t,1-t,u,v,1-u-v)>0,'Contact does not cross triangle interior')
    return {'face':index,'fraction':t,'pointMetres':[x+t*(y-x) for x,y in zip(start,end,strict=True)],'barycentric':[1-u-v,u,v]}


def prepare(folder=ASSETS):
    r,exact,raw=authenticate(folder)
    d,n=r['design'],r['nativeDesign']
    dg=source_model(d,DETAIL,DETAIL_LABELS); ng=source_model(n,NATIVE,NATIVE_LABELS)
    dp,np=calibrated(exact['design']['vertices']),calibrated(exact['nativeDesign']['vertices'])
    proposal=derived_native(n,ng); cp=[np[i] for i in proposal['sourceVertexIndices']]
    require(len(NAMES)==177,'Wrong support group identities')
    streams={
        'source.vertices':struct.pack('<8sII',b'MSH039V1',len(dp),len(np))+b''.join(struct.pack('<3d',*p) for p in dp+np),
        'source-triangles.bin':b''.join(struct.pack('<3I',*f) for f in d['faces']+[[v+len(dp) for v in f] for f in n['faces']]),
        'source-piece-membership.bin':bytes(dg+ng),
        'PIECES.json':encode({'pieces':list(NAMES),'roundingMetres':ROUNDING}),
        'visual.bin':visual(dp,d['faces'],d['materials'],d['faceMaterials']),
        'native-visual.bin':visual(np,n['faces'],n['materials'],n['faceMaterials']),
        'derived-native-visual.bin':visual(cp,proposal['triangles'],n['materials'],proposal['faceMaterials']),
        'DERIVED-NATIVE.json':encode(proposal),
    }
    provenance={'sourceId':'SH039','revision':LEGACY,'originalSourcePath':SOURCE_PATH,'originalSourceSHA256':SOURCE_SHA,'sourcePaths':SOURCE_PATHS,'sourceHashes':PINS,'derivedHashes':{name:pin(data) for name,data in streams.items()},'calibrationRatio':'6000/611','sourceCoordinates':'relative original units; exact source ratio applied once','physicalStatus':'unqualified authored empty-dry budget; no operating or acquisition authority','license':raw['LICENSE'].decode()}
    def mesh(vertices,faces,groups):return {'vertices':vertices,'triangles':faces,'groups':groups}
    base={'schema':1,'id':'SH039','name':'Vulture Mk III','metresPerUnit':float(SCALE),'roundingMetres':ROUNDING,'detail':mesh(d['vertices'],d['faces'],dg),'native':mesh(n['vertices'],n['faces'],ng),'materials':[{'name':m['name'],'rgb':[m['colour']>>shift&255 for shift in (24,16,8)],'alpha':m['colour']&255,'emissive':m['emissive']} for m in d['materials']],'faceMaterials':d['faceMaterials'],'engineering':None,'flight':{'maxSpeed':52.5},'clearances':list(CLEARANCES),'provenance':{**provenance,'nativeVariant':'archived original; above 512 face cap; expected refusal'}}
    candidate={**base,'id':'SH039-DERIVED-NATIVE-508','name':'Vulture Mk III (derived native candidate)','native':mesh(proposal['vertices'],proposal['triangles'],proposal['groups']),'provenance':{**provenance,'nativeVariant':'508-face structural candidate derived only from archived SH039, not the original and not admitted'}}
    original_groups=support_groups(dp+np,dg+ng); candidate_groups=support_groups(dp+cp,dg+proposal['groups'])
    witness={'openings':[{'route':p,'originalNetMetres':clearance_bound(p,original_groups),'candidateNetMetres':clearance_bound(p,candidate_groups)} for p in CLEARANCES], 'contacts':[{'route':p,'detailed':first_hit(dp,d['faces'],p),'archivedNative':first_hit(np,n['faces'],p),'derivedNative':first_hit(cp,proposal['triangles'],p)} for p in CONTACTS]}
    require(all(min(row['originalNetMetres'],row['candidateNetMetres'])>0 for row in witness['openings']),'Local support blocks an intended aft opening')
    streams['SOURCE-GEOMETRY.json']=encode({'id':'SH039','originalDetailed':{'vertices':DETAIL[0],'faces':DETAIL[1],'components':DETAIL[2],'boundsMetres':bounds(dp)},'originalNative':{'vertices':NATIVE[0],'faces':NATIVE[1],'components':NATIVE[2],'boundsMetres':bounds(np),'faceLimit':LIMIT,'status':'above cap'},'derivedNative':{'vertices':len(cp),'faces':len(proposal['triangles']),'boundsMetres':bounds(cp),'status':'candidate only'},'provenance':provenance})
    return streams,{'VULTURE-MK3-ORIGINAL-STUDIO.json':encode(base),'VULTURE-MK3-CANDIDATE-STUDIO.json':encode(candidate),'CONTACT-WITNESSES.json':encode(witness)}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    for folder,files in zip((ASSETS,INTEGRATION),prepare(),strict=True):
        for name,data in files.items():
            path=folder/name
            if args.check:
                require(path.read_bytes()==data,f'Changed generated SH039 {name}')
            else:
                folder.mkdir(parents=True,exist_ok=True)
                path.write_bytes(data)


if __name__=='__main__':main()
