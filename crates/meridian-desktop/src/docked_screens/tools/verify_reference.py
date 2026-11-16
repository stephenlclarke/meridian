"""Verify exact immutable source/data bindings without compiling or running the game."""
import argparse
import hashlib
import json
import math
import re
import subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def verify(reference):
    manifest=json.loads((ROOT/'SOURCE.json').read_text())
    assert manifest['revision']=='a11fe16924d87d06021816f9037952afcbac08db'
    for item in manifest['sources']:
        raw=subprocess.check_output(['git','show',manifest['revision']+':'+item['path']],cwd=reference)
        assert len(raw)==item['bytes'] and hashlib.sha256(raw).hexdigest()==item['sha256'],item['path']
    for item in manifest['outputs']:
        raw=(ROOT/item['path']).read_bytes();assert len(raw)==item['bytes'] and hashlib.sha256(raw).hexdigest()==item['sha256'],item['path']
    worlds=[line.split('\t') for line in (ROOT/'data/worlds.tsv').read_text().splitlines()]
    objects=[line.split('\t') for line in (ROOT/'data/objects.tsv').read_text().splitlines()]
    specs=[line.split('\t') for line in (ROOT/'data/specifications.tsv').read_text().splitlines()]
    assert len(worlds)==2048 and all(len(row)==16 and (int(row[0]),int(row[1]))==divmod(i,256) for i,row in enumerate(worlds))
    assert all(math.isfinite(float(row[i])) for row in worlds for i in [3,4,10,11])
    assert len(objects)==171 and sum(row[3]=='ship' for row in objects)==162
    assert all(len(row)==8 and row[0]==f'SH{i+1:03}' for i,row in enumerate(objects))
    assert all(len(row)==4 and row[0] in {o[0] for o in objects} for row in specs)
    tests=re.findall(r'#\[test\]\s*fn\s+(\w+)',(ROOT/'tests.rs').read_text())
    integration=json.loads((ROOT/'INTEGRATION.json').read_text());assert tests==integration['authoredTests']
    for target in re.findall(r'\]\(([^)]+)\)',(ROOT/'README.md').read_text()):
        if '://' not in target:assert (ROOT/target).is_file(),target
    return {'status':'SOURCE_ONLY_PASS;DOES_NOT_EXECUTE_RUST_TESTS','immutableSourceFiles':len(manifest['sources']),'worlds':len(worlds),'ships':162,'otherObjects':9,'specificationFields':len(specs),'authoredRustTests':len(tests),'negativeOriginalCoordinatesPreserved':any(float(w[3])<0 or float(w[4])<0 for w in worlds)}

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', required=True, type=Path)
    args = parser.parse_args(argv)
    print(json.dumps(verify(args.reference)))


if __name__ == '__main__':
    main()
