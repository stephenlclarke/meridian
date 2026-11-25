"""Replay pinned original and candidate hull numerical receipts through their CLIs."""

from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = {
    'sh044-harpy-assault', 'sh045-erne-corvette', 'sh046-harpy-dropship', 'sh047-harpy-gunship',
}
MODES = (((), 'SURFACE-FIT.json'), (('--difference',), 'ORIGINAL-DIFFERENCE.json'))
GEOMETRY_REQUIRED = {'sh048-lanner', 'sh049-hyena', 'sh050-yak'}
GEOMETRY_MODES = (
    (('archive-fit',), 'ARCHIVE-FIT.json'),
    (('candidate-fit',), 'candidate-fit.json'),
    (('candidate-fidelity',), 'candidate-fidelity.json'),
)
MAX_REPLAY_WORKERS = 4


def receipt_command(folder, script, flags, receipt):
    return ' '.join(('python3', '-B', script.relative_to(ROOT).as_posix(), *flags, '|', 'cmp', '-',
                     (folder / receipt).relative_to(ROOT).as_posix()))


def measurement_cases(suffix='', modes=MODES):
    # The compact hull convention pairs integration/shNNN-hull-name with
    # tools/measure_hull_name.py. New hulls using that convention join the test
    # automatically; older, separately qualified _air or Goshawk tools do not.
    for folder in sorted((ROOT / 'integration').glob('sh[0-9][0-9][0-9]-*')):
        slug = folder.name.split('-', 1)[1].replace('-', '_')
        script = ROOT / 'tools' / f'measure_{slug}{suffix}.py'
        if script.is_file():
            yield folder, script
            continue
        criteria_path = folder / 'CRITERIA.json'
        if criteria_path.is_file():
            criteria = json.loads(criteria_path.read_bytes())
            declared = criteria.get('byteReplay', {}).get('commands', ())
            if (script.relative_to(ROOT).as_posix() in criteria.get('replayPins', {})
                    or any(receipt_command(folder, script, flags, receipt) in declared for flags, receipt in modes)):
                # The criteria still expect this conventional CLI. Include the
                # missing path so the assertion fails instead of silently skipping it.
                yield folder, script


def active_coverage():
    # Ordinary platform test jobs don't require coverage.py. In the coverage
    # job, its active instance receives the independently measured workers.
    module = sys.modules.get('coverage')
    current = module.Coverage.current() if module is not None else None
    return (module, current) if current is not None else None


def replay(script, flags, coverage_file=None, branch=False):
    command = [sys.executable, '-B']
    if coverage_file is not None:
        command.extend(('-m', 'coverage', 'run', '--parallel-mode',
                        f'--rcfile={ROOT / ".coveragerc"}', f'--data-file={coverage_file}'))
        if branch:
            command.append('--branch')
    command.extend((str(script), *flags))
    result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise RuntimeError(f'{script.name} {flags!r} exited {result.returncode}: '
                           f'{result.stderr.decode(errors="replace")}')
    return result.stdout


def merge_worker_coverage(active, coverage_file, script):
    files = list(coverage_file.parent.glob(coverage_file.name + '.*'))
    if len(files) != 1:
        raise RuntimeError(f'Expected exactly one isolated coverage file for {coverage_file.name}, found {len(files)}')
    module, current = active
    data = module.CoverageData(basename=str(files[0]))
    try:
        data.read()
        measured = {Path(name): name for name in data.measured_files()}
        script_name = measured.get(script.relative_to(ROOT)) or measured.get(script)
        if script_name is None or not data.lines(script_name):
            raise RuntimeError(f'No executed lines for {script.name} in {coverage_file.name}')
        current.get_data().update(data)
    finally:
        data.close()


def worker_count():
    usable = getattr(os, 'process_cpu_count', os.cpu_count)() or 1
    return min(MAX_REPLAY_WORKERS, usable)


