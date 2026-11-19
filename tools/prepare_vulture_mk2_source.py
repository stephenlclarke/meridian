"""Authenticate the exact archival SH038 inputs and reproduce its refused Studio package."""

import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

from import_goshawk_envelope_source import components


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh038'
OUTPUT = ROOT / 'integration/sh038-vulture-mk2/ORIGINAL-ARCHIVE-CANDIDATE.json'
PINS = {
    'resource.toml': '0f803ffa0cd5621773476f35e99e89ed1e9f9e8513899e40fbfa8b0621182eea',
    'native-export.json': '51ccec9fa500f39e5fe3c6ae1e8429bbddc13977cd1f7d15b47980ea400bedaf',
    'contract.toml': '55d9bb7a617e090e120ae639b19a6eb1f8fa91d220928bf58a498193c425ffd6',
    'engineering.json': '05e296d97b156545bb571b8271cf0fe5207da911cba66b8cea809cc7b1d44446',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
SOURCE_SHA = '41e9704f9c177717f42dabc4290bbe67b6792b9916a837dfaf9ef63d820191d4'
SOURCE_PATH = 'replacement-hulls/SH038-design.toml'
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def authenticated(folder):
    data = {name: (folder/name).read_bytes() for name in PINS}
    for name, raw in data.items():
        require(hashlib.sha256(raw).hexdigest() == PINS[name], f'Changed SH038 source {name}')
    resource = tomllib.loads(data['resource.toml'].decode())
    contract = tomllib.loads(data['contract.toml'].decode())
    archived = json.loads(data['native-export.json'])
    engineering = json.loads(data['engineering.json'])
    require(resource['id'] == contract['id'] == archived['id'] == engineering['id'] == 'SH038', 'Wrong SH038 identity')
    require(resource['name'] == contract['name'] == 'Vulture Mk II', 'Wrong SH038 variant')
    require(archived['sourcePath'] == contract['source']['geometryFile'] == SOURCE_PATH and
            archived['sourceSHA256'] == contract['source']['geometrySHA256'] == engineering['detailedSourceSHA256'] == SOURCE_SHA,
            'Wrong source mesh identity')
    require(archived['nativeDesign'] == resource['nativeDesign'], 'Native mesh differs from archived original')
    require(engineering['contractSHA256'] == PINS['contract.toml'] and engineering['metresPerSourceUnitExact'] == '6000/611',
            'Wrong original authored engineering reference')
    require(contract['geometry']['nativeFaceLimit'] == 512, 'Wrong native face policy')
    for key, expected in [('design',(5132,8272,501)), ('nativeDesign',(1812,3016,155))]:
        model = resource[key]
        require((len(model['vertices']),len(model['faces']),len(components(model))) == expected, f'Wrong {key} topology')
        require(model['license'] == model['appearanceLicense'] == 'MIT' and model['paletteName'] == 'Copper / graphite / sand',
                'Wrong original appearance')
    return resource, engineering, data['LICENSE'].decode()


def mesh(model, first_group):
    groups = [None]*len(model['vertices'])
    for i, group in enumerate(components(model), first_group):
        for index in group:
            groups[index] = i
    require(all(group is not None for group in groups), 'Unassigned original points')
    require(all(len({groups[i] for i in face}) == 1 for face in model['faces']), 'Crossed original component')
    return {'vertices': model['vertices'], 'triangles': model['faces'], 'groups': groups}


def definition(folder=ASSETS):
    resource, engineering, license_text = authenticated(folder)
    detail, native = resource['design'], resource['nativeDesign']
    tensor = engineering['inertiaAboutCentreOfMassKgM2']
    return {'schema': 1, 'id': 'SH038', 'name': 'Vulture Mk II',
            'metresPerUnit': float(Fraction(engineering['metresPerSourceUnitExact'])), 'roundingMetres': .02,
            'detail': mesh(detail,0), 'native': mesh(native,501),
            'materials': [{'name': m['name'], 'rgb': [(m['colour']>>i)&255 for i in (24,16,8)],
                           'alpha': m['colour']&255, 'emissive':m['emissive']} for m in detail['materials']],
            'faceMaterials': detail['faceMaterials'],
            'engineering': {'massKg': engineering['dryMassKilograms'], 'centerOfMass': engineering['centreOfMassMetres'],
                            'inertia': [tensor[i:i+3] for i in (0,3,6)]},
            'flight': {'maxSpeed': resource['game']['speed']}, 'clearances': [],
            'provenance': {'sourceId':'SH038','revision':REVISION,'sourceHashes':PINS,
                           'archiveModelSHA256':SOURCE_SHA,'sourceUnits':'relative original',
                           'metresPerSourceUnitExact':'6000/611','nativeFacePolicy':512,
                           'archivalCandidateOnly':True, 'license':license_text}}


def refusal(validator, package):
    with tempfile.TemporaryDirectory(prefix='meridian-sh038-') as temp:
        folder = Path(temp)
        path, report = folder/'package.json', folder/'report.json'
        path.write_bytes(package)
        result = subprocess.run([str(validator.resolve()),'validate',str(path),'--report',str(report)],
                                capture_output=True, check=False, timeout=180)
        require(report.is_file(), f'Missing real validator report: {result.stderr!r}')
        data = json.loads(report.read_bytes())
        require(result.returncode != 0 and data['valid'] is False and
                any('native' in row['message'].lower() and '512' in row['message'] for row in data['diagnostics']),
                f'Expected original native contract refusal: {data}')
        with validator.open('rb') as stream:
            executable = hashlib.file_digest(stream,'sha256').hexdigest()
        return {'validatorSHA256':executable,'packageSHA256':hashlib.sha256(package).hexdigest(),
                'exitCode':result.returncode,'report':data}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    parser.add_argument('--validator',type=Path)
    args = parser.parse_args()
    package = (json.dumps(definition(),allow_nan=False,separators=(',',':'))+'\n').encode()
    if args.check:
        require(OUTPUT.read_bytes()==package,'Changed archival SH038 candidate')
    else:
        OUTPUT.write_bytes(package)
    if args.validator:
        print(json.dumps(refusal(args.validator,package),indent=2))


if __name__ == '__main__':
    main()
