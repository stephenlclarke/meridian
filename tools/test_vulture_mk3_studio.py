"""Original provenance, candidate topology, physical witnesses and execution boundary."""
from collections import Counter
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

import prepare_vulture_mk3_studio as source
import verify_vulture_mk3_station as station
import verify_vulture_mk3_studio as studio


class VultureMk3SourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.streams,cls.outputs=source.prepare()
        cls.original=json.loads(cls.outputs['VULTURE-MK3-ORIGINAL-STUDIO.json'])
        cls.candidate=json.loads(cls.outputs['VULTURE-MK3-CANDIDATE-STUDIO.json'])
        cls.derived=json.loads(cls.streams['DERIVED-NATIVE.json'])
        cls.witness=json.loads(cls.outputs['CONTACT-WITNESSES.json'])

    def test_exact_source_streams_and_candidate_reproduce(self):
        for directory,files in ((source.ASSETS,self.streams),(source.INTEGRATION,self.outputs)):
            for name,value in files.items():
                with self.subTest(name=name):self.assertEqual((directory/name).read_bytes(),value)
        r,exact,_=source.authenticate(source.ASSETS)
        self.assertEqual(self.original['detail']['vertices'],r['design']['vertices'])
        self.assertEqual(self.original['detail']['triangles'],r['design']['faces'])
        self.assertEqual(self.original['native']['vertices'],r['nativeDesign']['vertices'])
        self.assertEqual(self.original['native']['triangles'],r['nativeDesign']['faces'])
        p=source.calibrated(exact['design']['vertices'])+source.calibrated(exact['nativeDesign']['vertices'])
        self.assertEqual(list(struct.iter_unpack('<3d',self.streams['source.vertices'][16:])),list(map(tuple,p)))

    def test_relative_scale_and_original_cap_are_preserved_not_reclassified(self):
        self.assertEqual(str(source.SCALE),'6000/611')
        self.assertEqual(self.original['metresPerUnit'],float(source.SCALE))
        self.assertEqual(len(self.original['native']['triangles']),3016)
        self.assertEqual(len(self.candidate['native']['triangles']),508)
        self.assertEqual(self.original['detail'],self.candidate['detail'])
        self.assertEqual(self.original['id'],'SH039')
        self.assertEqual(self.candidate['id'],'SH039-DERIVED-NATIVE-508')
        self.assertIsNone(self.candidate['engineering'])
        self.assertIsNone(self.original['engineering'])
        self.assertEqual(self.candidate['flight'],{'maxSpeed':52.5})
        self.assertEqual(len(source.NAMES),177)

    def test_candidate_uses_only_archived_points_with_closed_consistent_winding(self):
        original=self.original['native']; native=self.candidate['native']; selected=self.derived['sourceVertexIndices']
        self.assertEqual(len(set(selected)),274)
        self.assertEqual(native['vertices'],[original['vertices'][i] for i in selected])
        self.assertEqual(native['groups'],[original['groups'][i] for i in selected])
        self.assertEqual(set(v for face in native['triangles'] for v in face),set(range(274)))
        directed=Counter((a,b) for face in native['triangles'] for a,b in zip(face,face[1:]+face[:1]))
        self.assertTrue(all(count==directed[b,a]==1 for (a,b),count in directed.items()))
        self.assertTrue(all(len({native['groups'][i] for i in face})==1 for face in native['triangles']))
        self.assertEqual(len(self.derived['parts']),13)

    def test_original_and_candidate_actual_source_contact_witnesses(self):
        self.assertEqual([p['originalNetMetres'] for p in self.witness['openings']],[.271,.271,.173])
        self.assertEqual([p['candidateNetMetres'] for p in self.witness['openings']],[.271,.271,.173])
        self.assertEqual([(row['detailed']['face'],row['archivedNative']['face'],row['derivedNative']['face']) for row in self.witness['contacts']],[(89,53,29),(2671,1287,329),(5919,2331,425)])
        for row in self.witness['contacts']:
            for key in ('detailed','archivedNative','derivedNative'):
                self.assertGreater(min(row[key]['barycentric']),0)

    def test_tampered_original_bytes_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)
            for name in source.PINS:shutil.copyfile(source.ASSETS/name,path/name)
            file=path/'native-export.json'; file.write_bytes(file.read_bytes()+b' ')
            with self.assertRaisesRegex(ValueError,'Changed original SH039 native-export.json'):source.authenticate(path)

    def test_station_fixed_yaw_crossing_and_canonical_redirected_bytes(self):
        report=station.verify(); expected=(source.INTEGRATION/'STATION-RESULT.json').read_bytes()
        self.assertEqual(report,json.loads(expected))
        self.assertEqual([v['aligned']['netMillimetres'] for v in report['unions'].values()],[43099,43099])
        self.assertEqual([v['offsetStart']['netMillimetres'] for v in report['unions'].values()],[1420,1420])
        hit=report['lipCrossing']; self.assertEqual((hit['archivedNativeVertex'],hit['derivedNativeVertex'],hit['stationFace']),(120,72,482)); self.assertGreater(min(hit['barycentrics']),0)
        with BytesIO() as raw, TextIOWrapper(raw,encoding='utf-8',newline='\r\n') as stdout, redirect_stdout(stdout),patch.object(station,'verify',return_value=report):
            runpy.run_module('verify_vulture_mk3_station',run_name='__main__'); stdout.flush(); self.assertEqual(raw.getvalue(),expected)
        self.assertEqual(subprocess.check_output([sys.executable,'-B',str(source.ROOT/'tools/verify_vulture_mk3_station.py')]),expected)

    def test_studio_execution_inputs_distinguish_original_and_candidate_controls(self):
        original,candidate,negative,witness=studio.inputs()
        self.assertEqual(json.loads(original),self.original); self.assertEqual(json.loads(candidate),self.candidate)
        self.assertEqual(negative['native'],self.candidate['native'])
        self.assertEqual(negative['clearances'],self.candidate['clearances']+[row['route'] for row in witness['contacts']])

    def test_generator_cli_and_changed_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            assets=Path(temp)/'assets'; integration=Path(temp)/'integration'
            with patch.object(source,'ASSETS',assets),patch.object(source,'INTEGRATION',integration),patch.object(source,'prepare',return_value=(self.streams,self.outputs)):
                with patch('sys.argv',['prepare']):source.main()
                with patch('sys.argv',['prepare','--check']):
                    source.main()
                    (assets/'PIECES.json').write_bytes(b'changed')
                    with self.assertRaisesRegex(ValueError,'Changed generated SH039 PIECES.json'):source.main()

    def test_execution_report_boundary_refuses_changed_source_or_contacts(self):
        groups={}
        for key in ('detail','native'):
            mesh=self.candidate[key]
            for p,g in zip(mesh['vertices'],mesh['groups'],strict=True):groups.setdefault(g,[]).append([v*self.candidate['metresPerUnit'] for v in p])
        support=[{'id':g,'points':groups[g],'roundingMetres':.02} for g in sorted(groups)]
        openings=[{'name':v['route']['name'],'passed':True} for v in self.witness['openings']]
        contacts=[{'name':v['route']['name'],'passed':False} for v in self.witness['contacts']]
        positive={'valid':True,'supportGroups':support,'diagnostics':[],'clearances':openings}
        blocked={'valid':False,'supportGroups':support,'diagnostics':[{'message':f'Clearance {v["name"]} is blocked or indeterminate'} for v in contacts],'clearances':openings+contacts}
        original={'valid':False,'supportGroups':[],'diagnostics':[{'message':'native: invalid vertex, triangle or group cardinality (face limit 512)'}]}
        def fake(args,stdout,stderr,check):
            if '--export' in args:
                Path(args[args.index('--export')+1]).write_bytes(self.outputs['VULTURE-MK3-CANDIDATE-STUDIO.json'].rstrip(b'\n'))
                stdout.write(source.encode(positive)); return subprocess.CompletedProcess(args,0)
            path=Path(args[args.index('--report')+1]); path.write_bytes(source.encode(original if 'ORIGINAL' in path.name else blocked)); return subprocess.CompletedProcess(args,1)
        with tempfile.TemporaryDirectory() as temp:
            folder=Path(temp)/'evidence'
            with patch.object(studio.subprocess,'run',side_effect=fake):studio.execute(Path('studio'),Path('validator'),folder)
            self.assertEqual(studio.compare(folder)['actualSourcePointCount'],5406)
            with BytesIO() as raw, TextIOWrapper(raw,encoding='utf-8',newline='\r\n') as stdout, redirect_stdout(stdout),patch('sys.argv',['verify','--check','--output',str(folder)]):
                studio.main(); stdout.flush(); self.assertNotIn(b'\r\n',raw.getvalue())
            blocked['clearances'][-1]['passed']=True
            (folder/'CANDIDATE-CONTACT-REPORT.json').write_bytes(source.encode(blocked))
            with self.assertRaisesRegex(ValueError,'Changed opening/solid'):studio.compare(folder)


if __name__=='__main__':unittest.main()
