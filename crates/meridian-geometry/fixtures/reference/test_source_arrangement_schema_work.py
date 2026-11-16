"""Independent exact storage threshold and restored evidence checks."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
from types import SimpleNamespace
import unittest


def schema_work_suite(kernel, subject, helpers, records):
    refdir=Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/schema-work-reference'
    old=helpers.load('schema_work_exact_770592d',refdir/'before-770592d.py')
    # These describe storage itself; changing the live layout legitimately
    # changes these three diagnostics. Everything else is compared exactly.
    storage_fields={'peakSlots','peakAdmittedMembershipSlots','lastRequiredGeometrySlots'}

    def without_storage_fields(row):
        return {k:without_storage_fields(v) if isinstance(v,dict) else v
                for k,v in row.items() if k not in storage_fields}

    def independent_restore(work):
        schema=work['sourceArrangementPerFacetWorkEncoding']
        assert schema['version']==1
        rows=[]
        for tag,*fields in work['sourceArrangementPerFacetWork']:
            names=list(schema['columns'])
            if schema['tailsByTag'][tag] is not None:
                names.append(schema['tailsByTag'][tag])
            row=dict(zip(names,fields,strict=True))
            row['source']=list(row[schema['sourceAlias']])
            rows.append(row)
        return rows

    class Controls(unittest.TestCase):
        def test_exact_complete_schema_witness_fields_and_work(self):
            pins=json.loads((refdir/'FROZEN.json').read_text())['sha256']
            for name,pin in pins.items():self.assertEqual(hashlib.sha256((refdir/name).read_bytes()).hexdigest(),pin)
            identity=[helpers.box(kernel,(0,0,0,1,1,1),0)]
            overlap=[helpers.box(kernel,bounds,i) for i,bounds in enumerate(((0,0,0,4,4,1),(1,1,0,5,3,1)))]
            overlap=[[kernel.polygon([kernel.Vertex((p.point[0]+p.point[2],p.point[1],p.point[0]+2*p.point[2]),p.uv)
                                      for p in face.vertices],face.source,face.style) for face in shell] for shell in overlap]
            singular=[helpers.box(kernel,(0,0,0,1,1,1),0),helpers.box(kernel,(1,1,1,2,2,2),1)]
            fixture_row=None
            for case,shells,volume in [('identity',identity,F(1)),('oblique_overlap',overlap,F(18)),('singular_vertex',singular,None)]:
                with self.subTest(fixture=case):
                    runs=[];output=[]
                    for module in (old,subject):
                        budget=kernel.Budget();start=time.monotonic()
                        boundary,public=module.surface_union(shells,budget,kernel=kernel)
                        if volume is None:
                            with self.assertRaisesRegex(kernel.UnionFailure,'manifold|disconnected'):
                                kernel.boundary_mesh(boundary,budget)
                            triangles=None
                        else:
                            mesh=kernel.boundary_mesh(boundary,budget)
                            self.assertEqual(F(mesh['exactUnionVolumeSourceUnitsCubed']),volume)
                            triangles=len(mesh['faces'])
                        elapsed=time.monotonic()-start
                        work=budget.union_work
                        encoded=json.loads(json.dumps(work))
                        decoded=(independent_restore(encoded) if module is subject else encoded['sourceArrangementPerFacetWork'])
                        self.assertEqual(decoded,list(public['perSource']))
                        view=public['perSource']
                        options={'default':subject.facet_work_json_default} if module is subject else {}
                        self.assertEqual(decoded,json.loads(json.dumps(view,**options)))
                        if module is subject:
                            self.assertEqual(public['perSourceEncoding'],'read-only-sequence-v1')
                            self.assertEqual(view[0],decoded[0])
                            self.assertEqual(view[-1],decoded[-1])
                            self.assertEqual(view[1:5:2],decoded[1:5:2])
                            self.assertEqual(view[::-1],decoded[::-1])
                            self.assertEqual(decoded,view)
                            self.assertNotEqual(view,[])
                            self.assertFalse(hasattr(view,'append'))
                            with self.assertRaises(IndexError):view[len(view)]
                            with self.assertRaises(TypeError):view[0]=None
                        if case=='oblique_overlap' and module is old:
                            fixture_row=next(row for row in decoded if 'facetRegionWork' in row)
                        output.append((boundary,work['facetRegionWitness'],[without_storage_fields(row) for row in decoded]))
                        runs.append(dict(seconds=elapsed,completeOperations=budget.operations,
                                         workSlots=work['sourceArrangementWorkSlots'],witnessSlots=work['facetRegionWitnessSlots'],
                                         triangles=triangles,sectionWork=work['sectionReuseWork']))
                    self.assertEqual(output[0],output[1])
                    self.assertEqual(runs[0]['completeOperations'],runs[1]['completeOperations'])
                    self.assertEqual(runs[0]['witnessSlots'],runs[1]['witnessSlots'])
                    self.assertEqual(runs[0]['sectionWork'],runs[1]['sectionWork'])
                    self.assertLess(runs[1]['workSlots'],runs[0]['workSlots'])
                    self.assertLess(runs[1]['seconds'],10*runs[0]['seconds'])
                    records.append(dict(fixture=case,kind='schema_work_full_evidence_parity',runs=runs))
            self.assertIsNotNone(fixture_row)
            # Occupied prefix, nonempty new result and complete nested work:
            # the actual publisher must refuse one below the independently
            # counted total, and admit at the exact threshold.
            counters=dict(fixture_row);source=counters.pop('source')
            face=SimpleNamespace(source=tuple(source))
            cached=counters['facetRegionWork']
            for slack in (-1,0):
                with self.subTest(publicationSlack=slack):
                    budget=kernel.Budget()
                    _,work=subject.surface_union(identity,budget,kernel=kernel)
                    ledger=budget.union_work
                    prior_rows=list(ledger['sourceArrangementPerFacetWork'])
                    prior_slots=ledger['sourceArrangementWorkSlots']
                    polygon=identity[0][0];result=[polygon];cells=[polygon,polygon]
                    added=1+1+len(counters)+2*len(cached)
                    limit=ledger['facetRegionWitnessSlots']+prior_slots+added+len(result)+len(cells)+len(cells)+2*len(counters)
                    budget.maximum_polygons=limit+slack
                    if slack < 0:
                        with self.assertRaisesRegex(kernel.UnionFailure,'polygon budget') as caught:
                            subject._retain_facet_work(face,counters,cells,result,budget,kernel.UnionFailure)
                        retained=json.loads(caught.exception.__notes__[0])['unionWork']
                        self.assertEqual(independent_restore(retained),list(work['perSource']))
                        self.assertEqual(ledger['sourceArrangementPerFacetWork'],prior_rows)
                        self.assertEqual(ledger['sourceArrangementWorkSlots'],prior_slots)
                        self.assertEqual(result,[polygon])
                    else:
                        subject._retain_facet_work(face,counters,cells,result,budget,kernel.UnionFailure)
                        self.assertEqual(ledger['sourceArrangementPerFacetWork'][:-1],prior_rows)
                        self.assertEqual(ledger['sourceArrangementWorkSlots'],prior_slots+added)
                        self.assertEqual(independent_restore(ledger)[-1],fixture_row)
                        self.assertEqual(result,[polygon]*3)

    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
