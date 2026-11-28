"""Check exact original Osprey Patrol references, local supports, and offline inspection streams."""
import hashlib
import json
import struct
import unittest
from unittest.mock import patch

import prepare_osprey_patrol as prep
import rebuild_osprey_patrol as rebuild
import verify_osprey_patrol as proof


class OspreyPatrolSources(unittest.TestCase):
    def test_pinned_originals_and_distinct_candidate(self):
        provenance, inputs, _, models, parts = prep.load_source()
        for name, data in inputs.items():
            with self.subTest(path=name):
                record = provenance["paths"][name]
                self.assertEqual(prep.pin(data), {key: record[key] for key in ("bytes", "sha256")})
                self.assertEqual(record["blob"], hashlib.sha1(f"blob {len(data)}\0".encode()+data).hexdigest())
        self.assertEqual([len(mesh["faces"]) for mesh in models], [4688, 1384, 512])
        self.assertEqual([len(records) for records in parts], [168, 30, 30])
        self.assertEqual(str(prep.SCALE), "375000/49403")

    def test_either_nearest_triangle_rebuilds_the_complete_candidate(self):
        bind = rebuild.bind_texture_seam
        observed_faces = {
            (48, 52, 56): (45, (110, 111)),
            (40, 44, 48): (25, (111, 110)),
            (0, 1, 12): (0, (11, 10)),
        }
        for side in (0, 1):
            with self.subTest(side=side):
                def choose_side(original, ids, faces, centers, nearest, seam):
                    # Local candidate, original pair as observed on the authoring model.
                    index, pair = observed_faces[seam[0]]
                    nearest[index] = pair[side]
                    bind(original, ids, faces, centers, nearest, seam)

                with patch.object(rebuild, "bind_texture_seam", side_effect=choose_side) as observed:
                    rebuild.main()
                    self.assertEqual(observed.call_count, len(observed_faces))

    def test_seam_refuses_unrelated_faces_and_distinct_projections(self):
        bind = rebuild.bind_texture_seam
        for message in ("unexpected source face", "projections left the seam"):
            with self.subTest(message=message):
                def disturb_pod(original, ids, faces, centers, nearest, seam):
                    if message == "unexpected source face":
                        nearest[45] = 109
                    else:
                        centers = centers.copy()
                        centers[45] = original.triangles_center[110]
                    bind(original, ids, faces, centers, nearest, seam)

                with patch.object(rebuild, "bind_texture_seam", side_effect=disturb_pod):
                    with self.assertRaisesRegex(ValueError, message):
                        rebuild.main()

    def test_visual_streams_preserve_both_original_meshes(self):
        _, _, _, _, _, models = prep.collision_inputs()
        derived = prep.prepare()
        for (model, _), filename in zip(models[:2], ("visual.bin","native-visual.bin"), strict=True):
            stream = derived[filename]
            self.assertEqual(stream,(prep.ORIGINAL/filename).read_bytes())
            self.assertEqual(struct.unpack_from("<8sII",stream), (b"SH154VIS", len(model["faces"])*3,56))
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
        self.assertEqual(len(result["originalOverlaps"]), 26)
        self.assertEqual(result["unchangedNativeParts"], 8)
        self.assertEqual(result["recessedNativeWells"], 12)
        self.assertEqual(result["retainedArcs"], 2)
        self.assertEqual(sorted(result["clearanceMillimetres"].values()), [431, 1230, 1230, 1231, 1231, 2585, 3919])
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        source_summary = prep.ORIGINAL / "SOURCE-GEOMETRY.json"
        self.assertEqual(prep.ROOT / criteria["source"], source_summary)
        geometry = json.loads(source_summary.read_bytes())
        self.assertEqual(geometry["id"], "SH154")
        pins = criteria["replayPins"]
        self.assertEqual(pins[criteria["source"]], prep.pin(source_summary.read_bytes())["sha256"])
        candidate_path = prep.ORIGINAL / "GENERATED-COLLISION.json"
        candidate_bytes = candidate_path.read_bytes()
        self.assertEqual(json.loads(candidate_bytes)["id"], "SH154")
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
        self.assertEqual(localized["archiveToCandidate"]["directedMaximumMillimetres"]["upperCeiling"], 702)
        self.assertEqual(localized["candidateToArchive"]["directedMaximumMillimetres"]["upperCeiling"], 593)
        fit=json.loads((prep.INTEGRATION/"SURFACE-FIT.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"],{'detailed': 2791, 'archivedNative': 3632, 'candidateNative': 2570})

    def test_studio_remapping_preserves_original_detail_and_candidate_triangles(self):
        _,inputs,_,_,_,models=prep.collision_inputs()
        studio=json.loads(prep.prepare()[prep.STUDIO_NAME])
        for label,(model,_) in (("detail",models[0]),("native",models[2])):
            delivered=studio[label]
            self.assertEqual([[delivered["vertices"][i] for i in triangle] for triangle in delivered["triangles"]],
                             [[model["vertices"][i] for i in triangle] for triangle in model["faces"]])
        self.assertEqual(set(studio["detail"]["groups"]+studio["native"]["groups"]),set(range(178)))
        finite=json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
        self.assertEqual(studio["engineering"]["massKg"],finite["massKg"])
        self.assertEqual(studio["engineering"]["centerOfMass"],finite["centreMetres"])
        self.assertEqual(sum(studio["engineering"]["inertia"],[]),finite["inertiaAboutCentreOfMassKgM2"])
        self.assertEqual(len(studio["clearances"]),7)
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        signed=criteria["hullStudio"]["signedInstalledExport"]
        self.assertEqual(signed["triangleRoles"]["native"], "derived native inspection candidate")
        self.assertEqual(signed["nativeTriangles"], 512)
        self.assertEqual(criteria["faces"]["archivedNative"], 1384)

    def test_check_replays_without_writing(self):
        with patch.object(prep,"replace_output",side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv",["prepare_osprey_patrol","--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
