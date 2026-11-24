"""Execute both exact numerical receipts for each compact original-hull measurement CLI."""

import contextlib
import hashlib
from io import BytesIO, TextIOWrapper
import json
from pathlib import Path
import runpy
import sys
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = {'sh044-harpy-assault', 'sh045-erne-corvette', 'sh046-harpy-dropship'}
MODES = (((), 'SURFACE-FIT.json'), (('--difference',), 'ORIGINAL-DIFFERENCE.json'))


def measurement_cases():
    # The compact hull convention pairs integration/shNNN-hull-name with
    # tools/measure_hull_name.py. New hulls using that convention join the test
    # automatically; older, separately qualified _air or Goshawk tools do not.
    for folder in sorted((ROOT / 'integration').glob('sh[0-9][0-9][0-9]-*')):
        slug = folder.name.split('-', 1)[1].replace('-', '_')
        script = ROOT / 'tools' / f'measure_{slug}.py'
        if script.is_file():
            yield folder, script


def replay(script, flags):
    # Run the CLI entry point in this process so the project's ordinary Python
    # coverage includes numerical work and both argparse branches.
    with BytesIO() as output, TextIOWrapper(output, encoding='utf-8') as stdout:
        with contextlib.redirect_stdout(stdout), patch.object(sys, 'argv', [str(script), *flags]):
            runpy.run_path(str(script), run_name='__main__')
        stdout.flush()
        return output.getvalue()


class HullMeasurementReplays(unittest.TestCase):
    def test_frozen_fit_and_difference_are_recomputed_byte_for_byte(self):
        cases = list(measurement_cases())
        self.assertTrue(REQUIRED <= {folder.name for folder, _ in cases})
        for folder, script in cases:
            criteria = json.loads((folder / 'CRITERIA.json').read_bytes())
            script_name = script.relative_to(ROOT).as_posix()
            with self.subTest(hull=folder.name, script=script_name):
                self.assertEqual(hashlib.sha256(script.read_bytes()).hexdigest(),
                                 criteria['replayPins'][script_name])
            for flags, report in MODES:
                path = folder / report
                name = path.relative_to(ROOT).as_posix()
                command = ' '.join(('python3', '-B', script_name, *flags, '|', 'cmp', '-', name))
                with self.subTest(hull=folder.name, receipt=report):
                    self.assertIn(command, criteria['byteReplay']['commands'])
                    expected = path.read_bytes()
                    self.assertEqual(hashlib.sha256(expected).hexdigest(), criteria['replayPins'][name])
                    self.assertEqual(replay(script, flags), expected)
