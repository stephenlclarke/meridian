"""Check exact original Reindeer Freighter references, local supports, and offline inspection streams."""
import hashlib
import json
import struct
import unittest
from unittest.mock import patch

import prepare_reindeer_freighter as prep
import verify_reindeer_freighter as proof
import rebuild_reindeer_freighter as rebuild


class ReindeerFreighterSources(unittest.TestCase):
    def test_pinned_originals_and_distinct_candidate(self):
        provenance, inputs, _, models, parts = prep.load_source()
        for name, data in inputs.items():
            with self.subTest(path=name):
                record = provenance["paths"][name]
                self.assertEqual(prep.pin(data), {key: record[key] for key in ("bytes", "sha256")})
                self.assertEqual(record["blob"], hashlib.sha1(f"blob {len(data)}\0".encode()+data).hexdigest())
        self.assertEqual([len(mesh["faces"]) for mesh in models], [2532, 1052, 512])
        self.assertEqual([len(records) for records in parts], [131, 49, 49])
        self.assertEqual(str(prep.SCALE), "1375/46")

    def test_visual_streams_preserve_both_original_meshes(self):
        _, _, _, _, _, models = prep.collision_inputs()
        derived = prep.prepare()
        for (model, _), filename in zip(models[:2], ("visual.bin","native-visual.bin"), strict=True):
            stream = derived[filename]
            self.assertEqual(stream,(prep.ORIGINAL/filename).read_bytes())
            self.assertEqual(struct.unpack_from("<8sII",stream), (b"SH150VIS", len(model["faces"])*3,56))
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
        self.assertEqual(len(result["originalOverlaps"]), 72)
        self.assertEqual(result["unchangedNativeParts"], 12)
        self.assertEqual(result["recessedNativeWells"], 16)
        self.assertEqual(sorted(result["clearanceMillimetres"].values()), [1555, 1555, 1621, 2434, 3521, 3521, 10605])
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        source_summary = prep.ORIGINAL / "SOURCE-GEOMETRY.json"
        self.assertEqual(prep.ROOT / criteria["source"], source_summary)
        geometry = json.loads(source_summary.read_bytes())
        self.assertEqual(geometry["id"], "SH150")
        pins = criteria["replayPins"]
        self.assertEqual(pins[criteria["source"]], prep.pin(source_summary.read_bytes())["sha256"])
        candidate_path = prep.ORIGINAL / "GENERATED-COLLISION.json"
        candidate_bytes = candidate_path.read_bytes()
        self.assertEqual(json.loads(candidate_bytes)["id"], "SH150")
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
        self.assertEqual(localized["archiveToCandidate"]["directedMaximumMillimetres"]["upperCeiling"], 3391)
        self.assertEqual(localized["candidateToArchive"]["directedMaximumMillimetres"]["upperCeiling"], 2496)
        fit=json.loads((prep.INTEGRATION/"SURFACE-FIT.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"],{'detailed': 2798, 'archivedNative': 3712, 'candidateNative': 4367})

    def test_studio_remapping_preserves_original_detail_and_candidate_triangles(self):
        _,inputs,_,_,_,models=prep.collision_inputs()
        studio=json.loads(prep.prepare()[prep.STUDIO_NAME])
        for label,(model,_) in (("detail",models[0]),("native",models[2])):
            delivered=studio[label]
            self.assertEqual([[delivered["vertices"][i] for i in triangle] for triangle in delivered["triangles"]],
                             [[model["vertices"][i] for i in triangle] for triangle in model["faces"]])
        self.assertEqual(set(studio["detail"]["groups"]+studio["native"]["groups"]),set(range(131)))
        finite=json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
        self.assertEqual(studio["engineering"]["massKg"],finite["massKg"])
        self.assertEqual(studio["engineering"]["centerOfMass"],finite["centreMetres"])
        self.assertEqual(sum(studio["engineering"]["inertia"],[]),finite["inertiaAboutCentreOfMassKgM2"])
        self.assertEqual(len(studio["clearances"]),7)
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        signed=criteria["hullStudio"]["signedInstalledExport"]
        self.assertEqual(signed["triangleRoles"]["native"], "derived native inspection candidate")
        self.assertEqual(signed["nativeTriangles"], 512)
        self.assertEqual(criteria["faces"]["archivedNative"], 1052)

    def test_rebuild_uses_the_authored_seam_and_rejects_an_unrelated_source_face(self):
        rebuild.main()
        records = json.loads((prep.INTEGRATION / "REBUILD-SELECTIONS.json").read_bytes())["parts"]
        _, _, _, (_, original, _), (_, parts, _) = prep.load_source()
        record = records["5"]
        source = proof.section(original, parts[5])
        _, _, _, mappings = rebuild.rebuilt_section(source, record["indices"], record)
        self.assertEqual(mappings[7]["uv"], [source[3][6]["uv"][i] for i in (1, 2, 2)])
        record["sourceFaceIndices"][7] = 0
        with self.assertRaisesRegex(ValueError, "nearest source surface"):
            rebuild.rebuilt_section(source, record["indices"], record)
        with patch.object(rebuild, "REBUILD_SELECTIONS_SHA", "0" * 64):
            with self.assertRaisesRegex(ValueError, "Changed Reindeer reconstruction selections"):
                rebuild.main()

    def test_check_replays_without_writing(self):
        with patch.object(prep,"replace_output",side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv",["prepare_reindeer_freighter","--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
