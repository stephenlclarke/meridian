"""Authenticate original SH041 and prepare its exact source-local Studio package."""
import argparse
import json
import math
from pathlib import Path
import struct
import tomllib

from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components, pin

ROOT=Path(__file__).resolve().parents[1]
ASSETS=ROOT/'assets/fleet/sh041'
INTEGRATION=ROOT/'integration/sh041-serval-fighter'
LEGACY='0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0'
NATIVE_PATH='fleet-completion-20261019/batch-02-native-lod/SH041-design.toml'
NATIVE_SHA='5d3e7b5da7bee8f62a47aeac58f13064e185d6dc49c842863da82e2a97679fa6'
DETAIL=(2172,4236,27)
NATIVE=(128,224,8)
ROUNDING=.02
PATHS={
    'resource.toml':'Sources/L337Core/Resources/Ships/SH041.toml',
    'native-export.json':'Sources/L337Core/Resources/OriginalFleetShowroomModels/SH041.json',
    'contract.toml':'docs/fleet-design-v2/ships/SH041.toml',
    'engineering-budget.json':'Sources/L337Core/Resources/OriginalFleetPhysical/SH041.json',
    'finite-dry-reference.json':'Sources/L337Core/Resources/ServalPhysical/SH041-finite-dry-source.json',
    'LICENSE':'LICENSE',
}
PINS={
    'resource.toml':'d05b012c8860c93c87e1a9394b0caca0a50877b9008d2e62766cacdde8a43fe9',
    'native-export.json':'f8f3348fe645ce6bf33c0cb1c8ff9165f1ab8e56a23d2f324cd060d64331033d',
    'contract.toml':'7f7213ef84d6fc2e1895bf0097f9106c37df199b33d72bf6c78c975a9df197bb',
    'engineering-budget.json':'fc24f453e5fa80317db52c61ffff755f1f7cd503196bd34b8a71f019edd14575',
    'finite-dry-reference.json':'832462f19810838e4f3e721b4d7968c0bf254d15b4334f3d14c2ce014932a49e',
    'LICENSE':'033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
TEXTURE_PINS={
    'l337-patrol-diffuse':'9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse':'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details':'35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
DETAIL_NAMES=('fuselage','port-wing','port-strut','port-drive','port-drive','port-drive','starboard-wing','starboard-strut','starboard-drive','starboard-drive','starboard-drive','glazing','canopy-crest','port-pipe','port-pipe-mount',*(f'port-clamp-{i}' for i in range(5)),'starboard-pipe','starboard-pipe-mount',*(f'starboard-clamp-{i}' for i in range(5)))
NATIVE_NAMES=tuple(DETAIL_NAMES[i] for i in (0,1,2,3,6,7,8,11))
NAMES=tuple(dict.fromkeys(DETAIL_NAMES))
CLEARANCES=(
    {'name':'port-upper-aft-channel','start':[-4,1.5,-18],'end':[-4,1.5,-9],'radiusMetres':.10},
    {'name':'starboard-upper-aft-channel','start':[4,1.5,-18],'end':[4,1.5,-9],'radiusMetres':.10},
    {'name':'stern-between-drives','start':[0,0,-21],'end':[0,0,-12.8],'radiusMetres':.10},
)
CONTACTS=(
    {'name':'lower-fuselage-contact','start':[.45,-6,-5],'end':[.45,1,-5],'radiusMetres':.10},
    {'name':'starboard-wing-contact','start':[15,.23,-1.27],'end':[5,.23,-1.27],'radiusMetres':.10},
    {'name':'port-wing-contact','start':[-15,.23,-1.27],'end':[-5,.23,-1.27],'radiusMetres':.10},
)


def require(condition,message):
    if not condition:raise ValueError(message)


def encode(value):return (json.dumps(value,separators=(',',':'),allow_nan=False)+'\n').encode()


def authenticate(folder):
    raw={name:(folder/name).read_bytes() for name in PINS}
    for name,value in raw.items():require(pin(value)['sha256']==PINS[name],f'Changed original SH041 {name}')
    resource,contract=(tomllib.loads(raw[name].decode()) for name in ('resource.toml','contract.toml'))
    export,physical,dry=(json.loads(raw[name]) for name in ('native-export.json','engineering-budget.json','finite-dry-reference.json'))
    require(all(x['id']=='SH041' for x in (resource,contract,export,physical,dry)) and resource['name']==contract['name']==dry['name']=='Serval Fighter','Wrong SH041 identity')
    require(export['sourcePath']==contract['source']['geometryFile']==NATIVE_PATH and export['sourceSHA256']==contract['source']['geometrySHA256']==dry['nativeSourceSHA256']==NATIVE_SHA,'Wrong original native identity')
    require(resource['nativeDesign']==export['nativeDesign'],'Original native and archive differ')
    require(export['coordinateUnits']==physical['coordinateUnits']==dry['coordinateUnits']=='metres' and physical['metresPerSourceUnit']==dry['metresPerSourceUnit']==1,'Wrong original metre calibration')
    require(dry['nativeVertices']==128 and dry['nativeTriangles']==224 and dry['emptyDryReference']['massKg']==physical['dryMassKilograms']==23775 and dry['emptyStateOnly'] and not dry['nativeEvidenceQualified'] and not dry['operatingAuthorityGranted'] and not dry['peopleOrFuelCargoEquipmentGranted'],'Wrong finite dry authority')
    require(dry['r3SHA256']==PINS['contract.toml'] and dry['engineeringSHA256']==PINS['engineering-budget.json'] and dry['resourceSHA256']==PINS['resource.toml'],'Wrong finite dry source identity')
    require(contract['geometry']['nativeFaceLimit']==512 and resource['game']['speed']==contract['performance']['maxSpeed']==43.6,'Wrong authored performance or cap')
    for name,digest in TEXTURE_PINS.items():
        value=(folder.parent/'sh001'/f'{name}.rgba').read_bytes(); require(len(value)==4194304 and pin(value)['sha256']==digest,f'Changed original map {name}')
    return resource,raw


def bounds(points):return [[fn(axis) for axis in zip(*points,strict=True)] for fn in (min,max)]


def membership(mesh,counts,names):
    parts=components(mesh); points=mesh['vertices']; faces=mesh['faces']
    require((len(points),len(faces),len(parts))==counts,'Wrong original mesh counts')
    require(mesh['paletteName']=='slate-orange' and mesh['license']==mesh['appearanceLicense']=='MIT','Wrong appearance')
    require([t['asset'] for t in mesh['textures']]==TEXTURES and all(t['repeat'] and t['license']=='MIT' for t in mesh['textures']),'Wrong shared maps')
    require(all(len(p)==3 and all(math.isfinite(v) for v in p) for p in points),'Invalid source coordinate')
    require(all(len(f)==len(set(f))==3 and all(type(i) is int and 0<=i<len(points) for i in f) for f in faces),'Invalid source face')
    result=[None]*len(points)
    for part,name in zip(parts,names,strict=True):
        for i in part:result[i]=NAMES.index(name)
    require(all(result[a]==result[b]==result[c] for a,b,c in faces),'Original face crosses support groups')
    return result


def visual(mesh):
    result=bytearray(struct.pack('<8sII',b'SERVVIZ1',len(mesh['faces'])*3,56))
    for face,index,mapping in zip(mesh['faces'],mesh['faceMaterials'],mesh['faceTextures'],strict=True):
        a,b,c=(mesh['vertices'][i] for i in face); u,v=([q-p for p,q in zip(a,x,strict=True)] for x in (b,c))
        n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]; length=math.hypot(*n); slot=mapping['texture']
        require(length>0 and math.isfinite(length) and -1<=slot<len(TEXTURES),'Invalid original appearance face')
        material=mesh['materials'][index]; rgba=[(material['colour']>>shift&255)/255 for shift in (24,16,8,0)]
        for p,uv in zip((a,b,c),mapping['uv'],strict=True):result.extend(struct.pack('<12fII',*p,*(x/length for x in n),*rgba,*uv,slot&0xffffffff,int(material['emissive'])))
    return bytes(result)


def clearance_bound(path,groups):
    lo,hi=bounds((path['start'],path['end'])); margins=[]
    for group in groups:
        a,b=bounds(group); margins.append(max(max(u-z,x-v,0) for u,v,x,z in zip(a,b,lo,hi,strict=True))-ROUNDING-path['radiusMetres'])
    return round(min(margins)-.001,6)


def first_hit(mesh,path):
    start,end=path['start'],path['end']; axis,=[i for i in range(3) if start[i]!=end[i]]; a,b=[i for i in range(3) if i!=axis]; hits=[]
    for i,face in enumerate(mesh['faces']):
        p,q,s=(mesh['vertices'][v] for v in face); det=(q[a]-p[a])*(s[b]-p[b])-(q[b]-p[b])*(s[a]-p[a])
        if abs(det)<1e-10:continue
        u=((start[a]-p[a])*(s[b]-p[b])-(start[b]-p[b])*(s[a]-p[a]))/det; v=((q[a]-p[a])*(start[b]-p[b])-(q[b]-p[b])*(start[a]-p[a]))/det
        if min(u,v,1-u-v)<0:continue
        t=(p[axis]+u*(q[axis]-p[axis])+v*(s[axis]-p[axis])-start[axis])/(end[axis]-start[axis])
        if 0<=t<=1:hits.append((t,i,u,v))
    require(bool(hits),'No original source contact')
    t,i,u,v=min(hits); require(min(t,1-t,u,v,1-u-v)>.01,'Contact grazes original source face')
    return {'face':i,'fraction':t,'pointMetres':[p+t*(q-p) for p,q in zip(start,end,strict=True)],'barycentric':[1-u-v,u,v]}


def prepare(folder=ASSETS):
    resource,raw=authenticate(folder); d,n=resource['design'],resource['nativeDesign']; dg=membership(d,DETAIL,DETAIL_NAMES); ng=membership(n,NATIVE,NATIVE_NAMES)
    points=d['vertices']+n['vertices']; labels=dg+ng; groups=[[p for p,g in zip(points,labels,strict=True) if g==i] for i in range(len(NAMES))]
    require(len(NAMES)==23 and all(groups),'Wrong support count')
    faces=d['faces']+[[i+DETAIL[0] for i in f] for f in n['faces']]
    streams={'source.vertices':struct.pack('<8sII',b'MSH041V1',DETAIL[0],NATIVE[0])+b''.join(struct.pack('<3d',*p) for p in points),'source-triangles.bin':b''.join(struct.pack('<3I',*f) for f in faces),'source-piece-membership.bin':bytes(labels),'PIECES.json':encode({'pieces':list(NAMES),'roundingMetres':ROUNDING}),'visual.bin':visual(d),'native-visual.bin':visual(n)}
    provenance={'sourceId':'SH041','revision':LEGACY,'sourcePaths':PATHS,'sourceHashes':PINS,'derivedHashes':{name:pin(data) for name,data in streams.items()},'originalMetresPerUnit':1,'approvedNativeSource':NATIVE_PATH,'approvedNativeSHA256':NATIVE_SHA,'physicalStatus':'authored empty-dry references only; no operating authority','license':raw['LICENSE'].decode()}
    def mesh(m,g):return {'vertices':m['vertices'],'triangles':m['faces'],'groups':g}
    definition={'schema':1,'id':'SH041','name':'Serval Fighter','metresPerUnit':1,'roundingMetres':ROUNDING,'detail':mesh(d,dg),'native':mesh(n,ng),'materials':[{'name':m['name'],'rgb':[m['colour']>>shift&255 for shift in (24,16,8)],'alpha':m['colour']&255,'emissive':m['emissive']} for m in d['materials']],'faceMaterials':d['faceMaterials'],'faceTextures':[{'uv':m['uv'],'texture':d['textures'][m['texture']]['asset'] if m['texture']>=0 else None} for m in d['faceTextures']],'engineering':None,'flight':{'maxSpeed':43.6},'clearances':list(CLEARANCES),'provenance':provenance}
    openings=[{'route':p,'conservativeNetClearanceMetres':clearance_bound(p,groups)} for p in CLEARANCES]; require(all(row['conservativeNetClearanceMetres']>0 for row in openings),'Blocked original channel')
    contacts=[{'route':p,'detailed':first_hit(d,p),'archivedNative':first_hit(n,p)} for p in CONTACTS]
    streams['SOURCE-GEOMETRY.json']=encode({'id':'SH041','name':'Serval Fighter','detailed':dict(zip(('vertices','faces','components'),DETAIL,strict=True),boundsMetres=bounds(d['vertices'])),'native':dict(zip(('vertices','faces','components'),NATIVE,strict=True),boundsMetres=bounds(n['vertices']),faceLimit=512),'supportGroups':len(NAMES),'provenance':provenance})
    return streams,{'SERVAL-STUDIO.json':encode(definition),'CONTACT-WITNESSES.json':encode({'openings':openings,'contacts':contacts})}


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    for folder,files in zip((ASSETS,INTEGRATION),prepare(),strict=True):
        for name,data in files.items():
            path=folder/name
            if args.check:require(path.read_bytes()==data,f'Changed generated SH041 {name}')
            else:
                folder.mkdir(parents=True,exist_ok=True);path.write_bytes(data)


if __name__=='__main__':main()
