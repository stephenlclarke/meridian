"""Exercise the independent source checks, including mismatches in protected inputs."""
import shutil
import tempfile
import unittest
from pathlib import Path

import verify_mule_envelope_source as verifier


class MuleSourceVerification(unittest.TestCase):
    def test_bundled_assets_core_inverse_and_reference_bindings_agree(self):
        result = verifier.verify()
        self.assertEqual(result['protectedCoreFilesChecked'], 115)
        self.assertEqual(result['protectedCoreFilesWithMuleHooks'], 6)
        self.assertEqual(result['geometryCoreTests'], 1)
        self.assertEqual(result['restoreCoreTests'], 1)
        self.assertEqual(result['totalMuleCoreTests'], 24)
        self.assertEqual(result['vertexEncoding'], 4360 + 984)
        self.assertEqual(result['triangleEncoding'], 7236 + 1772)
        self.assertFalse(result['constructorsOrContactOrContainmentOrCompilerExecuted'])

    def test_unrelated_edit_in_nested_protected_source_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = Path('crates/meridian-core/src')
            baseline = Path('integration/sh001-mule/PROTECTED-CORE.json')
            shutil.copytree(verifier.ROOT / core, root / core)
            (root / baseline).parent.mkdir(parents=True)
            shutil.copyfile(verifier.ROOT / baseline, root / baseline)
            with (root / core / 'flight/emp/tests.rs').open('ab') as stream:
                stream.write(b'\n')
            with self.assertRaisesRegex(AssertionError, 'flight/emp/tests'):
                verifier.verify_protected_core(root)

    def test_component_triangle_may_not_use_a_neighbouring_part_vertex(self):
        part = dict(index=0, name='body primary pressure envelope', vertexStart=5,
                    vertexCount=3, faceStart=0, faceCount=1)
        assigned = dict(part, component=0, piece=0)
        verifier.verify_component('design', part, assigned, [(5, 6, 7)], {})
        with self.assertRaises(AssertionError):
            verifier.verify_component('design', part, assigned, [(5, 6, 8)], {})

    def test_stag_publication_must_be_removed_exactly_once(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = Path('crates/meridian-core/src')
            baseline = Path('integration/sh001-mule/PROTECTED-CORE.json')
            shutil.copytree(verifier.ROOT / core, root / core)
            (root / baseline).parent.mkdir(parents=True)
            shutil.copyfile(verifier.ROOT / baseline, root / baseline)
            source = root / core / verifier.LIB_SOURCE
            original = source.read_bytes()
            addition = b'pub mod stag_liner;\n\n'
            for altered in (original.replace(addition, b''), original + addition):
                source.write_bytes(altered)
                with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                    verifier.verify_protected_core(root)
