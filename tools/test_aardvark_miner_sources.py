"""Check exact original Aardvark Miner references, local supports, and offline inspection streams."""
import hashlib
import json
import struct
import unittest
from unittest.mock import patch

import prepare_aardvark_miner as prep
import verify_aardvark_miner as proof


class AardvarkMinerSources(unittest.TestCase):
    def test_pinned_originals_and_distinct_candidate(self):
        provenance, inputs, _, models, parts = prep.load_source()
        for name, data in inputs.items():
            with self.subTest(path=name):
                record = provenance["paths"][name]
                self.assertEqual(prep.pin(data), {key: record[key] for key in ("bytes", "sha256")})
                self.assertEqual(record["blob"], hashlib.sha1(f"blob {len(data)}\0".encode()+data).hexdigest())
        self.assertEqual([len(mesh["faces"]) for mesh in models], [12848, 1112, 512])
        self.assertEqual([len(records) for records in parts], [180, 34, 34])
        self.assertEqual(str(prep.SCALE), "4600/481")

    def test_visual_streams_preserve_both_original_meshes(self):
        _, _, _, _, _, models = prep.collision_inputs()
        derived = prep.prepare()
        for (model, _), filename in zip(models[:2], ("visual.bin","native-visual.bin"), strict=True):
            stream = derived[filename]
            self.assertEqual(stream,(prep.ORIGINAL/filename).read_bytes())
            self.assertEqual(struct.unpack_from("<8sII",stream), (b"ADKVIZ01", len(model["faces"])*3,56))
            corners = iter(struct.iter_unpack("<12fII",stream[16:]))
            for face, idx, mapping in zip(model["faces"],model["faceMaterials"],model["faceTextures"],strict=True):
                material = model["materials"][idx]
                rgb = [(material["colour"] >> shift & 255)/255 for shift in (24,16,8,0)]
                for index,uv in zip(face,mapping["uv"],strict=True):
                    row = next(corners)
                    expected = struct.unpack("<9f",struct.pack("<9f",*model["vertices"][index],*rgb,*uv))
                    self.assertEqual(row[:3]+row[6:12],expected)
                    self.assertEqual(row[12:],(mapping["texture"] & 0xffff_ffff,int(material["emissive"])))
            self.assertIsNone(next(corners,None))

    def test_exact_contacts_routes_and_frozen_replay_pins(self):
        result = proof.verify()
        self.assertEqual(prep.encode_json(result),(prep.INTEGRATION/"CONTACT-RESULT.json").read_bytes())
        self.assertEqual(len(result["originalOverlaps"]), 31)
        self.assertEqual(result["unchangedNativeParts"], 13)
        self.assertEqual(result["recessedNativeWells"], 11)
        self.assertEqual(sorted(result["clearanceMillimetres"].values()), [841, 1726, 1876, 2077, 2077, 2579, 2579])
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        source_summary = prep.ORIGINAL / "SOURCE-GEOMETRY.json"
        self.assertEqual(prep.ROOT / criteria["source"], source_summary)
        geometry = json.loads(source_summary.read_bytes())
        self.assertEqual(geometry["id"], "SH143")
        pins = criteria["replayPins"]
        self.assertEqual(pins[criteria["source"]], prep.pin(source_summary.read_bytes())["sha256"])
        candidate_path = prep.ORIGINAL / "GENERATED-COLLISION.json"
        candidate_bytes = candidate_path.read_bytes()
        self.assertEqual(json.loads(candidate_bytes)["id"], "SH143")
        candidate_key = candidate_path.relative_to(prep.ROOT).as_posix()
        self.assertEqual(pins[candidate_key], prep.pin(candidate_bytes)["sha256"])
        self.assertEqual(geometry["pins"]["GENERATED-COLLISION.json"], pins[candidate_key])
        for name,expected in criteria["replayPins"].items():
            with self.subTest(path=name):
                self.assertEqual(prep.pin((prep.ROOT/name).read_bytes())["sha256"],expected)
        for command in criteria["byteReplay"]["commands"]:
            _, separator, target = command.partition(" | cmp - ")
            if separator:
                self.assertIn(target, pins)
        localized = json.loads((prep.INTEGRATION / "CANDIDATE-FIT-LOCALIZATION.json").read_bytes())
        self.assertEqual(localized["archiveToCandidate"]["directedMaximumMillimetres"]["upperCeiling"], 1104)
        self.assertEqual(localized["candidateToArchive"]["directedMaximumMillimetres"]["upperCeiling"], 462)
        fit=json.loads((prep.INTEGRATION/"SURFACE-FIT.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"],{'detailed': 2680, 'archivedNative': 2824, 'candidateNative': 3285})

    def test_studio_remapping_preserves_original_detail_and_candidate_triangles(self):
        _,inputs,_,_,_,models=prep.collision_inputs()
        studio=json.loads(prep.prepare()[prep.STUDIO_NAME])
        for label,(model,_) in (("detail",models[0]),("native",models[2])):
            delivered=studio[label]
            self.assertEqual([[delivered["vertices"][i] for i in triangle] for triangle in delivered["triangles"]],
                             [[model["vertices"][i] for i in triangle] for triangle in model["faces"]])
        self.assertEqual(set(studio["detail"]["groups"]+studio["native"]["groups"]),set(range(180)))
        finite=json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
        self.assertEqual(studio["engineering"]["massKg"],finite["massKg"])
        self.assertEqual(studio["engineering"]["centerOfMass"],finite["centreMetres"])
        self.assertEqual(sum(studio["engineering"]["inertia"],[]),finite["inertiaAboutCentreOfMassKgM2"])
        self.assertEqual(len(studio["clearances"]),7)
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        signed=criteria["hullStudio"]["signedInstalledExport"]
        self.assertEqual(signed["triangleRoles"]["native"], "derived native inspection candidate")
        self.assertEqual(signed["nativeTriangles"], 512)
        self.assertEqual(criteria["faces"]["archivedNative"], 1112)

    def test_check_replays_without_writing(self):
        with patch.object(prep,"replace_output",side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv",["prepare_aardvark_miner","--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
