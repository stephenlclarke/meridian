"""Run or independently compare exact installed SH041 export and solid controls."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

from prepare_serval_studio import INTEGRATION, encode, pin, require


def inputs():
    raw=(INTEGRATION/'SERVAL-STUDIO.json').read_bytes(); definition=json.loads(raw); witnesses=json.loads((INTEGRATION/'CONTACT-WITNESSES.json').read_bytes())
    negative={**definition,'clearances':definition['clearances']+[row['route'] for row in witnesses['contacts']]}
    return raw,definition,negative,witnesses


def compare(folder):
    raw,source,negative,witness=inputs()
    require((folder/'SERVAL-EXPORTED.hull.json').read_bytes()==raw.rstrip(b'\n'),'Export modified original source')
    require(json.loads((folder/'BLOCKED-PROBES.json').read_bytes())==negative,'Negative input changed original source')
    positive,blocked=(json.loads((folder/name).read_bytes()) for name in ('EXPORT-REPORT.json','BLOCKED-REPORT.json'))
    groups={}
    for key in ('detail','native'):
        mesh=source[key]
        for point,g in zip(mesh['vertices'],mesh['groups'],strict=True):groups.setdefault(g,[]).append(point)
    support=[{'id':g,'points':groups[g],'roundingMetres':.02} for g in sorted(groups)]
    openings=[{'name':row['route']['name'],'passed':True} for row in witness['openings']]; contacts=[{'name':row['route']['name'],'passed':False} for row in witness['contacts']]
    require(positive['valid'] and not positive['diagnostics'] and len(support)==23 and positive['supportGroups']==support,'Original Studio support points differ or validation failed')
    require(positive['clearances']==openings and not blocked['valid'] and blocked['supportGroups']==support and blocked['clearances']==openings+contacts,'Opening/solid controls changed')
    require(blocked['diagnostics']==[{'message':f'Clearance {row["name"]} is blocked or indeterminate'} for row in contacts],'Unexpected blocked diagnostics')
    require(not (folder/'EXPORT-STDERR.txt').read_bytes(),'Unexpected export stderr')
    return {name:pin((folder/name).read_bytes()) for name in ('SERVAL-EXPORTED.hull.json','EXPORT-REPORT.json','BLOCKED-PROBES.json','BLOCKED-REPORT.json')}


def execute(studio,validator,folder):
    folder.mkdir(parents=True,exist_ok=False); _,_,negative,_=inputs(); (folder/'BLOCKED-PROBES.json').write_bytes(encode(negative))
    jobs=[([str(studio),'--definition',str(INTEGRATION/'SERVAL-STUDIO.json'),'--export',str(folder/'SERVAL-EXPORTED.hull.json'),'--no-browser'],0,'EXPORT-REPORT.json','EXPORT-STDERR.txt'),([str(validator),'validate',str(folder/'BLOCKED-PROBES.json'),'--report',str(folder/'BLOCKED-REPORT.json')],1,'BLOCKED-STDOUT.txt','BLOCKED-STDERR.txt')]
    for args,expected,output,error in jobs:
        with (folder/output).open('wb') as stdout,(folder/error).open('wb') as stderr:
            run=subprocess.run(args,stdout=stdout,stderr=stderr,check=False)
        require(run.returncode==expected,f'Expected {expected}, received {run.returncode}')


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--studio',type=Path);parser.add_argument('--validator',type=Path);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    if not args.check:
        if not args.studio or not args.validator:parser.error('Studio and validator required')
        execute(args.studio.resolve(),args.validator.resolve(),args.output.resolve())
    sys.stdout.buffer.write((json.dumps(compare(args.output),indent=2)+'\n').encode('utf-8'))


if __name__=='__main__':main()
