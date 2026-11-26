"""Verify SH022 source identity, exact visual output, independent fit and failure controls."""

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

import prepare_peregrine_mk_i as importer
import verify_peregrine_mk_i_fit as independent


class PeregrineMkI(unittest.TestCase):
    def setUp(self):
        self.inputs = {name: (importer.ASSETS/name).read_bytes() for name in (*importer.PINS,importer.CANDIDATE_RECEIPT)}

    def test_source_ownership_visual_and_frozen_engineering_rebuild_exactly(self):
        produced = importer.build(self.inputs)
        for name, raw in produced.items():
            with self.subTest(name=name):
                self.assertEqual((importer.ASSETS/name).read_bytes(), raw)
        manifest = json.loads(produced["MANIFEST.json"])
        self.assertEqual(manifest["id"], "SH022")
        self.assertEqual(manifest["collision"]["pieces"], 69)
        self.assertEqual(manifest["nativeCandidate"]["exactFaceLimit"], 512)
        self.assertGreater(manifest["nativeCandidate"]["faces"], 512)
        component = json.loads(produced["COMPONENT-PIECES.json"])
        self.assertEqual(len(component["detailed"]),165)
        self.assertEqual(len(component["nativeCandidate"]),27)
        self.assertEqual(len(component["pieces"]),69)
        self.assertEqual(set(range(69)),{p["piece"] for name in ("detailed","nativeCandidate") for p in component[name]})
        self.assertEqual(component["pieces"][0],"pods explicit member 0")
        self.assertEqual(component["pieces"][15],"pods explicit member 1")
        self.assertEqual(component["pieces"][30:33],["fork longitudinal envelope (negative X)","fork longitudinal envelope (positive X)","fork aft cross bridge"])
        self.assertEqual(manifest["nativeCandidate"]["faces"],964)
        receipt=json.loads(self.inputs[importer.CANDIDATE_RECEIPT])
        self.assertEqual(hashlib.sha256(self.inputs[importer.CANDIDATE_RECEIPT]).hexdigest(),importer.CANDIDATE_STATUS_SHA)
        self.assertEqual(receipt["peerProposal"]["status"],"IMPLEMENTED_SOURCE_GENERATOR_UNEXECUTED_NOT_NATIVE_QUALIFIED")
        self.assertEqual(receipt["peerProposal"]["reportedOriginalNativeFaces"],964)
        self.assertEqual(receipt["peerProposal"]["exactFaceLimit"],512)
        actual=receipt["laterQualificationInputs"]
        self.assertEqual(actual["status"],"INPUTS_FOR_G1_ONLY_NOT_FIDELITY_NATIVE_PHYSICAL_OR_RUNTIME_QUALIFICATION")
        self.assertEqual(actual["cache"],{"path":"actual-run/candidates/SH022.json","bytes":68833,"sha256":"bde6d62a7afe085e73027db3b57ea715615bff89e4de0b2fe82af9c07e52d70f"})
        self.assertIsNone(actual["candidateFaceCount"])
        self.assertEqual(actual["continuousFidelityStatus"],"UNISSUED_GENERAL_METRIC_AND_TOLERANCE_NO_AUTOMATIC_PASS")
        self.assertEqual(manifest["separateNativeCandidate"]["status"],"CACHED_INPUT_UNQUALIFIED_NOT_SELECTED")
        visual=produced["visual.bin"]
        self.assertEqual(struct.unpack_from("<8sII",visual),(b"SH022VZ1",10452,56))
        fingerprint=0xcbf29ce484222325
        for byte in visual:
            fingerprint=((fingerprint^byte)*0x100000001b3)&((1<<64)-1)
        renderer=(importer.ROOT/"crates/meridian-desktop/src/mule_render/showroom.rs").read_text()
        self.assertEqual(fingerprint,int(re.search(r"PEREGRINE_I_VISUAL_FNV:\s*u64\s*=\s*(0x[\da-f_]+)",renderer)[1].replace("_",""),0))
        physical = json.loads(self.inputs["physical.json"])
        self.assertEqual(physical["dryMassKilograms"], 103972)
        self.assertEqual(manifest["physicalAuthority"]["sha256"], hashlib.sha256(self.inputs["physical.json"]).hexdigest())
        runtime = (importer.ROOT/"crates/meridian-core/src/peregrine_mk_i.rs").read_text()
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
        runtime = (importer.ROOT/"crates/meridian-core/src/peregrine_mk_i.rs").read_text()
        checks = (importer.ROOT/"crates/meridian-core/src/peregrine_mk_i/tests.rs").read_text()
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
        self.assertEqual(words("ORIGINAL_VERTEX96_BITS"),[bits(Fraction(x)*ratio) for x in resource["design"]["vertices"][96]])
        self.assertEqual(re.search(r'PEREGRINE_I_CONTRACT_SHA256:\s*&str\s*=\s*"([\da-f]+)"',runtime)[1],
                         hashlib.sha256(self.inputs["contract.toml"]).hexdigest())
        self.assertEqual(re.search(r'PEREGRINE_I_PHYSICAL_SHA256:\s*&str\s*=\s*"([\da-f]+)"',runtime)[1],
                         hashlib.sha256(self.inputs["physical.json"]).hexdigest())
        speed = re.search(r"PEREGRINE_I_MAX_SPEED_UNITS:\s*f32\s*=\s*([\d.]+);",runtime)
        self.assertEqual(Decimal(speed[1]),contract["performance"]["maxSpeed"])
        self.assertEqual(int(re.search(r"PEREGRINE_I_NATIVE_EXACT_FACE_LIMIT:\s*usize\s*=\s*(\d+)",runtime)[1]),contract["geometry"]["nativeFaceLimit"])
        world = re.search(r"fn peregrine_mk_i_world_enforces_own_r3_speed_and_snapshot_scope[\s\S]+",world_checks)[0]
        self.assertRegex(world,r"inbound\(0.0,\s*220.0,\s*37.0\)")
        self.assertRegex(world,r"speed in \[0.0,\s*37.0_f32.next_up\(\)\]")

    def test_all_original_source_mutations_refuse_and_wrong_sibling_is_refused(self):
        for name, raw in self.inputs.items():
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "candidate receipt" if name==importer.CANDIDATE_RECEIPT else name):
                importer.build({**self.inputs, name: raw+b" "})
        explorer = (importer.ROOT/"assets/fleet/sh008/resource.toml").read_bytes()
        with self.assertRaisesRegex(ValueError, "resource.toml"):
            importer.build({**self.inputs, "resource.toml":explorer})
        custom=(importer.ROOT/"assets/fleet/sh010/resource.toml").read_bytes()
        with self.assertRaisesRegex(ValueError,"resource.toml"):
            importer.build({**self.inputs,"resource.toml":custom})

    def test_staging_reads_fixed_blob_argv_in_repository_with_spaces_and_refuses_missing_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo=Path(tmp)/"legacy repository with spaces"
            repo.mkdir()
            hooks=Path(tmp)/"empty hooks"
            hooks.mkdir()
            subprocess.run(["git","init","--quiet"],cwd=repo,check=True,capture_output=True)
            (repo/".gitattributes").write_text("* -text\n",encoding="utf-8")
            for name,(path,_) in importer.PINS.items():
                destination=repo/path
                destination.parent.mkdir(parents=True,exist_ok=True)
                destination.write_bytes(self.inputs[name])
            subprocess.run(["git","add","--all"],cwd=repo,check=True,capture_output=True)
            subprocess.run(["git","-c","user.name=Source Fixture","-c","user.email=fixture@example.invalid",
                            "-c","commit.gpgSign=false","-c",f"core.hooksPath={hooks}","commit","--quiet","-m","test source fixture"],
                           cwd=repo,check=True,capture_output=True)
            revision=subprocess.check_output(["git","rev-parse","HEAD"],cwd=repo,text=True).strip()
            with patch.object(importer,"LEGACY_REVISION",revision):
                real_check_output=subprocess.check_output
                with patch.object(importer.subprocess,"check_output",wraps=real_check_output) as calls:
                    self.assertEqual(importer.read_legacy_inputs(repo),{name:self.inputs[name] for name in importer.PINS})
                    self.assertEqual(calls.call_count,len(importer.PINS))
                    for call,(_, (source_path,_)) in zip(calls.call_args_list,importer.PINS.items(),strict=True):
                        self.assertEqual(call.args,(["git","cat-file","blob",f"{revision}:{source_path}"],))
                        self.assertEqual(call.kwargs,{"cwd":repo.resolve(strict=True)})
                with patch.object(importer,"PINS",{**importer.PINS,"missing":("missing-source.blob","unused")}):
                    output=Path(tmp)/"missing blob output"
                    with patch.object(importer,"ASSETS",output),patch.object(sys,"argv",["prepare_peregrine_mk_i.py","--legacy-repo",str(repo)]):
                        with self.assertRaises(subprocess.CalledProcessError):
                            importer.main()
                    self.assertFalse(output.exists())
                with self.assertRaises(FileNotFoundError):
                    importer.read_legacy_inputs(repo/"missing directory")

    def test_unqualified_candidate_parent_mutation_is_refused(self):
        with self.assertRaisesRegex(ValueError,"candidate input"):
            importer.candidate_status({"peerBlueprint":b"changed","qualificationInputs":b"changed"})

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
            directory = Path(tmp)/"sh022"
            shutil.copytree(importer.ASSETS,directory)
            with patch.object(importer,"ASSETS",directory), patch.object(sys,"argv",["prepare_peregrine_mk_i.py","--check"]), redirect_stdout(StringIO()):
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
                         "93a6f508e260c271f6598881d68992116e923f68f657c3b8fa954080d7e2d10d")
        actual = independent.verify()
        self.assertTrue(independent.same_witness(expected,actual))
        self.assertGreater(actual["bowGapChannel"]["minimumSeparatingGapMetres"],2.71)
        self.assertEqual(actual["bowGapChannel"]["localTranslationMetres"],[0,0,10])
        self.assertGreater(actual["completeControlledSweep"]["offApertureGapMetres"],3.23)
        self.assertGreater(actual["alignedApertureSlackMetres"],41.6)
        self.assertLess(actual["offApertureSlackMetres"],0)
        self.assertGreater(actual["offApertureStationSweepGapMetres"],4.05)
        self.assertGreater(min(actual["lip"]["barycentric"]),0)
        self.assertEqual(actual["launch"]["outwardF32Bits"],"0x44909320")
        expected["bowGapChannel"]["minimumSeparatingGapMetres"] += 1
        self.assertFalse(independent.same_witness(expected,actual))


if __name__ == "__main__":
    unittest.main()
