"""Independent source-backed local recovery and production-refusal witnesses."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
import unittest


def recovery_suite(kernel, subject, helpers, records):
    refdir=Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/local-recovery-reference'
    frozen=json.loads((refdir/'cases.json').read_text())
    old=helpers.load('recovery_frozen_c049',refdir/'before-c049-recovery.py')
    oracle=helpers.load('recovery_independent_oracle',refdir.parent/'reference/independent_planar_reference.py')

    def fixture(case,module):
        points=[tuple(map(F,p)) for p in case['points']]
        domain=[points[i] for i in case['domain']]
        vertices=[(x,y,F(-1)) for x,y in domain]+[(x,y,F(0)) for x,y in domain]
        faces=[[0,2,1],[3,4,5],[0,1,4],[0,4,3],[1,2,5],[1,5,4],[2,0,3],[2,3,5]]
        textures=[dict(index=0,uv=[[vertices[i][0]+2*vertices[i][1],vertices[i][2]-vertices[i][1]] for i in f]) for f in faces]
        shell=kernel.input_shell(vertices,faces,[0]*8,textures,0)
        budget=kernel.Budget();arr=module.PlanarArrangement(domain,budget,kernel.UnionFailure)
        arr.work['sourceFacet']=[0,1];arr.region_axes=(0,1)
        a,b=[points[i] for i in case['target']]
        barriers=[(p,p) for p in points]+[(a,b)]+[(points[x],points[y]) for x,y in case.get('locked',[])]
        cells=arr.cells(barriers,lambda p:True)
        region=module.FacetRegions(arr,cells)
        mesh=[tuple(points[i] for i in tri) for tri in case['mesh']]
        region.triangles=list(mesh)
        region.locked={region.edge(points[x],points[y]) for x,y in case.get('locked',[])}
        return points,shell,budget,arr,region,mesh,a,b

    def verify_mesh(test,mesh,expected,domain):
        test.assertEqual(sum(oracle.twice_area(t) for t in mesh),expected)
        for i,t in enumerate(mesh):
            test.assertGreater(oracle.twice_area(t),0)
            test.assertTrue(all(oracle.orient(a,b,p)>=0 for p in t for a,b in zip(domain,domain[1:]+domain[:1])))
            for u in mesh[i+1:]:test.assertEqual(oracle.twice_area(oracle.intersection(list(t),u)),0)

    class Controls(unittest.TestCase):
        def test_frozen_source_stars_old_refusal_new_vertex_preserving_recovery(self):
            manifest=json.loads((refdir/'FROZEN.json').read_text())
            for name,pin in manifest['files'].items():
                data=(refdir/name).read_bytes();self.assertEqual(len(data),pin['bytes']);self.assertEqual(hashlib.sha256(data).hexdigest(),pin['sha256'])
            for case in frozen['cases']:
                with self.subTest(case=case['id']):
                    points,shell,budget,arr,baseline,mesh,a,b=fixture(case,old)
                    domain=[points[i] for i in case['domain']]
                    verify_mesh(self,mesh,F(case['twiceArea']),domain)
                    start=time.monotonic()
                    with self.assertRaisesRegex(kernel.UnionFailure,case['expectedOldRefusal']) as caught:baseline.recover(a,b)
                    old_seconds=time.monotonic()-start
                    self.assertEqual(len(getattr(caught.exception,'__notes__',[])),0)
                    points,shell,budget,arr,region,mesh,a,b=fixture(case,subject)
                    locked=set(region.locked);start=time.monotonic();region.recover(a,b)
                    self.assertEqual(len(region.triangles),case['triangles'])
                    self.assertEqual(set(p for t in region.triangles for p in t),set(points))
                    edges={region.edge(x,y) for t in region.triangles for x,y in zip(t,t[1:]+t[:1])}
                    self.assertIn(region.edge(a,b),edges);self.assertTrue(locked<=edges)
                    verify_mesh(self,region.triangles,F(case['twiceArea']),domain)
                    common=sum(oracle.twice_area(oracle.intersection(list(t),u)) for t in mesh for u in region.triangles)
                    self.assertEqual(common,F(case['twiceArea']))
                    source=shell[1];boundary=[f for f in shell if f.source!=source.source]
                    for triangle in region.triangles:
                        vertices=[]
                        for x,y in triangle:
                            p=(x,y,F(0));uv=kernel.source_uv(source,p)
                            self.assertEqual(uv,(x+2*y,-y));vertices.append(kernel.Vertex(p,uv))
                        boundary.append(kernel.polygon(vertices,source.source,source.style))
                    kernel.audit_source_fragments(boundary,shell,budget)
                    kernel.audit_boundary_appearance(boundary,shell,budget)
                    final=kernel.boundary_mesh(boundary,budget)
                    self.assertEqual(F(final['exactUnionVolumeSourceUnitsCubed']),F(case['twiceArea'])/2)
                    self.assertTrue(budget.union_work['activeFacetRegion']['geometryEncoded'])
                    records.append(dict(fixture=case['id'],kind='local_recovery_source_counterexample',oldExpectedRefusalSeconds=old_seconds,currentRecoveryAndQualificationSeconds=time.monotonic()-start,currentOperations=budget.operations,originalVertexCount=len(points),triangles=case['triangles'],localFlips=region.work['localDiagonalFlips'],backtracks=region.work['recoveryBacktracks'],oldFailureNotPerformanceBaseline=True))

        def test_production_refusal_retains_bounded_active_state_and_storage(self):
            case=frozen['cases'][0]
            points,shell,budget,arr,region,mesh,a,b=fixture(case,subject)
            region.replace_mesh(mesh)
            crossed=next(region.edge(c,d) for t in mesh for c,d in zip(t,t[1:]+t[:1]) if region.crosses(a,b,c,d))
            region.locked={crossed};snapshot=repr(region.triangles)
            with self.assertRaisesRegex(kernel.UnionFailure,'locked physical constraint') as caught:region.recover(a,b)
            note=json.loads(caught.exception.__notes__[0]);active=note['unionWork']['activeFacetRegion']
            self.assertEqual(active['source'],[0,1]);self.assertTrue(active['geometryEncoded'])
            encoded=[tuple(map(F,p)) for p in active['points']]
            self.assertEqual([encoded[i] for i in active['target']],[a,b])
            self.assertEqual({tuple(encoded[i] for i in t) for t in active['meshTriangles']},set(mesh))
            self.assertEqual(note['operations'],budget.operations);self.assertEqual(repr(region.triangles),snapshot)
            points,shell,budget,arr,region,mesh,a,b=fixture(case,subject)
            region.replace_mesh(mesh);budget.maximum_operations=budget.operations
            with self.assertRaisesRegex(kernel.UnionFailure,'operation budget') as caught:region.recover(a,b)
            note=json.loads(caught.exception.__notes__[0]);active=note['unionWork']['activeFacetRegion']
            self.assertTrue(active['geometryEncoded']);self.assertEqual(len(active['meshTriangles']),len(mesh))
            self.assertEqual(budget.operations,budget.maximum_operations+1)
            self.assertTrue(all(i is not None for i in active['target']))
            points,shell,budget,arr,region,mesh,a,b=fixture(case,subject)
            region.replace_mesh(mesh);budget.maximum_polygons=region.current_slots()-1
            with self.assertRaisesRegex(kernel.UnionFailure,'polygon budget') as caught:region.guard()
            note=json.loads(caught.exception.__notes__[0]);active=note['unionWork']['activeFacetRegion']
            self.assertGreater(active['requiredSlots'],budget.maximum_polygons);self.assertTrue(active['geometryEncoded'])
            # Actual complex memberships replace the old quadratic reservation under100k.
            points,shell,budget,arr,region,mesh,a,b=fixture(frozen['cases'][1],subject)
            region.complex();self.assertLessEqual(region.work['peakSlots'],100000)
            self.assertEqual(budget.maximum_polygons,100000)
            storage=json.loads((refdir/'SUPPLEMENTAL-STORAGE-CONTROL.json').read_text())
            storage_rows=[]
            for lane,module in [('old_quadratic',old),('actual_memberships',subject)]:
                budget=kernel.Budget();domain=helpers.points(storage['domain'])
                arr=module.PlanarArrangement(domain,budget,kernel.UnionFailure)
                barriers=[(tuple(map(F,p)),tuple(map(F,p))) for p in storage['contacts']]
                cells=arr.cells(barriers,lambda p:True)
                self.assertEqual(sum(oracle.twice_area(c) for c in cells),F(storage['twiceArea']))
                graph=module.FacetRegions(arr,cells)
                if lane=='old_quadratic':
                    with self.assertRaisesRegex(kernel.UnionFailure,'polygon budget'):graph.complex()
                    storage_rows.append(dict(lane=lane,refused=True,operations=budget.operations))
                else:
                    boundary,keep=graph.complex()
                    self.assertTrue(arr.region_protected<=keep)
                    self.assertLessEqual(graph.work['peakSlots'],100000)
                    storage_rows.append(dict(lane=lane,refused=False,operations=budget.operations,peakSlots=graph.work['peakSlots']))
                self.assertEqual(budget.maximum_polygons,100000)
            records.append(dict(fixture='bounded_actual_graph_memberships',kind='local_recovery_storage_control',scope=storage['scope'],lanes=storage_rows))
            original_recover=subject.FacetRegions.recover_indexed
            def plain_predicate(current,x,y):
                current.active.update(stage='independent_plain_predicate_fixture',target=[current.point_ids[x],current.point_ids[y]])
                raise kernel.UnionFailure('Independent plain recovery predicate fixture')
            subject.FacetRegions.recover_indexed=plain_predicate
            try:
                budget=kernel.Budget();shells=[helpers.box(kernel,(0,0,0,2,2,1),0),helpers.box(kernel,(1,1,0,3,3,1),1)]
                with self.assertRaisesRegex(kernel.UnionFailure,'plain recovery predicate') as caught:subject.surface_union(shells,budget,kernel=kernel)
                note=json.loads(caught.exception.__notes__[0]);active=note['unionWork']['activeFacetRegion']
                self.assertTrue(active['geometryEncoded']);self.assertEqual(active['source'],note['unionWork']['currentSource'])
                self.assertTrue(active['meshTriangles']);self.assertEqual(note['operations'],budget.operations)
            finally:subject.FacetRegions.recover_indexed=original_recover
            composed=helpers.load('local_recovery_outer_api',Path(__file__).with_name('exact-surface-union-source-arrangement.py'))
            original_audit=composed.audit_source_fragments
            def plain_audit(*args):raise composed.UnionFailure('Independent plain full-source audit fixture')
            composed.audit_source_fragments=plain_audit
            try:
                budget=composed.Budget();shell=helpers.box(composed,(0,0,0,1,1,1),0)
                with self.assertRaisesRegex(composed.UnionFailure,'plain full-source audit') as caught:composed.surface_union([shell],budget)
                note=json.loads(caught.exception.__notes__[0]);work=note['unionWork']
                self.assertEqual(work['stage'],'source_fragment_audit')
                self.assertEqual(len(work['facetRegionWitness']),12)
                self.assertEqual(len(work['sourceArrangementPerFacetWork']),12)
                self.assertFalse(work['facetRegionWitnessComplete'])
                self.assertEqual(note['operations'],budget.operations)
            finally:composed.audit_source_fragments=original_audit
            budget=composed.Budget()
            shells=[helpers.box(composed,(0,0,0,1,1,1),0),helpers.box(composed,(1,1,1,2,2,2),1)]
            boundary,_=composed.surface_union(shells,budget)
            with self.assertRaisesRegex(composed.UnionFailure,'manifold|disconnected') as caught:composed.boundary_mesh(boundary,budget)
            note=json.loads(caught.exception.__notes__[0])
            self.assertEqual(note['operations'],budget.operations)
            self.assertTrue(note['unionWork']['facetRegionWitnessComplete'])
            self.assertEqual(len(note['unionWork']['facetRegionWitness']),24)
            self.assertTrue(note['conformanceWork'])
            # Exercise the ACTUAL target-already-present branch at both ID preparations.
            for remaining in (1,2):
                with self.subTest(targetPresentBudgetRemaining=remaining):
                    points,shell,budget,arr,region,mesh,_,_=fixture(frozen['cases'][0],subject)
                    a,b=points[0],points[1]
                    before_locked=set(region.locked);before_mesh=repr(region.triangles)
                    real_mesh_edges=region.mesh_edges
                    armed=[]
                    def arm_target_publication():
                        edges=real_mesh_edges()
                        self.assertIn(region.edge(a,b),edges)
                        budget.maximum_operations=budget.operations+remaining
                        armed.append(budget.maximum_operations)
                        return edges
                    region.mesh_edges=arm_target_publication
                    with self.assertRaisesRegex(kernel.UnionFailure,'operation budget') as caught:region.recover(a,b)
                    self.assertEqual(len(armed),1)
                    self.assertEqual(budget.operations,budget.maximum_operations+1)
                    note=json.loads(caught.exception.__notes__[0]);active=note['unionWork']['activeFacetRegion']
                    pool=[tuple(map(F,p)) for p in active['points']]
                    retained={tuple(sorted((pool[x],pool[y]))) for x,y in active['lockedConstraints']}
                    self.assertEqual(region.locked,retained)
                    self.assertEqual(region.locked,before_locked)
                    self.assertEqual(repr(region.triangles),before_mesh)
                    self.assertEqual(note['operations'],budget.operations)
            # All preparation succeeding publishes both representations together.
            points,shell,budget,arr,region,mesh,_,_=fixture(frozen['cases'][0],subject)
            region.recover(points[0],points[1])
            pool=[tuple(map(F,p)) for p in region.active['points']]
            retained={tuple(sorted((pool[x],pool[y]))) for x,y in region.active['lockedConstraints']}
            self.assertEqual(region.locked,retained)
            self.assertIn(region.edge(points[0],points[1]),retained)
    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
