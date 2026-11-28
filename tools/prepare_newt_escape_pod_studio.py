"""Build the exact SH040 cryostasis capsule geometry and original appearance for Studio."""

import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT/'assets/fleet/sh040'
OUTPUT = ROOT/'integration/sh040-newt-escape-pod/NEWT-STUDIO.json'
RAW_TENSOR_OUTPUT = OUTPUT.with_name('NEWT-ORIGINAL-TENSOR.json')
PINS = {
    'resource.toml':'510a2059ce49a7d902c11ca65303264afa8ee103cd1a9e9ad823dbe05fee816c',
    'native-export.json':'d8bdfba7e35120cd6b020b4ca5878da6e3f3e9278bbfc5ee2c65e86280510e1d',
    'contract.toml':'6dd8182abf9a00406132db7926c962d470ea7f6c112bb29fad8646aef9aec0ed',
    'engineering.json':'0d7cd491b77b6e14016daa8a3c5e622416963870f669f80d63ad3b4d74aaee17',
    'LICENSE':'033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
SOURCE_SHA='5b52208ecf64228a843dc792ff4614af3ff9109034de4bedb6ddb0b7918072d9'
SOURCE_PATH='docs/fleet-design-v2/successors/SH040-cryostasis.toml'
OPEN_ROUTES=[
    {'name':name,'start':[x,y,-1.5],'end':[x,y,1.5],'radiusMetres':.1}
    for name,x,y in [('Port exterior',-.8,0),('Starboard exterior',.8,0),('Above closed lid',0,.7),('Below vessel',0,-.75)]
]


def require(condition, message):
    if not condition: raise ValueError(message)


def authenticated(folder):
    raw={name:(folder/name).read_bytes() for name in PINS}
    for name,data in raw.items():
        require(hashlib.sha256(data).hexdigest()==PINS[name], f'Changed original Newt {name}')
    resource=tomllib.loads(raw['resource.toml'].decode()); contract=tomllib.loads(raw['contract.toml'].decode())
    archived=json.loads(raw['native-export.json']); engineering=json.loads(raw['engineering.json'])
    require(resource['id']==contract['id']==archived['id']==engineering['id']=='SH040' and resource['name']==contract['name']=='Newt Escape Pod','Wrong capsule identity')
    require(contract['objectKind']=='capsule' and contract['access']['purchasePolicy']=='never' and contract['access']['playerFlyable'] is True,'Wrong original capsule scope')
    require(archived['sourcePath']==contract['source']['geometryFile']==SOURCE_PATH and archived['sourceSHA256']==contract['source']['geometrySHA256']==SOURCE_SHA,'Wrong cryostasis successor')
    require(archived['nativeDesign']==resource['nativeDesign'],'Native archive mismatch')
    require(engineering['contractSHA256']==PINS['contract.toml'] and engineering['metresPerSourceUnitExact']=='60/19','Wrong adopted capsule engineering')
    require(contract['geometry']['nativeFaceLimit']==512,'Wrong original native budget')
    for domain,counts in [('design',(256,415)),('nativeDesign',(48,92))]:
        mesh=resource[domain]
        require((len(mesh['vertices']),len(mesh['faces']))==counts,'Wrong original topology')
        require(mesh['license']==mesh['appearanceLicense']=='MIT' and mesh['paletteName']=='newt-cryostasis','Wrong capsule appearance')
    return resource,engineering,raw['LICENSE'].decode()


def triangulate(mesh):
    triangles, materials=[],[]
    for face,material in zip(mesh['faces'],mesh['faceMaterials'],strict=True):
        require(len(face) in (3,4) and len(set(face))==len(face),'Unexpected original polygon')
        for i in range(1,len(face)-1):
            triangles.append([face[0],face[i],face[i+1]]); materials.append(material)
    return {'vertices':mesh['vertices'],'triangles':triangles,'groups':[0]*len(mesh['vertices'])},materials


def definition(folder=ASSETS, normalize=True):
    source,engineering,license_text=authenticated(folder)
    detail,materials=triangulate(source['design']); native,_=triangulate(source['nativeDesign'])
    require(len(detail['triangles'])==416 and len(native['triangles'])==92,'Wrong triangulated topology')
    require(detail['vertices'][:48]==native['vertices'] and detail['triangles'][:92]==native['triangles'],'Shell coordinates/topology differ after original glass fan')
    # The opaque vessel is closed. All internal decorative points are separately
    # verified inside its convex support; no occupant or internal mechanism is
    # given independent collision or simulation authority.
    tensor=engineering['inertiaAboutCentreOfMassKgM2']; palette=[]
    matrix=[tensor[i:i+3] for i in (0,3,6)]
    corrections=[]
    for i,j in ((0,1),(0,2),(1,2)):
        a,b=matrix[i][j],matrix[j][i]
        if a!=b:
            require(abs(a-b)<1e-12,'Original inertia asymmetry exceeds round-off representation')
            if normalize:
                mean=math.fsum((a,b))/2
                corrections.append({'indices':[[i,j],[j,i]],'sourceValues':[a,b],'symmetricMean':mean,
                                    'changesKgM2':[mean-a,mean-b]})
                matrix[i][j]=matrix[j][i]=mean
    for material in source['design']['materials']:
        opacity=material.get('opticalOpacity')
        alpha=round(255*opacity) if opacity is not None else material['colour']&255
        palette.append({'name':material['name'],'rgb':[(material['colour']>>i)&255 for i in (24,16,8)],
                        'alpha':alpha,'emissive':material['emissive']})
    return {'schema':1,'id':'SH040','name':'Newt Escape Pod','metresPerUnit':float(Fraction(60,19)),
            'roundingMetres':.02,'detail':detail,'native':native,'materials':palette,'faceMaterials':materials,
            'engineering':{'massKg':engineering['dryMassKilograms'],'centerOfMass':engineering['centreOfMassMetres'],
                           'inertia':matrix},
            'flight':{'maxSpeed':source['game']['speed']},'clearances':OPEN_ROUTES,
            'provenance':{'sourceId':'SH040','revision':'a11fe16924d87d06021816f9037952afcbac08db',
                          'tree':'68fec574df12ddf74acb14747e0dd6839f478642','sourceHashes':PINS,
                          'sourceGeometrySHA256':SOURCE_SHA,'sourceGeometryPath':SOURCE_PATH,
                          'license':license_text,'originalMetresPerSourceUnitExact':'60/19','objectKind':'capsule',
                          'purchasePolicy':'never','originalGlassOpticalOpacity':.25,
                          'inertiaRepresentation':{'mode':'round-off symmetry normalization' if normalize else 'exact archived tensor',
                                                   'corrections':corrections},
                          'scope':'Closed cryostasis vessel; exact native shell, one triangulated visible glass quad, no acquisition or actual occupant authority'}}


def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('--check',action='store_true'); args=parser.parse_args()
    for path,normalized in [(RAW_TENSOR_OUTPUT,False),(OUTPUT,True)]:
        encoded=(json.dumps(definition(normalize=normalized),allow_nan=False,separators=(',',':'))+'\n').encode()
        if args.check: require(path.read_bytes()==encoded,'Changed SH040 Studio package')
        else: path.write_bytes(encoded)


if __name__=='__main__': main()