class HullMeasurementReplays(unittest.TestCase):
    def assert_replays(self, suffix, required, modes):
        cases = list(measurement_cases(suffix, modes))
        self.assertTrue(required <= {folder.name for folder, _ in cases})
        self.assert_pinned_replays(cases, modes)

    def assert_pinned_replays(self, cases, modes):
        jobs = []
        for folder, script in cases:
            criteria = json.loads((folder / 'CRITERIA.json').read_bytes())
            script_name = script.relative_to(ROOT).as_posix()
            with self.subTest(hull=folder.name, script=script_name):
                self.assertTrue(script.is_file(), f'Missing pinned measurement CLI: {script_name}')
                self.assertEqual(hashlib.sha256(script.read_bytes()).hexdigest(),
                                 criteria['replayPins'][script_name])
                for flags, receipt in modes:
                    path = folder / receipt
                    name = path.relative_to(ROOT).as_posix()
                    command = receipt_command(folder, script, flags, receipt)
                    with self.subTest(hull=folder.name, receipt=receipt):
                        self.assertIn(command, criteria['byteReplay']['commands'])
                        expected = path.read_bytes()
                        self.assertEqual(hashlib.sha256(expected).hexdigest(), criteria['replayPins'][name])
                        jobs.append((folder.name, script, flags, receipt, expected))

        active = active_coverage()
        branch = active[1].get_option('run:branch') if active else False
        with tempfile.TemporaryDirectory(prefix='meridian-replay-coverage-') as scratch:
            with ThreadPoolExecutor(max_workers=worker_count()) as pool:
                scheduled = []
                for index, (hull, script, flags, receipt, expected) in enumerate(jobs):
                    coverage_file = Path(scratch) / f'worker-{index}.coverage' if active else None
                    future = pool.submit(replay, script, flags, coverage_file, branch)
                    scheduled.append((hull, script, receipt, expected, coverage_file, future))
                for hull, script, receipt, expected, coverage_file, future in scheduled:
                    with self.subTest(hull=hull, receipt=receipt):
                        actual = future.result()
                        if active:
                            merge_worker_coverage(active, coverage_file, script)
                        self.assertEqual(actual, expected)

    def test_condor_collision_fit_is_recomputed_byte_for_byte(self):
        case = (ROOT / 'integration/sh055-condor', ROOT / 'tools/measure_condor.py')
        self.assert_pinned_replays([case], ((('--collision',), 'COLLISION-FIT.json'),))

    def test_frozen_fit_and_difference_are_recomputed_byte_for_byte(self):
        self.assert_replays('', REQUIRED, MODES)

    def test_archive_candidate_and_fidelity_are_recomputed_byte_for_byte(self):
        # The three-mode geometry convention qualifies both a frozen archive
        # and a separately measured candidate. New matching hulls join automatically.
        self.assert_replays('_geometry', GEOMETRY_REQUIRED, GEOMETRY_MODES)


class ReplayHarnessTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory(prefix='meridian-replay-fixture-')
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.folder = self.root / 'integration/sh001-example'
        self.folder.mkdir(parents=True)
        self.script = self.root / 'tools/measure_example.py'
        self.script.parent.mkdir()
        self.receipt = self.folder / 'SURFACE-FIT.json'
        self.source = b'print("expected")\n'
        self.expected = b'expected\n'
        self.command = 'python3 -B tools/measure_example.py | cmp - integration/sh001-example/SURFACE-FIT.json'
        self.criteria = {
            'replayPins': {
                'tools/measure_example.py': hashlib.sha256(self.source).hexdigest(),
                'integration/sh001-example/SURFACE-FIT.json': hashlib.sha256(self.expected).hexdigest(),
            },
            'byteReplay': {'commands': [self.command]},
        }

    def probe(self, *, source=None, receipt=None, command=None, output=None, error=None):
        self.script.write_bytes(self.source if source is None else source)
        self.receipt.write_bytes(self.expected if receipt is None else receipt)
        criteria = dict(self.criteria, byteReplay={'commands': [self.command if command is None else command]})
        (self.folder / 'CRITERIA.json').write_text(json.dumps(criteria))
        module = sys.modules[__name__]
        result = unittest.TestResult()
        with (patch.object(module, 'ROOT', self.root), patch.object(module, 'REQUIRED', {self.folder.name}),
              patch.object(module, 'MODES', (((), 'SURFACE-FIT.json'),)),
              patch.object(module, 'active_coverage', return_value=None),
              patch.object(module, 'replay', return_value=self.expected if output is None else output,
                           side_effect=error) as launch):
            HullMeasurementReplays('test_frozen_fit_and_difference_are_recomputed_byte_for_byte').run(result)
        return result, launch

    def test_changed_script_report_or_command_is_refused_before_dispatch(self):
        for change in ({'source': b'changed'}, {'receipt': b'changed'}, {'command': 'changed'}):
            with self.subTest(change=change):
                result, launch = self.probe(**change)
                self.assertFalse(result.wasSuccessful())
                self.assertEqual(len(result.failures), 1)
                launch.assert_not_called()

    def test_missing_declared_cli_is_not_skipped_in_either_replay_convention(self):
        module = sys.modules[__name__]
        for suffix, flags, receipt, method in (
            ('', (), 'SURFACE-FIT.json', 'test_frozen_fit_and_difference_are_recomputed_byte_for_byte'),
            ('_geometry', ('archive-fit',), 'ARCHIVE-FIT.json',
             'test_archive_candidate_and_fidelity_are_recomputed_byte_for_byte'),
        ):
            script = self.script.with_name(f'measure_example{suffix}.py')
            script_name = script.relative_to(self.root).as_posix()
            with patch.object(module, 'ROOT', self.root):
                command = receipt_command(self.folder, script, flags, receipt)
            for pins, commands in (({script_name: 'pinned'}, []), ({}, [command])):
                (self.folder / 'CRITERIA.json').write_text(json.dumps({
                    'replayPins': pins, 'byteReplay': {'commands': commands},
                }))
                result = unittest.TestResult()
                with (self.subTest(suffix=suffix, pins=pins),
                      patch.object(module, 'ROOT', self.root), patch.object(module, 'REQUIRED', set()),
                      patch.object(module, 'GEOMETRY_REQUIRED', set()),
                      patch.object(module, 'MODES', ((flags, receipt),)),
                      patch.object(module, 'GEOMETRY_MODES', ((flags, receipt),)),
                      patch.object(module, 'active_coverage', return_value=None),
                      patch.object(module, 'replay') as launch):
                    HullMeasurementReplays(method).run(result)
                    self.assertEqual(len(result.failures), 1)
                    self.assertIn(f'Missing pinned measurement CLI: {script_name}', result.failures[0][1])
                    launch.assert_not_called()

    def test_wrong_bytes_and_worker_failure_propagate_to_unittest(self):
        result, launch = self.probe(output=b'wrong')
        self.assertEqual(len(result.failures), 1)
        launch.assert_called_once()
        result, launch = self.probe(error=RuntimeError('worker failed'))
        self.assertEqual(len(result.errors), 1)
        self.assertIn('worker failed', result.errors[0][1])
        launch.assert_called_once()

    def test_nonzero_worker_cannot_pass_on_matching_stdout(self):
        self.script.write_text('import sys; sys.stdout.write("expected"); sys.stderr.write("failure"); sys.exit(7)')
        with self.assertRaisesRegex(RuntimeError, 'exited 7: failure'):
            replay(self.script, ())

    def test_missing_or_duplicate_worker_coverage_cannot_be_combined(self):
        coverage_file = self.root / 'isolated.coverage'
        with self.assertRaisesRegex(RuntimeError, 'found 0'):
            merge_worker_coverage(None, coverage_file, self.script)
        for suffix in ('one', 'two'):
            coverage_file.with_name(coverage_file.name + '.' + suffix).touch()
        with self.assertRaisesRegex(RuntimeError, 'found 2'):
            merge_worker_coverage(None, coverage_file, self.script)

    def test_worker_coverage_must_contain_real_script_lines_before_merge(self):
        coverage_file = self.root / 'isolated.coverage'
        coverage_file.with_name(coverage_file.name + '.one').touch()
        module, current = Mock(), Mock()
        data = module.CoverageData.return_value
        data.measured_files.return_value = {'tools/measure_example.py'}
        with patch.object(sys.modules[__name__], 'ROOT', self.root):
            data.lines.return_value = []
            with self.assertRaisesRegex(RuntimeError, 'No executed lines'):
                merge_worker_coverage((module, current), coverage_file, self.script)
            current.get_data.assert_not_called()
            data.lines.return_value = [1]
            merge_worker_coverage((module, current), coverage_file, self.script)
        current.get_data.return_value.update.assert_called_once_with(data)
        self.assertEqual(data.close.call_count, 2)
