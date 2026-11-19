"""Independent SH039 surface and outside-coverage certificates."""
from contextlib import redirect_stdout
from io import BytesIO, TextIOWrapper
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

import measure_vulture_mk3_fit as measure
import verify_vulture_mk3_shield_blocker as shield


class VultureMk3FitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.meshes,cls.unions=measure.load()

    def test_original_and_derived_are_independent_inputs(self):
        self.assertEqual([len(mesh.faces) for mesh in self.meshes.values()],[8272,3016,508])
        self.assertEqual([len(set(ids)) for _,ids in self.unions.values()],[177,169])
        self.assertEqual([len(points) for points,_ in self.unions.values()],[6944,5406])

    def test_tampered_source_or_manifest_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            folder=Path(temp)/'assets'; shutil.copytree(measure.ASSETS,folder)
            manifest=folder/'SOURCE-GEOMETRY.json'; original=manifest.read_bytes(); manifest.write_bytes(original+b' ')
            with self.assertRaisesRegex(ValueError,'Changed SH039 source manifest'):measure.load(folder)
            manifest.write_bytes(original); binary=folder/'source-triangles.bin'; raw=binary.read_bytes(); binary.write_bytes(raw[:-1]+bytes([raw[-1]^1]))
            with self.assertRaisesRegex(ValueError,'Changed SH039 fit input source-triangles.bin'):measure.load(folder)

    def test_four_direct_surface_certificates_bound_full_triangles(self):
        result=measure.direct_fit(self.meshes)
        controls={'detailedToderivedNative':(1150,1210),'derivedNativeTodetailed':(770,940),'archivedNativeToderivedNative':(1160,1220),'derivedNativeToarchivedNative':(630,950)}
        self.assertEqual(set(result),set(controls))
        for name,(lower,upper) in controls.items():
            self.assertGreaterEqual(result[name]['witness']['strictLowerMillimetres'],lower)
            self.assertLessEqual(result[name]['fullUpperMillimetres'],upper)
            self.assertGreaterEqual(result[name]['fullUpperMillimetres'],result[name]['witness']['strictLowerMillimetres'])

    def test_original_and_candidate_rounded_union_fit(self):
        original={key:self.meshes[key] for key in ('detailed','archivedNative')}
        for name,(points,ids) in self.unions.items():
            result=measure.union_fit(points,ids,original)
            self.assertEqual(result['groups'],177 if name=='original' else 169)
            for key,lower,upper in (('detailed',195,3500),('archivedNative',335,3470)):
                row=result['summary'][key]
                self.assertGreaterEqual(row['largestCertifiedExposedSample']['certifiedLowerMillimetres'],lower)
                self.assertLessEqual(row['fullUpperMillimetres'],upper)
                self.assertEqual(row['fullUpperMillimetres'],max(piece['comparisons'][key]['fullUpperMillimetres'] for piece in result['pieces']))

    def test_archived_native_collision_coverage_is_not_claimed(self):
        witness=measure.candidate_coverage_witness(self.meshes,self.unions)
        self.assertTrue(witness['certifiedOutsideWitness'])
        self.assertGreaterEqual(witness['strictLowerMillimetresFromRoundedCandidateUnion'],120)

    def test_no_regrouping_can_enclose_original_native_surface_interiors(self):
        report=shield.blocker(self.meshes,self.unions)
        self.assertFalse(report['archivedOriginalCoverage'])
        self.assertEqual(report['detailedSourceFacesContainedByOwnConvexGroups'],8272)
        self.assertGreaterEqual(report['certifiedOutsideGlobalRoundedHullVertexCount'],100)
        self.assertGreaterEqual(report['strongestWitness']['strictOutsideLowerMillimetres'],90)
        self.assertEqual([r['nativeFace'] for r in report['strictOriginalTriangleInteriorWitnesses']],[1427,2471])
        self.assertTrue(all(min(r['barycentricNumeratorsOver1000'])>0 and r['exactOutsideLowerMillimetres']==17 for r in report['strictOriginalTriangleInteriorWitnesses']))
        self.assertTrue(report['completePanelAdditionWouldExceedCap'])
        self.assertEqual(report['remainingFaces'],4)

    def test_cli_emits_lf_even_with_host_crlf_translation(self):
        cases=(([],'direct_fit'),(['--union','candidate'],'union_fit'),(['--union','original','--samples-only'],'union_fit'),(['--coverage-gap'],'candidate_coverage_witness'))
        for flags,operation in cases:
            with self.subTest(flags=flags),patch('sys.argv',['measure']+flags),patch.object(measure,'load',return_value=(self.meshes,self.unions)),patch.object(measure,operation,return_value={'passed':True}),BytesIO() as raw,TextIOWrapper(raw,encoding='utf-8',newline='\r\n') as out,redirect_stdout(out):
                measure.main(); out.flush(); self.assertNotIn(b'\r\n',raw.getvalue()); self.assertTrue(json.loads(raw.getvalue()))
        with patch.object(measure.fit,'audit_fan',side_effect=AssertionError('invalid fan')),self.assertRaisesRegex(ValueError,'radial fan not certified'):
            measure.make_pieces(np.zeros((4,3)),np.zeros(4,dtype=int))


if __name__=='__main__':unittest.main()
