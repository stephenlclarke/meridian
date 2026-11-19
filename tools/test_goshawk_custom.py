"""Verify SH010 source identity, exact visual output, independent fit and failure controls."""

from contextlib import redirect_stdout
import hashlib
from io import StringIO
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import prepare_goshawk_custom as importer
import verify_goshawk_custom_fit as independent


class GoshawkCustom(unittest.TestCase):
    def setUp(self):
        self.inputs = {name: (importer.ASSETS/name).read_bytes() for name in importer.PINS}

    def test_source_ownership_visual_and_frozen_engineering_rebuild_exactly(self):
        produced = importer.build(self.inputs)
        for name, raw in produced.items():
            with self.subTest(name=name):
                self.assertEqual((importer.ASSETS/name).read_bytes(), raw)
        manifest = json.loads(produced["MANIFEST.json"])
        self.assertEqual(manifest["id"], "SH010")
        self.assertEqual(manifest["collision"]["pieces"], 73)
        self.assertEqual(manifest["nativeCandidate"]["exactFaceLimit"], 512)
        self.assertGreater(manifest["nativeCandidate"]["faces"], 512)
        component = json.loads(produced["COMPONENT-PIECES.json"])
        self.assertEqual([p["piece"] for p in component["detailed"][-4:]], list(range(69,73)))
        self.assertEqual([p["piece"] for p in component["nativeCandidate"][-4:]], list(range(69,73)))
        physical = json.loads(self.inputs["physical.json"])
        self.assertEqual(physical["dryMassKilograms"], 32411)
        self.assertEqual(manifest["physicalAuthority"]["sha256"], hashlib.sha256(self.inputs["physical.json"]).hexdigest())
        runtime = (importer.ROOT/"crates/meridian-core/src/goshawk_custom.rs").read_text()
        for field,key in (("center_of_mass_metres","centreOfMassMetres"),
                          ("inertia_about_center_kg_m2","inertiaAboutCentreOfMassKgM2")):
            record = re.search(rf"{field}:\s*\[([^]]+)\]",runtime)
            self.assertIsNotNone(record)
            self.assertEqual([float(token) for token in record[1].split(",") if token.strip()],physical[key])
        scalar = re.search(r"dry_mass_kilograms:\s*(\d+)",runtime)
        self.assertIsNotNone(scalar)
        self.assertEqual(int(scalar[1]),physical["dryMassKilograms"])

    def test_all_original_source_mutations_refuse_and_wrong_sibling_is_refused(self):
        for name, raw in self.inputs.items():
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, name):
                importer.build({**self.inputs, name: raw+b" "})
        explorer = (importer.ROOT/"assets/fleet/sh008/resource.toml").read_bytes()
        with self.assertRaisesRegex(ValueError, "resource.toml"):
            importer.build({**self.inputs, "resource.toml":explorer})

    def test_creator_component_identity_rejects_material_and_membership_drift(self):
        resource, owner, _ = importer.authenticated(self.inputs)
        detailed, _, _ = importer.custom_partition()
        with self.assertRaisesRegex(ValueError, "Unassigned source vertices"):
            importer.stage_variant(resource["design"],owner["models"]["design"],importer.COUNTS[0],
                                   {k:v for k,v in detailed.items() if k!=0})
        material = resource["design"]["faceMaterials"]
        material[0] = 2
        with self.assertRaisesRegex(ValueError, "Creator material run mismatch"):
            importer.stage_variant(resource["design"],owner["models"]["design"],importer.COUNTS[0],detailed)

    def test_immutable_check_refuses_corrupt_asset_without_replacing_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)/"sh010"
            shutil.copytree(importer.ASSETS,directory)
            with patch.object(importer,"ASSETS",directory), patch.object(sys,"argv",["prepare_goshawk_custom.py","--check"]), redirect_stdout(StringIO()):
                importer.main()
                path = directory/"source-triangles.bin"
                damaged = path.read_bytes()[12:]
                path.write_bytes(damaged)
                with self.assertRaisesRegex(ValueError,"source-triangles"):
                    importer.main()
                self.assertEqual(path.read_bytes(),damaged)

    def test_optimized_interpreter_refuses_to_run(self):
        result = subprocess.run([sys.executable,"-B","-O",importer.__file__,"--check"],
                                capture_output=True,text=True,check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("requires Python assertions",result.stderr)

    def test_independent_fit_opening_contact_and_launch_reproduce_the_witness(self):
        expected = json.loads(independent.DESTINATION.read_bytes())
        self.assertEqual(hashlib.sha256(independent.DESTINATION.read_bytes()).hexdigest(),
                         "cbb4db3e32012690531819debd30564625a2e723f81a62c1beb26177e7ed9281")
        actual = independent.verify()
        self.assertTrue(independent.same_witness(expected,actual))
        self.assertGreater(actual["forwardChannel"]["minimumSeparatingGapMetres"],1.4)
        self.assertGreater(actual["alignedApertureSlackMetres"],40)
        self.assertLess(actual["offApertureSlackMetres"],0)
        self.assertGreater(actual["offApertureStationSweepGapMetres"],10)
        self.assertGreater(min(actual["lip"]["barycentric"]),0)
        self.assertEqual(actual["launch"]["outwardF32Bits"],"0x44902b9e")
        expected["forwardChannel"]["minimumSeparatingGapMetres"] += 1
        self.assertFalse(independent.same_witness(expected,actual))


if __name__ == "__main__":
    unittest.main()
