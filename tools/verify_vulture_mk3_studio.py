"""Replay the original refusal and the separate candidate export/solid controls."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import sys

from prepare_vulture_mk3_studio import ASSETS, DETAIL, INTEGRATION, encode, pin, require


def inputs():
    original=(INTEGRATION/'VULTURE-MK3-ORIGINAL-STUDIO.json').read_bytes()
    candidate=(INTEGRATION/'VULTURE-MK3-CANDIDATE-STUDIO.json').read_bytes()
    witness=json.loads((INTEGRATION/'CONTACT-WITNESSES.json').read_bytes())
    value=json.loads(candidate)
    negative={**value,'clearances':value['clearances']+[row['route'] for row in witness['contacts']]}
    return original,candidate,negative,witness


def compare(folder):
    _,source,negative,witness=inputs()
    package=json.loads(source)
    require((folder/'CANDIDATE-EXPORTED.hull.json').read_bytes()==source.rstrip(b'\n'),'Candidate export changed the authored source')
    require(json.loads((folder/'CANDIDATE-CONTACT-PROBES.json').read_bytes())==negative,'Changed negative source input')
    original,positive,blocked=(json.loads((folder/name).read_bytes()) for name in ('ORIGINAL-REFUSAL.json','CANDIDATE-EXPORT-REPORT.json','CANDIDATE-CONTACT-REPORT.json'))
    require(not original['valid'] and not original['supportGroups'] and original['diagnostics']==[{'message':'native: invalid vertex, triangle or group cardinality (face limit 512)'}],'Original cap refusal changed')
    require(positive['valid'] and not positive['diagnostics'] and len(positive['supportGroups'])==169,'Candidate invalid')
    expected={}
    for key in ('detail','native'):
        mesh=package[key]
        for point,group in zip(mesh['vertices'],mesh['groups'],strict=True):expected.setdefault(group,[]).append([v*package['metresPerUnit'] for v in point])
    require(positive['supportGroups']==[{'id':g,'points':expected[g],'roundingMetres':package['roundingMetres']} for g in sorted(expected)],'Installed scale or support points differ')
    calibrated=list(struct.iter_unpack('<3d',(ASSETS/'source.vertices').read_bytes()[16:]))
    derived=json.loads((ASSETS/'DERIVED-NATIVE.json').read_bytes())
    exact=calibrated[:DETAIL[0]]+[calibrated[DETAIL[0]+i] for i in derived['sourceVertexIndices']]
    source_coordinates=package['detail']['vertices']+package['native']['vertices']
    delta=max(abs(a-b*package['metresPerUnit']) for point,raw in zip(exact,source_coordinates,strict=True) for a,b in zip(point,raw,strict=True))
    require(delta<1e-12,'Installed binary64 scale differs from exact original ratio')
    openings=[{'name':row['route']['name'],'passed':True} for row in witness['openings']]
    contacts=[{'name':row['route']['name'],'passed':False} for row in witness['contacts']]
    require(positive['clearances']==openings and not blocked['valid'] and blocked['supportGroups']==positive['supportGroups'] and blocked['clearances']==openings+contacts,'Changed opening/solid control outcomes')
    require(blocked['diagnostics']==[{'message':f'Clearance {row["name"]} is blocked or indeterminate'} for row in contacts],'Changed contact diagnostics')
    require(not (folder/'CANDIDATE-STDERR.txt').read_bytes(),'Unexpected candidate export stderr')
    files=('ORIGINAL-REFUSAL.json','CANDIDATE-EXPORTED.hull.json','CANDIDATE-EXPORT-REPORT.json','CANDIDATE-CONTACT-PROBES.json','CANDIDATE-CONTACT-REPORT.json')
    return {'actualSourcePointCount':sum(map(len,expected.values())),'maximumMetreDifferenceFromExactRationalCalibration':delta,'files':{name:pin((folder/name).read_bytes()) for name in files}}


def execute(studio,validator,folder):
    folder.mkdir(parents=True,exist_ok=False)
    _,_,negative,_=inputs()
    (folder/'CANDIDATE-CONTACT-PROBES.json').write_bytes(encode(negative))
    jobs=[('ORIGINAL', [str(validator),'validate',str(INTEGRATION/'VULTURE-MK3-ORIGINAL-STUDIO.json'),'--report',str(folder/'ORIGINAL-REFUSAL.json')],1,'ORIGINAL-STDOUT.txt'),('CANDIDATE',[str(studio),'--definition',str(INTEGRATION/'VULTURE-MK3-CANDIDATE-STUDIO.json'),'--export',str(folder/'CANDIDATE-EXPORTED.hull.json'),'--no-browser'],0,'CANDIDATE-EXPORT-REPORT.json'),('CONTACT',[str(validator),'validate',str(folder/'CANDIDATE-CONTACT-PROBES.json'),'--report',str(folder/'CANDIDATE-CONTACT-REPORT.json')],1,'CONTACT-STDOUT.txt')]
    for name,args,expected,output in jobs:
        with (folder/output).open('wb') as stdout,(folder/f'{name}-STDERR.txt').open('wb') as stderr:
            result=subprocess.run(args,stdout=stdout,stderr=stderr,check=False)
        require(result.returncode==expected,f'{name} expected {expected}, exit {result.returncode}')


def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('--studio',type=Path); parser.add_argument('--validator',type=Path); parser.add_argument('--output',type=Path,required=True); parser.add_argument('--check',action='store_true'); args=parser.parse_args()
    if not args.check:
        if not args.studio or not args.validator:parser.error('Studio and validator executables required')
        execute(args.studio.resolve(),args.validator.resolve(),args.output.resolve())
    sys.stdout.buffer.write((json.dumps(compare(args.output),indent=2)+'\n').encode('utf-8'))


if __name__=='__main__':main()
