"""Verify frozen Lua inputs/provenance without compiling or executing a game/interpreter."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CORPUS_SHA = '3da46b069b5175fe5c5e4a9831fdac391c4de6b4bffe4259132c99800c9929d9'
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'


def sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def read_json(path: Path):
    return json.loads(path.read_bytes())


def contained(root: Path, name: str) -> Path:
    path = (root / name).resolve(strict=True)
    if not path.is_relative_to(root.resolve()) or not path.is_file():
        raise ValueError('Non-local artifact path')
    return path


def verify(reference: Path | None) -> dict:
    corpus = ROOT / 'tests/compat'
    if sha((corpus / 'MANIFEST.json').read_bytes()) != CORPUS_SHA:
        raise ValueError('Frozen G2 manifest changed')
    payloads = read_json(corpus / 'MANIFEST.json')['payloads']
    for item in payloads:
        raw = contained(corpus, item['path']).read_bytes()
        if (len(raw), sha(raw)) != (item['bytes'], item['sha256']):
            raise ValueError('Frozen G2 payload changed: ' + item['path'])
    cases = read_json(corpus / 'CASES.json')
    generated = (ROOT / 'tests/support/compat_cases.rs').read_text()
    if len(cases) != 139 or any(('run("' + row['id'] + '")') not in generated for row in cases):
        raise ValueError('Missing frozen named invocation')
    prelude = (ROOT / 'src/module_prelude.lua').read_bytes()
    compiled = (corpus / 'lua/modules-closed-cache.lua').read_bytes()
    if compiled[:compiled.index(b'\n-- ')] != prelude:
        raise ValueError('Original closed-module prelude differs from independent fixture')
    notices = read_json(ROOT / 'notices/DEPENDENCIES.json')['dependencies']
    for dependency in notices:
        for notice in dependency['notices']:
            if sha(contained(ROOT, notice['file']).read_bytes()) != notice['sha256']:
                raise ValueError('Retained dependency notice mismatch')
    vendor = read_json(ROOT / 'VENDOR.json')
    if sha(contained(ROOT, vendor['patch']['path']).read_bytes()) != vendor['patch']['sha256']:
        raise ValueError('Dependency patch mismatch')
    for dependency in vendor['dependencies']:
        for item in dependency['changes']:
            raw = contained(ROOT / 'vendor' / dependency['package'], item['path']).read_bytes()
            if sha(raw) != item['retainedSHA256']:
                raise ValueError('Patched upstream source changed')
    for path in (ROOT / 'src').rglob('*.rs'):
        if re.search(r'\bunsafe\s*(?:\{|fn\b|impl\b|extern\b)', path.read_text()):
            raise ValueError('Direct unsafe code in primary runtime crate')
    for path in ROOT.rglob('*.json'):
        read_json(path)
    for path in ROOT.rglob('*.toml'):
        tomllib.loads(path.read_text())
    source = read_json(ROOT / 'SOURCE.json')
    checked = 0
    if reference is not None:
        directory = reference.resolve(strict=True)
        if not directory.is_dir():
            raise ValueError('Reference object database must be a directory')
        for item in source['references']:
            if not re.fullmatch(r'Sources/[A-Za-z0-9_./-]+', item['path']) or '..' in item['path'].split('/'):
                raise ValueError('Non-selected source path')
            raw = subprocess.check_output(
                ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', REVISION + ':' + item['path']],
                cwd=directory,
            )
            if (len(raw), sha(raw)) != (item['bytes'], item['sha256']):
                raise ValueError('Pinned original source changed')
            checked += 1
    return {'status': 'SOURCE_ARTIFACT_CHECK_PASS_ONLY', 'corpusPayloads': len(payloads),
            'frozenInvocationsAuthored': len(cases), 'noticePackages': len(notices),
            'originalSourcesChecked': checked, 'modulePreludeBytes': len(prelude),
            'nativeCompilerTestsClippyCoverage': 'UNRUN; existing owner execution required'}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    result = json.dumps(verify(args.reference), indent=2) + '\n'
    if args.report is not None:
        args.report.write_text(result)
    print(result, end='')


if __name__ == '__main__':
    main()
