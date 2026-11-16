"""Independent coverage, contact and work checks for canonical finite strips."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
import unittest


def assert_source_equivalent(test, kernel, baseline, candidate, shells, helpers):
    root=Path(__file__).resolve().parents[2]
    reference=helpers.load('strip_independent_coverage',root/'docs/fleet-physical/source-arrangement/reference/independent_planar_reference.py')
    original={face.source:face for shell in shells for face in shell}
    grouped=[]
    for boundary in (baseline,candidate):
        groups={}
        for face in boundary:
            test.assertIn(face.source,original)
            source=original[face.source]
            test.assertEqual(face.style,source.style)
            test.assertGreater(kernel.dot(face.normal,source.normal),0)
            axis=max(range(3),key=lambda i:abs(source.normal[i]))
            axes=[i for i in range(3) if i!=axis]
            project=lambda p:tuple(p[i] for i in axes)
            source_points=[project(v.point) for v in source.vertices]
            a,b,c=source_points
            denominator=reference.orient(a,b,c)
            points=[]
            for vertex in face.vertices:
                test.assertEqual(kernel.dot(source.normal,vertex.point),source.offset)
                p=project(vertex.point);points.append(p)
                weights=(reference.orient(b,c,p)/denominator,
                         reference.orient(c,a,p)/denominator,
                         reference.orient(a,b,p)/denominator)
                test.assertTrue(all(w>=0 for w in weights))
                expected=tuple(sum(w*v.uv[k] for w,v in zip(weights,source.vertices)) for k in range(2))
                test.assertEqual(vertex.uv,expected)
            if reference.twice_area(points)<0:points.reverse()
            test.assertGreater(reference.twice_area(points),0)
            for x,y in zip(points,points[1:]+points[:1]):
                test.assertTrue(all(reference.orient(x,y,p)>=0 for p in points))
            groups.setdefault(face.source,[]).append(points)
        grouped.append(groups)
    test.assertEqual(set(grouped[0]),set(grouped[1]))
    for source,old in grouped[0].items():
        new=grouped[1][source]
        old_area=sum(reference.twice_area(p) for p in old)
        new_area=sum(reference.twice_area(p) for p in new)
        common=sum(reference.twice_area(reference.intersection(list(a),b)) for a in old for b in new)
        test.assertEqual(old_area,new_area)
        test.assertEqual(common,old_area)
        for i,a in enumerate(new):
            for b in new[i+1:]:
                test.assertEqual(reference.twice_area(reference.intersection(list(a),b)),0)


def strip_suite(kernel,subject,reference,helpers,records):
    refdir=Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/canonical-strips-reference'
    frozen=json.loads((refdir/'FROZEN.json').read_text())
    cases=json.loads((refdir/'cases.json').read_text())
    endpoint_frozen=json.loads((refdir/'FROZEN-ENDPOINT-CORRECTION.json').read_text())
    cases['cases'] += json.loads((refdir/'endpoint-correction-case.json').read_text())['cases']
    unmerged=helpers.load('frozen_8f78_unmerged_arrangement',refdir/'unmerged-source-arrangement-8f78.py')

    def barriers_for(case):
        barriers=[(tuple(map(F,a)),tuple(map(F,b))) for a,b in case['barriers']]
        regions=[helpers.points(p) for p in case['regions']]
        for region in regions:barriers.extend(zip(region,region[1:]+region[:1]))
        return barriers,regions

    def assert_segments(test,cells,barriers):
        edges=[(a,b) for cell in cells for a,b in zip(cell,cell[1:]+cell[:1])]
        for a,b in barriers:
            if a==b:
                test.assertTrue(any(a in cell for cell in cells));continue
            delta=tuple(b[k]-a[k] for k in range(2));axis=0 if delta[0] else 1
            intervals=[]
            for c,d in edges:
                if reference.orient(a,b,c)==reference.orient(a,b,d)==0:
                    low,high=sorted(((c[axis]-a[axis])/delta[axis],(d[axis]-a[axis])/delta[axis]))
                    if high>=0 and low<=1:intervals.append((max(F(0),low),min(F(1),high)))
            end=F(0)
            for low,high in sorted(intervals):
                test.assertLessEqual(low,end);end=max(end,high)
            test.assertEqual(end,1)

    class Controls(unittest.TestCase):
        def test_frozen_reference_is_unchanged(self):
            for name,pin in {**frozen['files'],**endpoint_frozen['files']}.items():
                data=(refdir/name).read_bytes()
                self.assertEqual(len(data),pin['bytes'])
                self.assertEqual(hashlib.sha256(data).hexdigest(),pin['sha256'])

        def _check_physical_memberships_admitted_before_copy_and_contacts_preserved(self):
            domain=helpers.points([[0,0],[5,1],[1,5]])
            visited=[]
            constrained=subject.PlanarArrangement(domain,kernel.Budget(maximum_polygons=11),kernel.UnionFailure)
            with self.assertRaisesRegex(kernel.UnionFailure,'polygon budget'):
                constrained.cells([],lambda p:visited.append(p) or True)
            self.assertEqual(visited,[])
            self.assertEqual(constrained.work['physicalSegmentMemberships'],0)
            self.assertEqual(constrained.work['lastRequiredGeometrySlots'],12)
            self.assertLessEqual(constrained.work['peakAdmittedMembershipSlots'],11)
            # A genuine contact fixture still completes under a finite admitted
            # membership allowance; neither the point nor its interval is lost.
            barriers=[((F(2),F(2)),(F(2),F(2))),((F(1),F(1)),(F(3),F(2)))]
            budget=kernel.Budget(maximum_polygons=128)
            admitted=subject.PlanarArrangement(domain,budget,kernel.UnionFailure)
            actual=admitted.cells(barriers,lambda p:True)
            assert_segments(self,actual,barriers)
            self.assertTrue(any((F(2),F(2)) in cell for cell in actual))
            self.assertGreater(admitted.work['physicalSegmentMemberships'],3)
            self.assertLessEqual(admitted.work['peakAdmittedMembershipSlots'],128)
            self.assertEqual(sum(reference.twice_area(list(p))/F(2) for p in actual),12)

        def test_pre_cell_refusal_cannot_report_previous_facet_work(self):
            shell=helpers.box(kernel,(0,0,0,1,1,1),0)
            budget=kernel.Budget();index=subject.OriginalFaceIndex(shell,budget,kernel)
            budget.union_work['currentArrangementWork']={'previousFacetSentinel':True}
            budget.maximum_operations=budget.operations+30
            with self.assertRaisesRegex(kernel.UnionFailure,'operation budget') as caught:
                subject.reconstruct_face(shell[0],index,budget,kernel)
            current=budget.union_work['currentArrangementWork']
            self.assertNotIn('previousFacetSentinel',current)
            self.assertEqual(current['sourceFacet'],list(shell[0].source))
            self.assertEqual(current['phase'],'awaiting_constraints')
            self.assertEqual(current['classifiedCells'],0)
            notes=[json.loads(n) for n in caught.exception.__notes__ if n.startswith('{')]
            note=next(n for n in notes if 'operations' in n)
            self.assertEqual(note['unionWork']['currentArrangementWork'],current)

        def test_duplicate_raw_constraints_save_charged_clip_work(self):
            domain=helpers.points([[0,0],[5,1],[1,5]])
            barriers=[((F(1),F(1)),(F(3),F(2)))]*40
            lanes=[]
            for implementation in (unmerged,subject):
                budget=kernel.Budget();arr=implementation.PlanarArrangement(domain,budget,kernel.UnionFailure)
                output=arr.cells(barriers,lambda p:True);lanes.append((budget.operations,arr.work,output))
                assert_segments(self,output,barriers)
            self.assertLess(lanes[1][0],lanes[0][0])
            self.assertGreaterEqual(lanes[1][1]['rawConstraintCacheHits'],39)

        def test_sections_reuse_geometry_not_uv_or_domain(self):
            source=helpers.box(kernel,(0,0,0,1,1,1),0)
            budget=kernel.Budget();index=subject.OriginalFaceIndex(source,budget,kernel)
            first=source[0]
            second=kernel.Polygon(first.vertices,(7,9),'different-style',kernel.scale(first.normal,F(7,3)),first.offset*F(7,3))
            a=index.section_plane(first);b=index.section_plane(second)
            self.assertEqual(a,b)
            expected=unmerged._section(first,source[4],kernel.Budget(),kernel)
            before=budget.operations;value=index.section_for(first,4,a)
            self.assertEqual(value,(tuple(expected[0]),expected[1]))
            self.assertGreater(budget.operations,before)
            before=budget.operations;again=index.section_for(second,4,b)
            self.assertEqual(again,value)
            self.assertEqual(budget.operations-before,1)
            self.assertEqual(index.section_work['hits'],1)
            flipped=first.flipped();other=index.section_plane(flipped)
            self.assertNotEqual(other,a)
            index.section_for(flipped,4,other)
            self.assertEqual(index.section_work['misses'],2)

        def test_section_cache_saturation_computes_without_extra_budget(self):
            self._check_physical_memberships_admitted_before_copy_and_contacts_preserved()
            source=helpers.box(kernel,(0,0,0,1,1,1),0)
            budget=kernel.Budget();index=subject.OriginalFaceIndex(source,budget,kernel)
            budget.maximum_polygons=index.nodes
            plane=index.section_plane(source[0])
            expected=unmerged._section(source[0],source[4],kernel.Budget(),kernel)
            before=budget.operations
            for _ in range(2):
                actual=index.section_for(source[0],4,plane)
                self.assertEqual(actual,(tuple(expected[0]),expected[1]))
            self.assertGreater(budget.operations-before,2)
            self.assertEqual(index.section_cache,{})
            self.assertEqual(index.section_work['uncachedAtStorageBound'],2)

        def test_contact_and_convexity_guards_are_exercised(self):
            domain=helpers.points([[0,0],[6,0],[6,6],[0,6]])
            arrangement=subject.PlanarArrangement(domain,kernel.Budget(),kernel.UnionFailure)
            barrier=[((F(3),F(1)),(F(3),F(5)))]
            output=arrangement.cells(barrier,lambda p:True)
            assert_segments(self,output,barrier)
            self.assertGreater(arrangement.work['stripsContactBlocked'],0)
            # A top boundary with a downward notch cannot become one convex strip.
            other=subject.PlanarArrangement(domain,kernel.Budget(),kernel.UnionFailure)
            constraints=[((F(0),F(4)),(F(3),F(2))),((F(3),F(2)),(F(6),F(4)))]
            pieces=other.cells(constraints,lambda p:p[1]<F(4)-F(2,3)*p[0] if p[0]<3 else p[1]<F(2,3)*p[0])
            self.assertGreater(other.work['stripsConvexityRejected'],0)
            self.assertGreater(len(pieces),1)

        def test_sheared_closed_source_retains_fields_and_reduces_tessellation(self):
            old=helpers.box(kernel,(0,0,0,1,1,1),0)
            matrix=cases['solidReference']['matrix']
            vertices,lookup,faces,textures,materials=[],{},[],[],[]
            for face in old:
                indices=[]
                for vertex in face.vertices:
                    p=tuple(sum(F(row[k])*vertex.point[k] for k in range(3)) for row in matrix)
                    if p not in lookup:lookup[p]=len(vertices);vertices.append(p)
                    indices.append(lookup[p])
                style=json.loads(face.style);faces.append(indices);materials.append(style['material'])
                textures.append({**style['texture'],'uv':[list(v.uv) for v in face.vertices]})
            shell=kernel.input_shell(vertices,faces,materials,textures,0)
            lanes=[]
            for name,implementation in [('unmerged8f78',unmerged),('canonical_strips',subject)]:
                budget=kernel.Budget();start=time.monotonic()
                boundary,work=implementation.surface_union([shell],budget,kernel=kernel)
                mesh=kernel.boundary_mesh(boundary,budget)
                lanes.append(dict(lane=name,seconds=time.monotonic()-start,operations=budget.operations,
                                  boundary=boundary,triangles=len(mesh['faces']),volume=mesh['exactUnionVolumeSourceUnitsCubed'],work=work))
            baseline,candidate=lanes
            assert_source_equivalent(self,kernel,baseline['boundary'],candidate['boundary'],[shell],helpers)
            self.assertEqual(candidate['volume'],'1');self.assertEqual(baseline['volume'],'1')
            self.assertLess(candidate['triangles'],baseline['triangles'])
            self.assertLess(candidate['operations'],baseline['operations'])
            ratio=candidate['seconds']/baseline['seconds']
            records.append(dict(fixture='sheared_closed_source',kind='canonical_strip_full_pipeline_parity',
                                referenceSeconds=baseline['seconds'],candidateSeconds=candidate['seconds'],ratio=ratio,
                                referenceOperations=baseline['operations'],candidateOperations=candidate['operations'],
                                referenceTriangles=baseline['triangles'],candidateTriangles=candidate['triangles'],
                                exactPerSourceCoverageFieldsAndClosedVolume=True,work=candidate['work']))
            self.assertLess(ratio,10)

    for case in cases['cases']:
        def check(self,case=case):
            domain=helpers.points(case['domain']);barriers,regions=barriers_for(case)
            visible=lambda p:any(reference.inside(region,p) for region in regions)==(case['visible']=='inside')
            expected=reference.reference(domain,regions,case['visible'])
            lanes=[]
            for name,implementation in [('unmerged8f78',unmerged),('canonical_strips',subject)]:
                budget=kernel.Budget();arrangement=implementation.PlanarArrangement(domain,budget,kernel.UnionFailure)
                start=time.monotonic();cells=arrangement.cells(barriers,visible)
                lanes.append(dict(lane=name,seconds=time.monotonic()-start,operations=budget.operations,cells=cells,work=arrangement.work))
                self.assertEqual(sum(reference.twice_area(list(p))/F(2) for p in cells),F(case['area']))
                common=sum(reference.twice_area(reference.intersection(list(a),b))/F(2) for a in cells for b in expected)
                self.assertEqual(common,F(case['area']))
                for i,a in enumerate(cells):
                    for b in cells[i+1:]:self.assertEqual(reference.twice_area(reference.intersection(list(a),list(b))),0)
                    for x,y in zip(a,a[1:]+a[:1]):self.assertTrue(all(reference.orient(x,y,p)>=0 for p in a))
            baseline,candidate=lanes
            if not regions:assert_segments(self,candidate['cells'],barriers)
            for p in helpers.points(case.get('protectedPoints',[])):
                self.assertTrue(any(p in cell for cell in candidate['cells']))
            if 'expectedOutputPolygons' in case:self.assertEqual(len(candidate['cells']),case['expectedOutputPolygons'])
            if 'expectedOutputVertices' in case:self.assertEqual(set(candidate['cells'][0]),set(helpers.points(case['expectedOutputVertices'])))
            ratio=candidate['seconds']/baseline['seconds']
            records.append(dict(fixture=case['id'],kind='frozen_canonical_strip_planar_parity',
                                referenceSeconds=baseline['seconds'],candidateSeconds=candidate['seconds'],ratio=ratio,
                                referenceOperations=baseline['operations'],candidateOperations=candidate['operations'],
                                referenceCells=len(baseline['cells']),candidateCells=len(candidate['cells']),work=candidate['work']))
            self.assertLess(ratio,10)
        setattr(Controls,'test_frozen_strip_'+case['id'],check)
    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
