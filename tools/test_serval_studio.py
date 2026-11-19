"""Original SH041 data, visual identity, contact and Studio boundary regressions."""
from contextlib import redirect_stdout
from io import BytesIO, TextIOWrapper
import json
from pathlib import Path
import runpy
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import prepare_serval_studio as generator
import verify_serval_station as station
import verify_serval_studio as studio


class ServalSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.streams,cls.outputs=generator.prepare(); cls.definition=json.loads(cls.outputs['SERVAL-STUDIO.json']); cls.witness=json.loads(cls.outputs['CONTACT-WITNESSES.json']); cls.resource,_=generator.authenticate(generator.ASSETS)

    def test_exact_frozen_source_and_all_generated_outputs(self):
        for folder,files in ((generator.ASSETS,self.streams),(generator.INTEGRATION,self.outputs)):
            for name,data in files.items():
                with self.subTest(name=name):self.assertEqual((folder/name).read_bytes(),data)
        for key,legacy in (('detail','design'),('native','nativeDesign')):
            mesh=self.definition[key]; self.assertEqual(mesh['vertices'],self.resource[legacy]['vertices']); self.assertEqual(mesh['triangles'],self.resource[legacy]['faces']); self.assertTrue(all(len({mesh['groups'][i] for i in face})==1 for face in mesh['triangles']))
        self.assertEqual(self.definition['metresPerUnit'],1); self.assertIsNone(self.definition['engineering']); self.assertEqual(self.definition['flight'],{'maxSpeed':43.6}); self.assertEqual(len(generator.NAMES),23)
        points=self.definition['detail']['vertices']+self.definition['native']['vertices']; self.assertEqual(list(struct.iter_unpack('<3d',self.streams['source.vertices'][16:])),list(map(tuple,points)))

    def test_both_packed_visual_streams_keep_materials_uvs_and_positions(self):
        for file,source in (('visual.bin','design'),('native-visual.bin','nativeDesign')):
            mesh=self.resource[source]; packed=self.streams[file]; self.assertEqual(packed[:16],struct.pack('<8sII',b'SERVVIZ1',len(mesh['faces'])*3,56)); corners=list(struct.iter_unpack('<12fII',packed[16:]))
            for i,(face,material,mapping) in enumerate(zip(mesh['faces'],mesh['faceMaterials'],mesh['faceTextures'],strict=True)):
                mat=mesh['materials'][material]
                for j,v in enumerate(face):
                    row=corners[3*i+j]; self.assertEqual(struct.pack('<3f',*row[:3]),struct.pack('<3f',*mesh['vertices'][v])); self.assertEqual(struct.pack('<4f',*row[6:10]),struct.pack('<4f',*((mat['colour']>>shift&255)/255 for shift in (24,16,8,0)))); self.assertEqual(struct.pack('<2f',*row[10:12]),struct.pack('<2f',*mapping['uv'][j])); self.assertEqual(row[12:],(mapping['texture']&0xffffffff,int(mat['emissive'])))

    def test_openings_and_contact_original_triangle_interiors(self):
        self.assertEqual([r['conservativeNetClearanceMetres'] for r in self.witness['openings']],[.079,.079,.179]); self.assertEqual([(r['detailed']['face'],r['archivedNative']['face']) for r in self.witness['contacts']],[(168,37),(2157,149),(831,79)])
        for row in self.witness['contacts']:
            self.assertGreater(min(row['detailed']['barycentric']),0);self.assertGreater(min(row['archivedNative']['barycentric']),0)

    def test_wrong_original_bytes_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder=Path(temporary)
            for name in generator.PINS:shutil.copyfile(generator.ASSETS/name,folder/name)
            file=folder/'native-export.json'; file.write_bytes(file.read_bytes()+b' ')
            with self.assertRaisesRegex(ValueError,'Changed original SH041 native-export.json'):generator.authenticate(folder)

    def test_generator_cli_checks_and_refuses_changed_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            asset=Path(temporary)/'assets'; integration=Path(temporary)/'integration'
            with patch.object(generator,'ASSETS',asset),patch.object(generator,'INTEGRATION',integration),patch.object(generator,'prepare',return_value=(self.streams,self.outputs)):
                with patch('sys.argv',['prepare']):generator.main()
                with patch('sys.argv',['prepare','--check']):
                    generator.main(); (asset/'PIECES.json').write_bytes(b'changed')
                    with self.assertRaisesRegex(ValueError,'Changed generated SH041 PIECES.json'):generator.main()

    def test_station_exact_sweep_and_binary_redirected_output(self):
        report=station.verify(); expected=(generator.INTEGRATION/'STATION-RESULT.json').read_bytes(); self.assertEqual(report,json.loads(expected)); self.assertEqual(report['aligned']['netMillimetres'],45138); self.assertEqual(report['offsetStart']['netMillimetres'],1159)
        hit=report['nativeLipCrossing'];self.assertEqual((hit['nativeLocalVertex'],hit['nativeGlobalVertex'],hit['stationFace']),(26,2198,482));self.assertGreater(min(hit['barycentric']),0)
        with BytesIO() as raw,TextIOWrapper(raw,encoding='utf-8',newline='\r\n') as out,redirect_stdout(out):
            runpy.run_module('verify_serval_station',run_name='__main__');out.flush();self.assertEqual(raw.getvalue(),expected)
        self.assertEqual(subprocess.check_output([sys.executable,'-B',str(generator.ROOT/'tools/verify_serval_station.py')]),expected)

    def test_studio_boundary_replay_and_original_support_check(self):
        groups={}
        for key in ('detail','native'):
            mesh=self.definition[key]
            for p,g in zip(mesh['vertices'],mesh['groups'],strict=True):groups.setdefault(g,[]).append(p)
        support=[{'id':g,'points':groups[g],'roundingMetres':.02} for g in sorted(groups)]; opens=[{'name':r['route']['name'],'passed':True} for r in self.witness['openings']]; contacts=[{'name':r['route']['name'],'passed':False} for r in self.witness['contacts']]
        positive={'valid':True,'supportGroups':support,'diagnostics':[],'clearances':opens}; blocked={'valid':False,'supportGroups':support,'diagnostics':[{'message':f'Clearance {r["name"]} is blocked or indeterminate'} for r in contacts],'clearances':opens+contacts}
        def fake(args,stdout,stderr,check):
            if '--export' in args:
                Path(args[args.index('--export')+1]).write_bytes(self.outputs['SERVAL-STUDIO.json'].rstrip(b'\n'));stdout.write(generator.encode(positive));return subprocess.CompletedProcess(args,0)
            Path(args[args.index('--report')+1]).write_bytes(generator.encode(blocked));return subprocess.CompletedProcess(args,1)
        with tempfile.TemporaryDirectory() as temporary:
            folder=Path(temporary)/'evidence'
            with patch.object(studio.subprocess,'run',side_effect=fake):studio.execute(Path('studio'),Path('validator'),folder)
            self.assertEqual(len(studio.compare(folder)),4)
            with BytesIO() as raw,TextIOWrapper(raw,encoding='utf-8',newline='\r\n') as out,redirect_stdout(out),patch('sys.argv',['verify','--check','--output',str(folder)]):
                studio.main();out.flush();self.assertNotIn(b'\r\n',raw.getvalue())
            blocked['clearances'][-1]['passed']=True;(folder/'BLOCKED-REPORT.json').write_bytes(generator.encode(blocked))
            with self.assertRaisesRegex(ValueError,'Opening/solid'):studio.compare(folder)


if __name__=='__main__':unittest.main()
