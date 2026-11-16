"""Coverage relocation must retain every measurement and refuse unbound source files."""
import contextlib
import io
from pathlib import Path
import runpy
import sys
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import coverage_paths


class CoveragePathTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        (self.root / 'tools').mkdir()
        (self.root / 'crates/desktop/tools').mkdir(parents=True)
        (self.root / 'tools/ordinary.py').write_text('value = 1\n')
        (self.root / 'crates/desktop/tools/nested.py').write_text('value = 2\n')
        self.source = self.root / 'coverage/python.xml'
        self.source.parent.mkdir()
        self.output = self.root / 'coverage/python-sonar.xml'
        self.document = ET.fromstring('''<coverage lines-covered="1" lines-valid="2" branch-rate="0.5">
          <sources><source>tools</source><source>crates/desktop/tools</source></sources>
          <packages><package><classes>
            <class filename="ordinary.py" line-rate="1"><lines>
              <line number="1" hits="7" branch="true" condition-coverage="50% (1/2)"/>
            </lines></class>
            <class filename="nested.py" line-rate="0"><lines><line number="1" hits="0"/></lines></class>
          </classes></package></packages></coverage>''')
        self.write()

    def write(self):
        ET.ElementTree(self.document).write(self.source, encoding='utf-8', xml_declaration=True)

    def test_both_module_sources_resolve_without_changing_measurements_or_original(self):
        original = self.source.read_bytes()
        paths = coverage_paths.normalize(self.root, self.source, self.output)
        self.assertEqual(paths, ['tools/ordinary.py', 'crates/desktop/tools/nested.py'])
        output = ET.parse(self.output)
        self.assertEqual(output.find('./sources/source').text, self.root.as_posix())
        self.assertEqual(output.getroot().attrib, self.document.attrib)
        for before, after in zip(self.document.findall('.//class'), output.findall('.//class'), strict=True):
            self.assertEqual(ET.tostring(before.find('lines')), ET.tostring(after.find('lines')))
            self.assertEqual(before.get('line-rate'), after.get('line-rate'))
        self.assertEqual(self.source.read_bytes(), original)

    def test_existing_root_relative_filename_and_missing_sources_element(self):
        self.document.remove(self.document.find('sources'))
        for row, name in zip(self.document.findall('.//class'),
                             ['tools/ordinary.py', 'crates/desktop/tools/nested.py'], strict=True):
            row.set('filename', name)
        self.write()
        coverage_paths.normalize(self.root, self.source, self.output)
        self.assertEqual(ET.parse(self.output).find('./sources/source').text, self.root.as_posix())

    def test_ambiguous_and_absent_filenames_refuse_before_output_is_written(self):
        (self.root / 'crates/desktop/tools/ordinary.py').write_text('different = 1\n')
        with self.assertRaisesRegex(ValueError, 'ambiguous'):
            coverage_paths.normalize(self.root, self.source, self.output)
        self.assertFalse(self.output.exists())
        (self.root / 'crates/desktop/tools/ordinary.py').unlink()
        (self.root / 'tools/ordinary.py').unlink()
        with self.assertRaisesRegex(ValueError, 'absent'):
            coverage_paths.normalize(self.root, self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_parent_traversal_outside_source_and_empty_report_refuse(self):
        first = self.document.find('.//class')
        for name in ['', '../outside.py']:
            with self.subTest(filename=name):
                first.set('filename', name)
                self.write()
                with self.assertRaises(ValueError):
                    coverage_paths.normalize(self.root, self.source, self.output)
        first.set('filename', 'ordinary.py')
        self.document.find('./sources/source').text = str(self.root.parent)
        self.write()
        with self.assertRaisesRegex(ValueError, 'outside'):
            coverage_paths.normalize(self.root, self.source, self.output)
        self.document.find('./sources/source').text = 'tools'
        self.document.find('.//classes').clear()
        self.write()
        with self.assertRaisesRegex(ValueError, 'no source classes'):
            coverage_paths.normalize(self.root, self.source, self.output)

    def test_original_report_cannot_be_overwritten(self):
        original = self.source.read_bytes()
        with self.assertRaisesRegex(ValueError, 'original'):
            coverage_paths.normalize(self.root, self.source, self.source)
        self.assertEqual(self.source.read_bytes(), original)

    def test_cli_requires_explicit_inputs_and_executes_the_real_entry_point(self):
        with contextlib.redirect_stdout(io.StringIO()):
            coverage_paths.main(['--root', str(self.root), '--input', str(self.source), '--output', str(self.output)])
        self.assertTrue(self.output.is_file())
        script = Path(coverage_paths.__file__)
        with patch.object(sys, 'argv', [str(script)]), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                runpy.run_path(str(script), run_name='__main__')
            self.assertEqual(raised.exception.code, 2)


if __name__ == '__main__':
    unittest.main()
