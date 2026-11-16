"""Execute one reviewed, compiled Rust whole-five harness and seal its output.

The compiler owner supplies the exact source manifest and compiled binary.
This host wrapper uses filesystem/monotonic time; the geometry library does not.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x', encoding='utf-8') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def verify_source(project, manifest):
    files = manifest['files']
    head = manifest.get('head', manifest.get('sourceCommit', manifest.get('commit')))
    tree = manifest.get('tree', manifest.get('sourceTree'))
    for value in (head, tree):
        if not isinstance(value, str) or len(value) != 40 or any(c not in '0123456789abcdef' for c in value):
            raise ValueError('Need exact source commit and tree')
    required = ['Cargo.toml', 'Cargo.lock',
                'crates/meridian-geometry/Cargo.toml',
                'crates/meridian-geometry/src/bin/original_five.rs',
                'crates/meridian-geometry/tools/run_original_five.py']
    required += ['crates/meridian-geometry/src/' + name + '.rs' for name in
                 ('lib', 'adapter', 'arrangement', 'budget', 'certificate', 'exact', 'source')]
    required += ['crates/meridian-geometry/fixtures/' + ident + '.facets' for ident in
                 ('SH010', 'SH011', 'SH016', 'SH017', 'SH166')]
    required += ['crates/meridian-geometry/fixtures/ORIGINAL-BINDINGS.json',
                 'crates/meridian-geometry/integration/WHOLE-FIVE-CRITERIA.json']
    if any(name not in files for name in required):
        raise ValueError('Source binding misses harness/library/input or workspace files')
    for name, pin in files.items():
        relative = Path(name)
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError('Unsafe source binding path')
        path = project / relative
        if path.is_symlink() or not path.resolve(strict=True).is_relative_to(project):
            raise ValueError('Source path escapes frozen workspace')
        expected = pin['sha256'] if isinstance(pin, dict) else pin
        if digest(path) != expected:
            raise ValueError('Source binding mismatch: ' + name)
    helper = project / 'crates/meridian-geometry/tools/run_original_five.py'
    if helper.resolve() != Path(__file__).resolve():
        raise ValueError('Execute the helper from the verified frozen source')
    return head, tree


def validate_terminal(terminal, code, exports):
    names = ('bound_original_decode', 'original_construction', 'source_and_affine_fields',
             'complete_facet_certificates', 'closed_topology_and_limits')
    ids = ('SH010', 'SH011', 'SH016', 'SH017', 'SH166')
    if terminal['declaredBodies'] != 25 or [row['id'] for row in terminal['cases']] != list(ids):
        raise ValueError('Rust terminal changed declared cases or body count')
    counts = dict.fromkeys(('PASS', 'REFUSED', 'FAIL', 'UNRUN'), 0)
    for row in terminal['cases']:
        if [body['name'] for body in row['bodies']] != list(names):
            raise ValueError('Rust terminal changed per-case criteria')
        statuses = [body['status'] for body in row['bodies']]
        for body in row['bodies']:
            if body['status'] not in counts or body['sourceAsset'] != row['id']:
                raise ValueError('Rust terminal body identity or status mismatch')
            if body['sourceInputSHA256'] != exports[row['id']]:
                raise ValueError('Rust terminal body changed bound original input')
            counts[body['status']] += 1
            for key in ('chargedOperationDelta', 'peakStorageAtBodyTerminal'):
                if (body[key] is None) != (body['status'] == 'UNRUN'):
                    raise ValueError('Rust terminal reports measurements for an unrun body')
        for index, status in enumerate(statuses):
            if status != 'PASS' and any(value != 'UNRUN' for value in statuses[index+1:]):
                raise ValueError('Rust terminal ran checks after a failed dependency')
        if row['completeCandidateReturned'] != (statuses[1] == 'PASS'):
            raise ValueError('Rust terminal candidate disagrees with construction')
        if row['allCandidateChecksPassed'] != all(value == 'PASS' for value in statuses):
            raise ValueError('Rust terminal candidate-check summary mismatch')
        if row['fleetAdmission']:
            raise ValueError('Rust terminal grants fleet admission')
    for status, key in [('PASS', 'bodiesPass'), ('REFUSED', 'bodiesRefused'),
                        ('FAIL', 'bodiesFail'), ('UNRUN', 'bodiesUnrun')]:
        if terminal[key] != counts[status]:
            raise ValueError('Rust terminal aggregate count mismatch')
    if (code == 0) != (counts['PASS'] == 25) or code not in (0, 1):
        raise ValueError('Rust process status disagrees with case outcomes')
    if terminal['all171Accepted'] or terminal['nativeAcceptance']:
        raise ValueError('Rust terminal grants broader acceptance')


def execute(project, source_binding, binary, output):
    project = project.resolve(strict=True)
    binary = binary.resolve(strict=True)
    source_binding = source_binding.resolve(strict=True)
    manifest = json.loads(source_binding.read_bytes())
    head, tree = verify_source(project, manifest)
    if output.exists() or not output.is_absolute() or output != output.resolve():
        raise ValueError('Need a new absolute physical output directory')
    output.mkdir(mode=0o700)
    source_hash = digest(source_binding)
    binary_hash = digest(binary)
    binding = {'sourceHead': head, 'sourceTree': tree, 'sourceManifestSHA256': source_hash,
               'binary': str(binary), 'binarySHA256': binary_hash,
               'helperSHA256': digest(Path(__file__).resolve()),
               'sourceFiles': manifest['files'],
               'declaredBodies': 25, 'originalCaseCount': 5,
               'limits': {'operations': 2000000, 'storageMemberships': 100000,
                          'rationalBits': 4096, 'candidateTriangles': 512},
               'scope': 'One changed Rust original-five construction and dependent candidate checks',
               'fleetBaseline': {'ships': 23, 'shipTotal': 162, 'stations': 4, 'stationTotal': 6}}
    binding_file = output / 'EXECUTION-BINDING.json'
    save(binding_file, binding)
    results = output / 'results'
    command = [str(binary), '--out', str(results), '--binding', str(binding_file)]
    started = time.monotonic_ns()
    with (output / 'stdout.log').open('xb') as stdout, (output / 'stderr.log').open('xb') as stderr:
        process = subprocess.Popen(command, cwd=project, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
        start = {'event': 'ACTUAL_RUST_ORIGINAL_FIVE_START', 'pid': process.pid, 'parentPID': os.getpid(),
                 'argv': command, 'monotonicStartedNanoseconds': started,
                 'sourceHead': head, 'sourceTree': tree, 'sourceManifestSHA256': source_hash,
                 'binarySHA256': binary_hash, 'executionBindingSHA256': digest(binding_file),
                 'out': str(output)}
        save(output / 'ACTUAL-START.json', start)
        print(json.dumps(start), flush=True)
        code = process.wait()
    elapsed = time.monotonic_ns() - started
    terminal_file = results / 'TERMINAL.json'
    terminal = None
    postflight_error = None
    try:
        terminal = json.loads(terminal_file.read_bytes())
        exports = {row['id']: row['exportSHA256'] for row in json.loads(
            (project / 'crates/meridian-geometry/fixtures/ORIGINAL-BINDINGS.json').read_bytes())}
        validate_terminal(terminal, code, exports)
        if digest(results / 'EXECUTION-BINDING.json') != digest(binding_file):
            raise ValueError('Rust result did not retain the exact execution binding')
        unchanged = digest(binary) == binary_hash and digest(source_binding) == source_hash
        verify_source(project, manifest)
    except (OSError, ValueError, KeyError, TypeError) as error:
        unchanged = False
        postflight_error = str(error)
    owner_exit = code if unchanged else 2
    record = {'exitCode': owner_exit, 'rustExitCode': code,
              'monotonicElapsedNanoseconds': elapsed,
              'inputsAndBinaryUnchanged': unchanged, 'postflightError': postflight_error,
              'actualRustTerminal': terminal, 'sourceHead': head, 'sourceTree': tree,
              'noRetry': True, 'all171Accepted': False, 'nativeAcceptance': False}
    save(output / 'OWNER-TERMINAL.json', record)
    artifacts = {str(path.relative_to(output)): {'sha256': digest(path), 'bytes': path.stat().st_size}
                 for path in sorted(output.rglob('*')) if path.is_file()}
    save(output / 'ARTIFACTS.json', {'files': artifacts, 'sourceHead': head, 'sourceTree': tree})
    print(json.dumps({'event': 'ACTUAL_RUST_ORIGINAL_FIVE_TERMINAL', 'exitCode': owner_exit,
                      'terminalSHA256': digest(output / 'OWNER-TERMINAL.json'),
                      'artifactManifestSHA256': digest(output / 'ARTIFACTS.json')}), flush=True)
    return owner_exit


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--source-binding', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    sys.exit(execute(args.project, args.source_binding, args.binary, args.out))
