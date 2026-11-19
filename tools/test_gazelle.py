"""Independent SH019 source, seam, material and fit regression tests."""

from contextlib import redirect_stdout
from collections import Counter
from decimal import Decimal
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

from encode_goshawk_visual import encode_model
import prepare_gazelle as importer
import verify_gazelle_fit as independent


class Gazelle(unittest.TestCase):
    def setUp(self):
        self.inputs={name:(importer.ASSETS/name).read_bytes() for name in importer.PINS}

    def test_exact_source_rebuild_and_approved_metres_without_engineering(self):
        outputs=importer.build(self.inputs)
        for name,raw in outputs.items():
            self.assertEqual((importer.ASSETS/name).read_bytes(),raw,name)
        self.assertEqual(hashlib.sha256(outputs["MANIFEST.json"]).hexdigest(),importer.MANIFEST_SHA)
        manifest=json.loads(outputs["MANIFEST.json"])
        self.assertEqual(manifest["metresPerSourceUnit"],"1")
        self.assertEqual(manifest["nativeCandidate"]["faces"],304)
        self.assertEqual(manifest["nativeCandidate"]["exactFaceLimit"],512)
        self.assertEqual(manifest["collision"]["supportPoints"],[3104,204])
        self.assertEqual(manifest["collision"]["pieces"],37)
        self.assertEqual(manifest["physicalAuthority"],{"status":"AUTHORED_METRE_GEOMETRY_ONLY_NO_REGISTERED_ENGINEERING","dryMassKg":None,"centreOfMassMetres":None,"inertiaKgM2":None})
        for name,raw in self.inputs.items():
            if name.endswith("registrations.swift"):
                self.assertNotIn(b'"SH019"',raw)

    def test_canonical_faces_support_copies_and_eight_actual_loop_sectors(self):
        manifest,vertices,faces,groups,face_groups,owners,labels=independent.read_source()
        self.assertEqual((len(vertices),len(faces),len(groups)),(3180,6204,37))
        self.assertEqual(labels[2:10],[f"elliptical service loop sector {i}" for i in range(8)])
        self.assertEqual([len(owners[i]) for i in range(2,10)],[116]*8)
        self.assertEqual([len(face_groups[i]) for i in range(2,10)],[200]*8)
        counts=Counter(i for group in owners for i in group)
        expected={676+ring*12+i for ring in range(0,64,8) for i in range(12)}|set(range(3008+44,3008+76))
        self.assertEqual({i for i,value in counts.items() if value==2},expected)
        self.assertEqual(len(expected),128)
        self.assertTrue(all(value in (1,2) for value in counts.values()))
        self.assertEqual(set(counts),set(range(len(vertices))))
        self.assertEqual(manifest["detailed"]["boundsMetres"],manifest["nativeCandidate"]["boundsMetres"])

    def test_original_visual_records_preserve_no_map_colour_uv_and_emissive(self):
        resource=importer.authenticated(self.inputs)
        raw=(importer.ASSETS/"visual.bin").read_bytes()
        self.assertEqual(struct.unpack_from("<8sII",raw),(b"SH019VZ1",17700,56))
        records=list(struct.iter_unpack("<12fII",raw[16:]))
        model=resource["design"]
        untextured=Counter()
        for i,(face,material_index,mapping) in enumerate(zip(model["faces"],model["faceMaterials"],model["faceTextures"],strict=True)):
            material=model["materials"][material_index]
            colour=[(material["colour"]>>shift&255)/255 for shift in (24,16,8,0)]
            expected_texture=3 if mapping["texture"]==-1 else mapping["texture"]
            for j,(vertex,uv) in enumerate(zip(face,mapping["uv"],strict=True)):
                actual=records[i*3+j]
                expected=struct.pack("<9f",*model["vertices"][vertex],*colour,*uv)
                self.assertEqual(struct.pack("<9f",*actual[:3],*actual[6:12]),expected)
                self.assertEqual(actual[12:],(expected_texture,int(material["emissive"])))
            if expected_texture==3:
                untextured[material["name"]]+=1
        self.assertEqual(untextured,{"glazing":960,"emitter":256})
        fingerprint=0xcbf29ce484222325
        for byte in raw:
            fingerprint=((fingerprint^byte)*0x100000001b3)&((1<<64)-1)
        renderer=(importer.ROOT/"crates/meridian-desktop/src/mule_render/showroom.rs").read_text()
        self.assertEqual(fingerprint,int(re.search(r"GAZELLE_VISUAL_FNV:\s*u64\s*=\s*(0x[\da-f_]+)",renderer)[1].replace("_",""),0))
        points=(importer.ASSETS/"source.vertices").read_bytes(); triangles=(importer.ASSETS/"source-triangles.bin").read_bytes()
        model=dict(model,vertices=[tuple(map(float,p)) for p in model["vertices"]])
        args=(model,points,triangles,importer.MAGIC,(3008,172,5900),importer.VISUAL_MAGIC)
        with self.assertRaisesRegex(ValueError,"Invalid original face"):
            encode_model(*args)
        model["faceTextures"][0]["texture"]=3
        with self.assertRaisesRegex(ValueError,"Invalid original face"):
            encode_model(*args,allow_untextured=True)

    def test_frozen_identity_speed_and_face_limit_bind_runtime(self):
        contract=tomllib.loads(self.inputs["contract.toml"].decode(),parse_float=Decimal)
        native=json.loads(self.inputs["native-source.json"])
        runtime=(importer.ROOT/"crates/meridian-core/src/gazelle.rs").read_text()
        for label,asset in (("RESOURCE","resource.toml"),("CONTRACT","contract.toml"),("NATIVE_SOURCE","native-source.json")):
            digest=re.search(rf'GAZELLE_{label}_SHA256:\s*&str\s*=\s*"([\da-f]+)"',runtime)[1]
            self.assertEqual(digest,hashlib.sha256(self.inputs[asset]).hexdigest())
        speed=Decimal(re.search(r"GAZELLE_MAX_SPEED_UNITS:\s*f32\s*=\s*([\d.]+)",runtime)[1])
        self.assertEqual(speed,contract["performance"]["maxSpeed"])
        self.assertEqual(int(re.search(r"GAZELLE_NATIVE_EXACT_FACE_LIMIT:\s*usize\s*=\s*(\d+)",runtime)[1]),contract["geometry"]["nativeFaceLimit"])
        self.assertEqual(int(re.search(r"GAZELLE_NATIVE_FACES:\s*usize\s*=\s*(\d+)",runtime)[1]),len(native["nativeDesign"]["faces"]))
        self.assertIn("GAZELLE_METRES_PER_SOURCE_UNIT: f64 = 1.0",runtime)
        self.assertNotIn("dry_mass_kilograms",runtime)
        world=(importer.ROOT/"crates/meridian-core/src/flight/goshawk_validation/tests.rs").read_text()
        self.assertRegex(world,r"inbound\(0.0,\s*220.0,\s*43.0\)")
        self.assertRegex(world,r"speed in \[0.0,\s*43.0_f32.next_up\(\)\]")

    def test_pinned_originals_refuse_all_byte_changes_and_other_resources(self):
        for name,raw in self.inputs.items():
            with self.subTest(name=name),self.assertRaisesRegex(ValueError,name):
                importer.build({**self.inputs,name:raw+b" "})
        for id in ("sh008","sh010"):
            sibling=(importer.ROOT/f"assets/fleet/{id}/resource.toml").read_bytes()
            with self.assertRaisesRegex(ValueError,"resource.toml"):
                importer.build({**self.inputs,"resource.toml":sibling})

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
                    self.assertEqual(importer.read_legacy_inputs(repo),self.inputs)
                    self.assertEqual(calls.call_count,len(importer.PINS))
                    for call,(_, (source_path,_)) in zip(calls.call_args_list,importer.PINS.items(),strict=True):
                        self.assertEqual(call.args,(["git","cat-file","blob",f"{revision}:{source_path}"],))
                        self.assertEqual(call.kwargs,{"cwd":repo.resolve(strict=True)})
                with patch.object(importer,"PINS",{**importer.PINS,"missing":("missing-source.blob","unused")}):
                    output=Path(tmp)/"missing blob output"
                    with patch.object(importer,"ASSETS",output),patch.object(sys,"argv",["prepare_gazelle.py","--legacy-repo",str(repo)]):
                        with self.assertRaises(subprocess.CalledProcessError):
                            importer.main()
                    self.assertFalse(output.exists())
                with self.assertRaises(FileNotFoundError):
                    importer.read_legacy_inputs(repo/"missing directory")

    def test_loop_topology_mutation_and_corrupted_support_refuse(self):
        resource=importer.authenticated(self.inputs)
        model=resource["design"]
        model["faces"][1344][1],model["faces"][1344][2]=model["faces"][1344][2],model["faces"][1344][1]
        with self.assertRaisesRegex(ValueError,"loop adjacency"):
            importer.stage_variant(model,"detailed",importer.COUNTS[0])
        with tempfile.TemporaryDirectory() as tmp:
            folder=Path(tmp)/"sh019";shutil.copytree(importer.ASSETS,folder)
            path=folder/"support-source-indices.bin"; damaged=bytearray(path.read_bytes());damaged[0]^=1;path.write_bytes(damaged)
            with self.assertRaisesRegex(ValueError,"Changed fixture"):
                independent.read_source(folder)
            with patch.object(importer,"ASSETS",folder),patch.object(sys,"argv",["prepare_gazelle.py","--check"]),redirect_stdout(StringIO()):
                with self.assertRaisesRegex(ValueError,"support-source-indices"):
                    importer.main()
            self.assertEqual(path.read_bytes(),damaged)

    def test_independent_station_fit_and_three_loop_corridors_replay(self):
        raw=independent.DESTINATION.read_bytes(); expected=json.loads(raw)
        self.assertEqual(hashlib.sha256(raw).hexdigest(),"ebf8e3be70d1b09e3c31966ad76b9efc4f59d489906e55f95cb409f5fa442583")
        actual=independent.verify()
        self.assertTrue(independent.same_witness(expected,actual))
        self.assertEqual(len(actual["serviceLoopChannels"]),3)
        self.assertGreater(actual["serviceLoopChannels"][0]["minimumSeparatingGapMetres"],0.68)
        self.assertTrue(all(row["minimumSeparatingGapMetres"]>0.48 for row in actual["serviceLoopChannels"][1:]))
        self.assertGreater(actual["completeControlledSweep"]["offApertureGapMetres"],3.25)
        self.assertLess(actual["offApertureSlackMetres"],0)
        self.assertGreater(min(actual["lip"]["barycentric"]),0)
        self.assertEqual(actual["launch"]["outwardF32Bits"],"0x44902429")
        expected["serviceLoopChannels"][0]["minimumSeparatingGapMetres"]+=1
        self.assertFalse(independent.same_witness(expected,actual))

    def test_optimized_interpreter_refuses(self):
        result=subprocess.run([sys.executable,"-B","-O",importer.__file__,"--check"],capture_output=True,text=True,check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("requires Python assertions",result.stderr)


if __name__=="__main__":
    unittest.main()
