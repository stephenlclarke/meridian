"""Check exact original Aurochs references, local supports, and offline inspection streams."""
import hashlib
import json
import struct
import unittest
from unittest.mock import patch

import prepare_aurochs as prep
import verify_aurochs as proof


class AurochsSources(unittest.TestCase):
    def test_pinned_originals_and_distinct_candidate(self):
        provenance, inputs, _, models, parts = prep.load_source()
        for name, data in inputs.items():
            with self.subTest(path=name):
                record = provenance["paths"][name]
                self.assertEqual(prep.pin(data), {key: record[key] for key in ("bytes", "sha256")})
                self.assertEqual(record["blob"], hashlib.sha1(f"blob {len(data)}\0".encode()+data).hexdigest())
        self.assertEqual([len(mesh["faces"]) for mesh in models], [2504, 720, 444])
        self.assertEqual([len(records) for records in parts], [110, 18, 18])
        self.assertEqual(str(prep.SCALE), "40000/4221")

    def test_visual_streams_preserve_both_original_meshes(self):
        _, _, _, _, _, models = prep.collision_inputs()
        derived = prep.prepare()
        for (model, _), filename in zip(models[:2], ("visual.bin","native-visual.bin"), strict=True):
            stream = derived[filename]
            self.assertEqual(stream,(prep.ORIGINAL/filename).read_bytes())
            self.assertEqual(struct.unpack_from("<8sII",stream), (b"AUROVIZ1", len(model["faces"])*3,56))
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
        self.assertEqual(len(result["originalOverlaps"]),12)
        self.assertEqual(result["nativeCargoCapsuleVerticalSeparationMillimetres"], 284)
        self.assertEqual(sorted(result["clearanceMillimetres"].values()),[772,779,822,1275,1355,1355,1373])
        criteria=json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        for name,expected in criteria["replayPins"].items():
            with self.subTest(path=name):
                self.assertEqual(prep.pin((prep.ROOT/name).read_bytes())["sha256"],expected)
        fit=json.loads((prep.INTEGRATION/"SURFACE-FIT.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"],{"detailed":2524,"archivedNative":2524,"candidateNative":2882})

    def test_studio_preserves_original_detail_and_candidate_triangles(self):
        _,inputs,_,_,_,models=prep.collision_inputs()
        studio=json.loads(prep.prepare()[prep.STUDIO_NAME])
        for label,(model,_) in (("detail",models[0]),("native",models[2])):
            delivered=studio[label]
            self.assertEqual([[delivered["vertices"][i] for i in triangle] for triangle in delivered["triangles"]],
                             [[model["vertices"][i] for i in triangle] for triangle in model["faces"]])
        self.assertEqual(set(studio["detail"]["groups"]+studio["native"]["groups"]),set(range(38)))
        finite=json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
        self.assertEqual(studio["engineering"]["massKg"],finite["massKg"])
        self.assertEqual(studio["engineering"]["centerOfMass"],finite["centreMetres"])
        self.assertEqual(sum(studio["engineering"]["inertia"],[]),finite["inertiaAboutCentreOfMassKgM2"])
        self.assertEqual(len(studio["clearances"]),7)
        criteria = json.loads((prep.INTEGRATION/"CRITERIA.json").read_bytes())
        signed = criteria["hullStudio"]["signedInstalledExport"]
        self.assertEqual(signed["triangleRoles"]["native"], "derived native inspection candidate")
        self.assertEqual(signed["nativeTriangles"], len(studio["native"]["triangles"]))
        self.assertEqual(criteria["faces"]["archivedNative"], len(models[1][0]["faces"]))

    def test_check_replays_without_writing(self):
        with patch.object(prep,"replace_output",side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv",["prepare_aurochs","--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
