"""Verify SH017 source identity, exact visual output, independent fit and failure controls."""

from contextlib import redirect_stdout
from decimal import Decimal
from fractions import Fraction
import hashlib
from io import StringIO
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import prepare_mole_prospector as importer
import verify_mole_prospector_fit as independent


class MoleProspector(unittest.TestCase):
    def setUp(self):
        self.inputs = {name: (importer.ASSETS/name).read_bytes() for name in importer.PINS}

    def test_source_ownership_visual_and_frozen_engineering_rebuild_exactly(self):
        produced = importer.build(self.inputs)
        for name, raw in produced.items():
            with self.subTest(name=name):
                self.assertEqual((importer.ASSETS/name).read_bytes(), raw)
        manifest = json.loads(produced["MANIFEST.json"])
        self.assertEqual(manifest["id"], "SH017")
        self.assertEqual(manifest["collision"]["pieces"], 83)
        self.assertEqual(manifest["nativeCandidate"]["exactFaceLimit"], 512)
        self.assertGreater(manifest["nativeCandidate"]["faces"], 512)
        component = json.loads(produced["COMPONENT-PIECES.json"])
        self.assertEqual(len(component["detailed"]),179)
        self.assertEqual(len(component["nativeCandidate"]),33)
        self.assertEqual(len(component["pieces"]),83)
        self.assertEqual(set(range(83)),{p["piece"] for name in ("detailed","nativeCandidate") for p in component[name]})
        for side, head, first in (("negative X",52,53),("positive X",58,59)):
            self.assertEqual(component["pieces"][head],f"boring head ({side})")
            self.assertEqual(component["pieces"][first:first+4],[f"boring head collar ({side}) axial {n}" for n in range(4)])
        collars = {p["piece"] for p in component["detailed"] if p["name"] == "boring head collar"}
        self.assertEqual(collars,{53,54,55,56,59,60,61,62})
        self.assertFalse(collars & {p["piece"] for p in component["nativeCandidate"]})
        visual = produced["visual.bin"]
        self.assertEqual(struct.unpack_from("<8sII",visual),(b"SH017VZ1",38508,56))
        fingerprint = 0xcbf29ce484222325
        for byte in visual:
            fingerprint = ((fingerprint ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
        renderer = (importer.ROOT/"crates/meridian-desktop/src/mule_render/showroom.rs").read_text()
        reported = re.search(r"PROSPECTOR_VISUAL_FNV:\s*u64\s*=\s*(0x[\da-f_]+)",renderer)
        self.assertEqual(fingerprint,int(reported[1].replace("_",""),0))
        physical = json.loads(self.inputs["physical.json"])
        self.assertEqual(physical["dryMassKilograms"], 116421)
        self.assertEqual(manifest["physicalAuthority"]["sha256"], hashlib.sha256(self.inputs["physical.json"]).hexdigest())
        runtime = (importer.ROOT/"crates/meridian-core/src/mole_prospector.rs").read_text()
        for field,key in (("center_of_mass_metres","centreOfMassMetres"),
                          ("inertia_about_center_kg_m2","inertiaAboutCentreOfMassKgM2")):
            record = re.search(rf"{field}:\s*\[([^]]+)\]",runtime)
            self.assertIsNotNone(record)
            self.assertEqual([float(token) for token in record[1].split(",") if token.strip()],physical[key])
        scalar = re.search(r"dry_mass_kilograms:\s*(\d+)",runtime)
        self.assertIsNotNone(scalar)
        self.assertEqual(int(scalar[1]),physical["dryMassKilograms"])

    def test_compiled_test_words_and_speed_bind_to_original_physical_and_contract_bytes(self):
        physical = json.loads(self.inputs["physical.json"], parse_float=Decimal)
        resource = tomllib.loads(self.inputs["resource.toml"].decode(), parse_float=Decimal)
        contract = tomllib.loads(self.inputs["contract.toml"].decode(), parse_float=Decimal)
        runtime = (importer.ROOT/"crates/meridian-core/src/mole_prospector.rs").read_text()
        checks = (importer.ROOT/"crates/meridian-core/src/mole_prospector/tests.rs").read_text()
        world_checks = (importer.ROOT/"crates/meridian-core/src/flight/goshawk_validation/tests.rs").read_text()

        def words(name):
            record = re.search(rf"const {name}:[^=]+?=\s*\[([^]]+)\]",checks)
            self.assertIsNotNone(record,name)
            return [int(token.strip().replace("_",""),0) for token in record[1].split(",") if token.strip()]

        def integer(name):
            record = re.search(rf"const {name}:[^=]+?=\s*(0x[\da-f_]+|[\d_]+);",checks)
            self.assertIsNotNone(record,name)
            return int(record[1].replace("_",""),0)

        def bits(value):
            return struct.unpack(">Q",struct.pack(">d",float(value)))[0]

        self.assertEqual(integer("ORIGINAL_MASS_KG"),physical["dryMassKilograms"])
        self.assertEqual(words("ORIGINAL_COM_BITS"),[bits(x) for x in physical["centreOfMassMetres"]])
        self.assertEqual(words("ORIGINAL_INERTIA_BITS"),[bits(x) for x in physical["inertiaAboutCentreOfMassKgM2"]])
        ratio = Fraction(physical["metresPerSourceUnitExact"])
        self.assertEqual(words("ORIGINAL_SCALE"),[ratio.numerator,ratio.denominator])
        self.assertEqual(integer("ORIGINAL_SCALE_BITS"),bits(ratio))
        self.assertEqual(words("ORIGINAL_VERTEX1776_BITS"),[bits(Fraction(x)*ratio) for x in resource["design"]["vertices"][1776]])
        self.assertEqual(re.search(r'PROSPECTOR_CONTRACT_SHA256:\s*&str\s*=\s*"([\da-f]+)"',runtime)[1],
                         hashlib.sha256(self.inputs["contract.toml"]).hexdigest())
        self.assertEqual(re.search(r'PROSPECTOR_PHYSICAL_SHA256:\s*&str\s*=\s*"([\da-f]+)"',runtime)[1],
                         hashlib.sha256(self.inputs["physical.json"]).hexdigest())
        speed = re.search(r"PROSPECTOR_MAX_SPEED_UNITS:\s*f32\s*=\s*([\d.]+);",runtime)
        self.assertEqual(Decimal(speed[1]),contract["performance"]["maxSpeed"])
        world = re.search(r"fn mole_prospector_world_enforces_own_r3_speed_and_snapshot_scope[\s\S]+",world_checks)[0]
        self.assertRegex(world,r"inbound\(0.0,\s*220.0,\s*29.0\)")
        self.assertRegex(world,r"speed in \[0.0,\s*29.0_f32.next_up\(\)\]")

    def test_all_original_source_mutations_refuse_and_wrong_sibling_is_refused(self):
        for name, raw in self.inputs.items():
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, name):
                importer.build({**self.inputs, name: raw+b" "})
        explorer = (importer.ROOT/"assets/fleet/sh008/resource.toml").read_bytes()
        with self.assertRaisesRegex(ValueError, "resource.toml"):
            importer.build({**self.inputs, "resource.toml":explorer})
        custom = (importer.ROOT/"assets/fleet/sh010/resource.toml").read_bytes()
        with self.assertRaisesRegex(ValueError, "resource.toml"):
            importer.build({**self.inputs, "resource.toml":custom})

    def test_mole_sibling_catalogue_identity_and_relabelled_source_refusal(self):
        catalogue = importer.ROOT/"crates/meridian-desktop/src/docked_screens/data"
        def rows(name):
            return [line.split("\t") for line in (catalogue/name).read_text(encoding="utf-8").splitlines()]

        objects = [(row[0],row[1]) for row in rows("objects.tsv") if row[0] in {"SH016","SH017"}]
        self.assertEqual(objects,[("SH016","Mole Miner"),("SH017","Mole Prospector")])
        specs = rows("specifications.tsv")
        criteria = json.loads((independent.DESTINATION.parent/"CRITERIA.json").read_bytes())
        identity = criteria["joinedRegression"]["siblingIdentity"]
        for id, name, speed in (("SH016","Mole Miner",25),("SH017","Mole Prospector",29)):
            with self.subTest(id=id):
                self.assertEqual(identity[id]["name"],name)
                self.assertEqual(identity[id]["maxSpeedUnits"],speed)
                self.assertEqual(identity[id]["nativeExactFaceLimit"],512)
                self.assertEqual([row[3] for row in specs if row[:3]==[id,"performance","maxSpeed"]],[f"{speed}.0"])
                self.assertEqual([row[3] for row in specs if row[:3]==[id,"geometry","nativeFaceLimit"]],["512"])
        for key, asset in (("frozenResourceSha256","resource.toml"),("frozenContractSha256","contract.toml")):
            self.assertEqual(identity["SH017"][key],hashlib.sha256(self.inputs[asset]).hexdigest())
            self.assertNotEqual(identity["SH016"][key],identity["SH017"][key])
        # A relabelled SH017 source is not SH016. The distinct frozen SH016 source is not bundled here.
        changed = self.inputs["resource.toml"].replace(b'id = "SH017"',b'id = "SH016"',1)
        self.assertNotEqual(changed,self.inputs["resource.toml"])
        with self.assertRaisesRegex(ValueError,"resource.toml"):
            importer.build({**self.inputs,"resource.toml":changed})

    def test_creator_component_identity_rejects_material_and_membership_drift(self):
        resource, owner, _ = importer.authenticated(self.inputs)
        detailed, _, _ = importer.partition(resource,owner)
        with self.assertRaisesRegex(ValueError, "Unassigned source vertices"):
            importer.stage_variant(resource["design"],owner["models"]["design"],importer.COUNTS[0],
                                   {k:v for k,v in detailed.items() if k!=0})
        material = resource["design"]["faceMaterials"]
        material[0] = 2
        with self.assertRaisesRegex(ValueError, "Creator material run mismatch"):
            importer.stage_variant(resource["design"],owner["models"]["design"],importer.COUNTS[0],detailed)

    def test_immutable_check_refuses_corrupt_asset_without_replacing_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)/"sh017"
            shutil.copytree(importer.ASSETS,directory)
            with patch.object(importer,"ASSETS",directory), patch.object(sys,"argv",["prepare_mole_prospector.py","--check"]), redirect_stdout(StringIO()):
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
                         "b4e6b0fbe4f0951ddcf34daa912bc555c62b967e48909591e0faf9772046a1bf")
        actual = independent.verify()
        self.assertTrue(independent.same_witness(expected,actual))
        self.assertGreater(actual["toolCradleChannel"]["minimumSeparatingGapMetres"],2.72)
        self.assertGreater(actual["completeControlledSweep"]["offApertureGapMetres"],0.69)
        self.assertGreater(actual["alignedApertureSlackMetres"],40)
        self.assertLess(actual["offApertureSlackMetres"],0)
        self.assertGreater(actual["offApertureStationSweepGapMetres"],1.5)
        self.assertGreater(min(actual["lip"]["barycentric"]),0)
        self.assertEqual(actual["launch"]["outwardF32Bits"],"0x449101da")
        expected["toolCradleChannel"]["minimumSeparatingGapMetres"] += 1
        self.assertFalse(independent.same_witness(expected,actual))


if __name__ == "__main__":
    unittest.main()
