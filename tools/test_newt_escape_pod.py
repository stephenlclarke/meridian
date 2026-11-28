"""Original capsule scope, symmetry-only representation, geometry and retained evidence."""

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from prepare_newt_escape_pod_studio import ASSETS, OUTPUT, RAW_TENSOR_OUTPUT, authenticated, definition
from verify_newt_escape_pod import check_group, geometry
from measure_newt_escape_pod_fit import audit


class NewtEscapePodTests(unittest.TestCase):
    def test_original_native_mesh_and_glass_fan_are_preserved(self):
        original,_,_=authenticated(ASSETS); package=definition()
        self.assertEqual(package,json.loads(OUTPUT.read_bytes()))
        detail,native=package['detail'],package['native']
        self.assertEqual(detail['vertices'],original['design']['vertices'])
        self.assertEqual(native['vertices'],original['nativeDesign']['vertices'])
        self.assertEqual(native['triangles'],original['nativeDesign']['faces'])
        self.assertEqual(len(detail['triangles']),416)
        self.assertEqual(detail['triangles'][38:40],[[14,15,27],[14,27,26]])
        self.assertEqual(package['faceMaterials'][38:40],[3,3])
        self.assertEqual(package['materials'][3]['alpha'],64)
        self.assertEqual(package['provenance']['originalGlassOpticalOpacity'],.25)
        self.assertEqual(package['provenance']['objectKind'],'capsule')
        self.assertEqual(package['provenance']['purchasePolicy'],'never')
        self.assertEqual(set(detail['groups']+native['groups']),{0})

    def test_only_mirrored_roundoff_pair_is_normalized(self):
        normalized=definition(); raw=definition(normalize=False)
        self.assertEqual(raw,json.loads(RAW_TENSOR_OUTPUT.read_bytes()))
        self.assertEqual(raw['engineering']['massKg'],248)
        change=normalized['provenance']['inertiaRepresentation']['corrections']
        self.assertEqual(len(change),1)
        self.assertEqual(change[0]['indices'],[[1,2],[2,1]])
        self.assertEqual(change[0]['sourceValues'],[-.41661110632905385,-.4166111063290544])
        self.assertEqual(change[0]['symmetricMean'],-.4166111063290541)
        self.assertEqual(change[0]['changesKgM2'],[-2.7755575615628914e-16,2.7755575615628914e-16])
        expected=deepcopy(raw); expected['engineering']['inertia'][1][2]=expected['engineering']['inertia'][2][1]=change[0]['symmetricMean']
        expected['provenance']['inertiaRepresentation']=normalized['provenance']['inertiaRepresentation']
        self.assertEqual(normalized,expected)

    def test_independent_station_and_raw_validator_receipts(self):
        saved=json.loads(OUTPUT.with_name('CONTACT-RESULT.json').read_bytes())
        self.assertEqual(geometry(),{k:v for k,v in saved.items() if k!='validator'})
        self.assertEqual(saved['station']['aligned']['lowerGapMillimetres'],50273)
        self.assertEqual(saved['station']['offsetOpening']['lowerGapMillimetres'],978)
        self.assertEqual(saved['station']['offsetContact']['stationFace'],482)
        self.assertGreater(min(saved['station']['offsetContact']['barycentrics']),0)
        self.assertEqual([r['exitCode'] for r in saved['validator']['cases']],[1,0,1,1,1,1])
        for case in saved['validator']['cases']:
            input_bytes=(OUTPUT.parent/case['input']).read_bytes(); report_bytes=(OUTPUT.parent/case['report']).read_bytes()
            self.assertEqual(hashlib.sha256(input_bytes).hexdigest(),case['inputSHA256'])
            self.assertEqual(hashlib.sha256(report_bytes).hexdigest(),case['reportSHA256'])
            report=json.loads(report_bytes); check_group(report,json.loads(input_bytes))
            self.assertEqual(report['valid'],case['exitCode']==0)

    def test_exact_containment_and_surface_fit_replay(self):
        saved=json.loads(OUTPUT.with_name('SURFACE-FIT.json').read_bytes()); actual=audit()
        self.assertEqual(actual['packageSHA256'],saved['packageSHA256'])
        self.assertEqual(actual['containment'],saved['containment'])
        self.assertTrue(actual['containment']['allDetailedContained'])
        self.assertEqual(actual['containment']['exactSupportPlanesChecked'],88)
        for domain in ('detailed','native'):
            self.assertEqual(actual['summary'][domain]['largestSample']['certifiedLowerMillimetres'],19)
            self.assertEqual(actual['summary'][domain]['fullUpperMillimetres'],29)

    def test_source_mutation_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder=Path(tmp)/'source'; shutil.copytree(ASSETS,folder)
            p=folder/'native-export.json'; p.write_bytes(p.read_bytes().replace(b'SH040',b'SH041',1))
            with self.assertRaisesRegex(ValueError,'Changed original Newt native-export.json'):
                definition(folder)


if __name__=='__main__': unittest.main()
