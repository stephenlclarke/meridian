"""Independent full construction parity for indexed facet recovery."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
import unittest


def check_live_slots(test, kernel, subject, helpers, oracle, case):
    import sys
    import weakref

    class Corridor(list):
        pass

    points=helpers.points(case['points']);domain=[points[i] for i in case['domain']]
    a,b=(points[i] for i in case['target'])
    for slack in (-1,0):
        with test.subTest(rollbackWorkspaceSlack=slack):
            budget=kernel.Budget();arr=subject.PlanarArrangement(domain,budget,kernel.UnionFailure)
            cells=arr.cells([(p,p) for p in points]+[(a,b)],lambda p:True)
            region=subject.FacetRegions(arr,cells)
            region.replace_mesh([tuple(points[i] for i in tri) for tri in case['mesh']])
            entry_mesh=list(region.triangles)
            corridor=region.local_corridor;flip=region.flip_candidate
            commit=region.commit_indexed_flip;fallback=region.recover
            attempts=[];thresholds=[];forward_mesh=[]
            def observe_corridor(*args):
                found=corridor(*args)
                test.assertTrue(found)
                observed=Corridor(found)
                attempts.append(weakref.ref(observed))
                return observed
            def deny_second_flip(*args):
                return flip(*args) if len(attempts)==1 else None
            def bound_rollback(*args):
                if len(attempts)==1:
                    commit(*args);forward_mesh.extend(region.triangles)
                    return
                test.assertEqual(len(attempts),2)
                test.assertEqual(region.work['indexedMonotoneFlips'],1)
                test.assertIsNone(attempts[-1](), 'rollback retained an unaccounted nonempty corridor')
                maximum=budget.maximum_polygons
                threshold=region.current_slots()+80
                thresholds.append(threshold)
                budget.maximum_polygons=threshold+slack
                try:return commit(*args)
                finally:budget.maximum_polygons=maximum
            def verify_fallback(*args):
                test.assertEqual(region.triangles,entry_mesh)
                test.assertFalse(region.locked)
                region.flip_candidate=flip
                return fallback(*args)
            region.local_corridor=observe_corridor
            region.flip_candidate=deny_second_flip
            region.commit_indexed_flip=bound_rollback
            region.recover=verify_fallback
            if slack<0:
                with test.assertRaisesRegex(kernel.UnionFailure,'polygon budget'):
                    region.recover_indexed(a,b)
                test.assertEqual(region.triangles,forward_mesh)
                test.assertEqual(region.work['indexedRollbackFlips'],0)
                test.assertFalse(region.locked)
            else:
                region.recover_indexed(a,b)
                test.assertEqual(region.work['indexedRollbackFlips'],1)
                test.assertEqual(region.work['indexedFallbacks'],1)
                test.assertIn(region.edge(a,b),region.locked)
                test.assertEqual(set(p for t in region.triangles for p in t),set(points))
                test.assertEqual(sum(oracle.twice_area(t) for t in region.triangles),F(case['twiceArea']))
            test.assertEqual(len(thresholds),1)

    for slack in (-1,0):
        with test.subTest(nodingExtensionSlack=slack):
            budget=kernel.Budget()
            arr=subject.PlanarArrangement(helpers.points([[0,0],[8,0],[0,8]]),budget,kernel.UnionFailure)
            arr.cells([],lambda p:True)
            # Three reciprocal T-junctions share the large cell's y=2 edge.
            cells=[helpers.points(cell) for cell in [
                [[0,2],[6,2],[0,8]],[[0,0],[2,2],[0,2]],
                [[0,0],[4,2],[2,2]],[[0,0],[6,2],[4,2]]]]
            region=subject.FacetRegions(arr,cells)
            guard=region.guard
            thresholds=[];after_cells=[]
            def bound_extension(extra=0):
                caller=sys._getframe(1)
                if caller.f_code.co_name!='graph_guard':
                    return guard(extra)
                scope=caller.f_back.f_locals
                live_cut=scope.get('cuts')
                if 'sectors' in scope:
                    test.assertNotIn('noded',scope)
                    test.assertNotIn('cuts',scope)
                    after_cells.append(True)
                elif live_cut is not None and not thresholds and len(live_cut)>2:
                    count=len(live_cut)
                    live_growth=len(scope['noded'])+count+2*(count-1)
                    requested_growth=caller.f_locals['extra']
                    test.assertEqual(requested_growth,live_growth,'noding extension omits live destination growth')
                    threshold=region.current_slots()+extra
                    maximum=budget.maximum_polygons
                    thresholds.append(threshold)
                    budget.maximum_polygons=threshold+slack
                    try:return guard(extra)
                    finally:budget.maximum_polygons=maximum
                return guard(extra)
            region.guard=bound_extension
            if slack<0:
                with test.assertRaisesRegex(kernel.UnionFailure,'polygon budget'):
                    region.complex()
            else:
                _,keep=region.complex()
                test.assertTrue(region.protected<=keep)
                test.assertTrue(after_cells)
            test.assertEqual(len(thresholds),1)


def incidence_suite(kernel, subject, helpers, records):
    refdir=Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/local-incidence-reference'
    old=helpers.load('frozen_4ab6_incidence',refdir/'before-4ab6.py')
    coverage=helpers.load('local_incidence_coverage',Path(__file__).with_name('test_source_arrangement_strips.py'))
    oracle=helpers.load('local_incidence_oracle',refdir.parent/'reference/independent_planar_reference.py')

    class Controls(unittest.TestCase):
        def test_indexed_complete_source_cost_contact_and_fallback_parity(self):
            manifest=json.loads((refdir/'FROZEN.json').read_text())
            for name,pin in manifest['sha256'].items():
                self.assertEqual(hashlib.sha256((refdir/name).read_bytes()).hexdigest(),pin)
            rows=[]
            for case in json.loads((refdir/'CASES.json').read_text())['cases']:
                with self.subTest(case=case['id']):
                    if case.get('reuse'):
                        shells=[helpers.box(kernel,(-2,-2,-2,2,2,0),0)]
                        vertices=[(F(-1,2),F(0),F(1)),(F(1,2),F(0),F(1)),(F(0),F(1),F(1)),(F(0),F(1,2),F(0))]
                        faces=[]
                        for omitted in range(4):
                            face=[i for i in range(4) if i!=omitted];a,b,c=[vertices[i] for i in face]
                            if kernel.dot(kernel.cross(kernel.subtract(b,a),kernel.subtract(c,a)),kernel.subtract(vertices[omitted],a))>0:face.reverse()
                            faces.append(face)
                        textures=[dict(index=0,uv=[[vertices[i][0]+2*vertices[i][1],vertices[i][2]-vertices[i][1]] for i in face]) for face in faces]
                        shells.append(kernel.input_shell(vertices,faces,[0]*4,textures,1))
                    else:
                        shells=[helpers.box(kernel,bounds,i) for i,bounds in enumerate(case['boxes'])]
                    if case.get('shear'):
                        shells=[[kernel.polygon([kernel.Vertex((v.point[0]+v.point[2],v.point[1],v.point[0]+2*v.point[2]),v.uv)
                                                for v in face.vertices],face.source,face.style) for face in shell] for shell in shells]
                    outputs=[];runs=[]
                    for module in (old,subject):
                        budget=kernel.Budget();start=time.monotonic()
                        # Identical charged-operation probes in both lanes; no
                        # work is excluded from the complete budget comparison.
                        measured={}
                        original=[]
                        def instrument(owner,name,key):
                            method=getattr(owner,name)
                            def counted(obj,*args,**kwargs):
                                before=obj.budget.operations
                                try:return method(obj,*args,**kwargs)
                                finally:measured[key]=measured.get(key,0)+obj.budget.operations-before
                            original.append((owner,name,method));setattr(owner,name,counted)
                        instrument(module.OriginalFaceIndex,'near','originalSectionLookupAndConstruction')
                        instrument(module.OriginalFaceIndex,'section_for','originalSectionLookupAndConstruction')
                        instrument(module.OriginalFaceIndex,'section_plane','originalSectionLookupAndConstruction')
                        instrument(module.FacetRegions,'complex','classifiedComplex')
                        instrument(module.FacetRegions,'recover' if module is old else 'recover_indexed','allConstraintRecovery')
                        try:boundary,work=module.surface_union(shells,budget,kernel=kernel)
                        finally:
                            for owner,name,method in reversed(original):setattr(owner,name,method)
                        source_ops=budget.operations
                        if case.get('singular'):
                            with self.assertRaisesRegex(kernel.UnionFailure,'manifold|disconnected'):
                                kernel.boundary_mesh(boundary,budget)
                            triangles=None
                        else:
                            mesh=kernel.boundary_mesh(boundary,budget)
                            self.assertEqual(F(mesh['exactUnionVolumeSourceUnitsCubed']),F(case['volume']))
                            triangles=len(mesh['faces'])
                        stages=[facet.get('facetRegionWork',{}) for facet in work['perSource']]
                        runs.append(dict(seconds=time.monotonic()-start,sourceOperations=source_ops,completeOperations=budget.operations,
                                         triangles=triangles,matchedChargedStages=measured,regionCounters={k:sum(f.get(k,0) for f in stages)
                                           for k in ['indexedComplexWork','indexedPointAndConstraintBuildWork','indexedRecoveryWork','indexedDirectLocks','indexedMonotoneFlips','indexedFallbacks','localDiagonalFlips']}))
                        outputs.append(boundary)
                    coverage.assert_source_equivalent(self,kernel,*outputs,shells,helpers)
                    self.assertLess(runs[1]['seconds'],10*runs[0]['seconds'])
                    row=dict(fixture=case['id'],kind='complete_incidence_parity',runs=runs)
                    rows.append(row);records.append(row)
            for row in rows:
                before,after=row['runs']
                self.assertEqual(before['triangles'],after['triangles'])
                self.assertEqual(before['matchedChargedStages']['originalSectionLookupAndConstruction'],
                                 after['matchedChargedStages']['originalSectionLookupAndConstruction'])
                self.assertLess(after['matchedChargedStages']['allConstraintRecovery'],
                                before['matchedChargedStages']['allConstraintRecovery'])
                self.assertLess(after['completeOperations'],before['completeOperations'])
            self.assertGreater(sum(row['runs'][1]['regionCounters']['indexedDirectLocks'] for row in rows),0)
            self.assertGreater(sum(row['runs'][1]['regionCounters']['indexedMonotoneFlips'] for row in rows),0)

            # Force corridor uncertainty on an independent valid fixture:
            # fallback must still insert the exact target and keep all vertices.
            case=json.loads((refdir.parent/'local-recovery-reference/cases.json').read_text())['cases'][0]
            points=helpers.points(case['points']);domain=[points[i] for i in case['domain']]
            a,b=(points[i] for i in case['target'])
            budget=kernel.Budget();arr=subject.PlanarArrangement(domain,budget,kernel.UnionFailure)
            cells=arr.cells([(p,p) for p in points]+[(a,b)],lambda p:True)
            region=subject.FacetRegions(arr,cells)
            region.replace_mesh([tuple(points[i] for i in tri) for tri in case['mesh']])
            original_corridor=region.local_corridor
            original_recover=region.recover
            original_mesh=list(region.triangles)
            attempts=[]
            def corridor_once(*args):
                attempts.append(1)
                return original_corridor(*args) if len(attempts)==1 else None
            def checked_fallback(*args):
                self.assertEqual(region.triangles,original_mesh)
                self.assertFalse(region.locked)
                return original_recover(*args)
            region.local_corridor=corridor_once
            region.recover=checked_fallback
            region.recover_indexed(a,b)
            self.assertEqual(region.work['indexedMonotoneFlips'],1)
            self.assertEqual(region.work['indexedRollbackFlips'],1)
            self.assertEqual(region.work['indexedFallbacks'],1)
            self.assertIn(region.edge(a,b),region.locked)
            self.assertEqual(set(p for t in region.triangles for p in t),set(points))
            self.assertEqual(sum(oracle.twice_area(t) for t in region.triangles),F(case['twiceArea']))
            check_live_slots(self,kernel,subject,helpers,oracle,case)

    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
