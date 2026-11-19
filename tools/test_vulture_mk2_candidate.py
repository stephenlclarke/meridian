"""Source identity, exact archive refusal, and independent derived hull evidence."""

import hashlib
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from prepare_vulture_mk2_source import ASSETS, OUTPUT, authenticated, definition as archival
from prepare_vulture_mk2_candidate import CANDIDATE, NATIVE_PARTS, definition
from verify_vulture_mk2_candidate import verify


class VultureMk2Tests(unittest.TestCase):
    def test_exact_originals_remain_distinct_and_refused(self):
        source, engineering, _ = authenticated(ASSETS)
        package = archival()
        self.assertEqual(package, json.loads(OUTPUT.read_bytes()))
        for domain, original in [('detail','design'),('native','nativeDesign')]:
            self.assertEqual(package[domain]['vertices'],source[original]['vertices'])
            self.assertEqual(package[domain]['triangles'],source[original]['faces'])
        self.assertEqual(len(package['native']['triangles']),3016)
        self.assertEqual(package['engineering']['massKg'],engineering['dryMassKilograms'])
        self.assertEqual(package['engineering']['massKg'],57625)
        receipt=json.loads(OUTPUT.with_name('ORIGINAL-REFUSAL.json').read_bytes())
        self.assertEqual(receipt['packageSHA256'],hashlib.sha256(OUTPUT.read_bytes()).hexdigest())
        self.assertEqual(receipt['exitCode'],1)
        self.assertFalse(receipt['report']['valid'])
        self.assertIn('face limit 512',receipt['report']['diagnostics'][0]['message'])

    def test_derived_is_original_detail_plus_separate_native_approximation(self):
        source,_,_ = authenticated(ASSETS)
        package=definition(); self.assertEqual(package,json.loads(CANDIDATE.read_bytes()))
        self.assertTrue(package['provenance']['derivedNativeApproximation'])
        self.assertEqual(package['provenance']['originalArchiveSHA256'],hashlib.sha256(OUTPUT.read_bytes()).hexdigest())
        self.assertEqual((len(package['native']['vertices']),len(package['native']['triangles'])),(324,512))
        self.assertEqual(len(package['provenance']['groupNames']),522)
        self.assertEqual(package['engineering'],archival()['engineering'])
        self.assertEqual(package['faceMaterials'],source['design']['faceMaterials'])
        detail=package['detail']; original=source['design']
        self.assertEqual(len(detail['triangles']),8272)
        for a,b in zip(detail['triangles'],original['faces'],strict=True):
            self.assertEqual([detail['vertices'][i] for i in a],[original['vertices'][i] for i in b])
        native=package['native']; original_points=set(map(tuple,source['nativeDesign']['vertices']))
        self.assertTrue(set(map(tuple,native['vertices'])).issubset(original_points))
        names=package['provenance']['groupNames']
        self.assertEqual({int(names[i].split('-')[1]) for i in native['groups']},set(NATIVE_PARTS.values()))
        for domain in (detail,native):
            self.assertTrue(all(len({domain['groups'][i] for i in f})==1 for f in domain['triangles']))

    def test_contact_certificates_and_retained_actual_validator(self):
        computed=verify(); actual=json.loads(CANDIDATE.with_name('CONTACT-RESULT.json').read_bytes())
        self.assertEqual(computed,{k:v for k,v in actual.items() if k!='validator'})
        self.assertEqual(len(computed['openRoutes']),4)
        self.assertTrue(all(r['lowerBoundMetres']>0 for r in computed['openRoutes']))
        self.assertEqual(computed['station']['aligned']['minimumSeparationMillimetres'],43865)
        self.assertEqual(computed['station']['offsetOpening']['minimumSeparationMillimetres'],979)
        self.assertEqual(computed['station']['offsetContact']['face'],482)
        self.assertGreater(min(computed['station']['offsetContact']['barycentrics']),0)
        self.assertEqual([r['exitCode'] for r in actual['validator']['cases']],[0,1,1,1,1])

    def test_fit_is_separate_and_bound_to_both_originals(self):
        report=json.loads(CANDIDATE.with_name('SURFACE-FIT.json').read_bytes())
        self.assertEqual(report['candidateSHA256'],hashlib.sha256(CANDIDATE.read_bytes()).hexdigest())
        self.assertEqual(len(report['perGroup']),522)
        for domain in ('detailed','native'):
            summary=report['summary'][domain]
            self.assertLess(summary['largestSample']['certifiedLowerMillimetres'],summary['fullUpperMillimetres'])
            for row in ('derivedNativeToOriginal','originalToDerivedNative'):
                compare=report['derivedNativeComparison'][domain][row]
                self.assertLessEqual(compare['certifiedLowerMillimetres'],compare['fullSurfaceUpperMillimetres'])

    def test_mutated_original_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            folder=Path(temp)/'original'; shutil.copytree(ASSETS,folder)
            p=folder/'native-export.json'; p.write_bytes(p.read_bytes().replace(b'SH038',b'SH037',1))
            with self.assertRaisesRegex(ValueError,'Changed SH038 source native-export.json'):
                archival(folder)


if __name__=='__main__': unittest.main()
