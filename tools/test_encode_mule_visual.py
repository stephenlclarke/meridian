"""Offline source fields/pixels only, never a hull algorithm, shader or GPU."""
import hashlib
import json
from pathlib import Path
import struct
import tomllib
import unittest
import encode_mule_visual as encoder

class SourceEncoding(unittest.TestCase):
    def test_all_faces_preserve_source_corner_order_material_uv_layer_and_flags(self):
        folder=encoder.ROOT/'assets/fleet/sh001';source=(folder/'resource.toml').read_bytes()
        actual=(folder/'visual.bin').read_bytes();self.assertEqual(encoder.encode(source),actual)
        model=tomllib.loads(source.decode())['design'];offset=16
        original=tomllib.loads((folder/'original-design.toml').read_text())['design']
        self.assertEqual(model, original)
        self.assertEqual(
            [texture['asset'] for texture in original['textures']],
            ['l337-patrol-diffuse', 'l337-strike-diffuse', 'l337-service-details'],
        )
        faces_per_layer = [0, 0, 0]
        for face,material_index,texture in zip(model['faces'],model['faceMaterials'],model['faceTextures'],strict=True):
            faces_per_layer[texture['texture']] += 1
            material=model['materials'][material_index]
            for index,uv in zip(face,texture['uv'],strict=True):
                values=struct.unpack_from('<12fII',actual,offset);offset+=56
                expected=struct.unpack('<3f',struct.pack('<3f',*(v*40000/3411 for v in model['vertices'][index])))
                self.assertEqual(values[:3],expected)
                rgba=[(material['colour']>>shift&255)/255 for shift in (24,16,8,0)]
                self.assertEqual(values[6:10],struct.unpack('<4f',struct.pack('<4f',*rgba)))
                self.assertEqual(values[10:12],struct.unpack('<2f',struct.pack('<2f',*uv)))
                self.assertEqual(values[12:],(texture['texture'],int(material['emissive'])))
        self.assertEqual(offset,len(actual));self.assertEqual(offset,16+7236*3*56)
        self.assertEqual(faces_per_layer, [452, 0, 6784])

    def test_three_original_rgba_maps_and_licenses_match_immutable_manifest(self):
        folder=encoder.ROOT/'assets/fleet/sh001';raw=(folder/'TEXTURE-SOURCE.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),'96c4c9699b81f50be5a8aefeeca74a6e4870e0bb981d1cd33a9ff2921cd2bd3c')
        source=json.loads(raw);self.assertEqual(len(source['assets']),3)
        for asset in source['assets']:
            data=(folder/asset['rgbaFile']).read_bytes();self.assertEqual(len(data),4194304)
            self.assertEqual(hashlib.sha256(data).hexdigest(),asset['rgbaSHA256']);self.assertEqual(asset['license'],'MIT')
            self.assertEqual(data[3::4],b'\xff'*(1024*1024))
        self.assertTrue((folder/'LICENSE').is_file())

    def test_changed_resource_refuses_instead_of_encoding_unbound_geometry(self):
        source=(encoder.ROOT/'assets/fleet/sh001/resource.toml').read_bytes()
        with self.assertRaises(ValueError):encoder.encode(source.replace(b'Mule',b'MULE',1))

if __name__=='__main__':unittest.main()
