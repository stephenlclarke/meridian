"""Qualify the actual generic Studio export and its positive/negative SH036 probes."""
import argparse
import json
from pathlib import Path
import subprocess

from prepare_zebu_studio import INTEGRATION, encode_json, pin, require


def compare_reports(folder, source, witnesses):
    exported = (folder / 'ZEBU-EXPORTED.hull.json').read_bytes()
    require(exported == source.rstrip(b'\n'), 'Export modified original hull content')
    positive = json.loads((folder / 'EXPORT-REPORT.json').read_bytes())
    negative = json.loads((folder / 'BLOCKED-REPORT.json').read_bytes())
    openings = [r['route']['name'] for r in witnesses['clearances']]
    contacts = [r['route']['name'] for r in witnesses['contacts']]
    definition = json.loads(source)
    expected_negative = {**definition, 'clearances': definition['clearances'] + [row['route'] for row in witnesses['contacts']]}
    require(json.loads((folder / 'BLOCKED-PROBES.json').read_bytes()) == expected_negative, 'Negative input changed source geometry or routes')
    require(positive['valid'] and not positive['diagnostics'] and len(positive['supportGroups']) == 24, 'Original Zebu export was not valid')
    groups = {}
    for key in ('detail', 'native'):
        mesh = definition[key]
        for point, group in zip(mesh['vertices'], mesh['groups'], strict=True):
            groups.setdefault(group, []).append(point)
    require(positive['supportGroups'] == [{'id': i, 'points': groups[i], 'roundingMetres': definition['roundingMetres']} for i in sorted(groups)], 'Report support points differ from exact authored source')
    require(not negative['valid'] and negative['supportGroups'] == positive['supportGroups'], 'Negative route changed or accepted the envelope')
    require(positive['clearances'] == [{'name': name, 'passed': True} for name in openings], 'Changed expected clear openings')
    require(negative['clearances'] == positive['clearances'] + [{'name': name, 'passed': False} for name in contacts], 'Original contact routes were not specifically refused')
    require(negative['diagnostics'] == [{'message': f'Clearance {name} is blocked or indeterminate'} for name in contacts], 'Unexpected negative diagnostics')
    require(not (folder / 'EXPORT-STDERR.txt').read_bytes(), 'Unexpected export stderr')
    return {name: pin((folder / name).read_bytes()) for name in ('ZEBU-EXPORTED.hull.json', 'EXPORT-REPORT.json', 'BLOCKED-PROBES.json', 'BLOCKED-REPORT.json')}


def execute(studio, validator, folder, source, witnesses):
    folder.mkdir(parents=True, exist_ok=False)
    definition = json.loads(source)
    paths = definition['clearances'] + [row['route'] for row in witnesses['contacts']]
    negative = {**definition, 'clearances': paths}
    (folder / 'BLOCKED-PROBES.json').write_bytes(encode_json(negative))
    with (folder / 'EXPORT-REPORT.json').open('wb') as stdout, (folder / 'EXPORT-STDERR.txt').open('wb') as stderr:
        result = subprocess.run([str(studio), '--definition', str(INTEGRATION / 'ZEBU-STUDIO.json'), '--export', str(folder / 'ZEBU-EXPORTED.hull.json'), '--no-browser'], stdout=stdout, stderr=stderr, check=False)
    require(result.returncode == 0, f'Studio export failed with exit {result.returncode}')
    with (folder / 'BLOCKED-STDOUT.txt').open('wb') as stdout, (folder / 'BLOCKED-STDERR.txt').open('wb') as stderr:
        result = subprocess.run([str(validator), 'validate', str(folder / 'BLOCKED-PROBES.json'), '--report', str(folder / 'BLOCKED-REPORT.json')], stdout=stdout, stderr=stderr, check=False)
    require(result.returncode == 1, f'Contact route expected validation refusal, exit {result.returncode}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--studio', type=Path, help='Installed or built Studio executable')
    parser.add_argument('--validator', type=Path, help='Exact bundled meridian-hull-tool executable')
    parser.add_argument('--output', required=True, type=Path, help='New evidence directory, or existing directory with --check')
    parser.add_argument('--check', action='store_true', help='Verify existing raw evidence without running either binary')
    args = parser.parse_args()
    source = (INTEGRATION / 'ZEBU-STUDIO.json').read_bytes()
    witnesses = json.loads((INTEGRATION / 'CONTACT-WITNESSES.json').read_bytes())
    if not args.check:
        if args.studio is None or args.validator is None:
            parser.error('--studio and --validator are required to execute evidence')
        execute(args.studio.resolve(), args.validator.resolve(), args.output.resolve(), source, witnesses)
    print(json.dumps(compare_reports(args.output, source, witnesses), indent=2))


if __name__ == '__main__':
    main()
