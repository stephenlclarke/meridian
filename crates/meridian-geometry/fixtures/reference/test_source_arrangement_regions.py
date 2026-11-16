"""Frozen exact facet-region/constraint/star coverage controls."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
import unittest


def region_suite(kernel, subject, helpers, records):
    refdir=Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/facet-region-reference'
    baseline=helpers.load('region_unfiltered_4579',refdir/'unfiltered-4579.py')
    oracle=helpers.load('region_independent_clip',refdir.parent/'reference/independent_planar_reference.py')
    coverage=helpers.load('region_source_coverage',Path(__file__).with_name('test_source_arrangement_strips.py'))
    frozen=json.loads((refdir/'cases.json').read_text())

    def contains(polygon, point):
        # Independent exact even-odd classifier supports the nonconvex reference.
        x,y=point;result=False
        for a,b in zip(polygon,polygon[1:]+polygon[:1]):
            if oracle.orient(a,b,point)==0 and all(min(a[k],b[k])<=point[k]<=max(a[k],b[k]) for k in range(2)):
                return True
            if (a[1]>y)!=(b[1]>y):
                intercept=a[0]+(y-a[1])*(b[0]-a[0])/(b[1]-a[1])
                if x<intercept:result=not result
        return result

    def boundary_rays(polygons, point):
        # Independent local oriented-boundary star; reciprocal interior rays cancel.
        rays={}
        for polygon in polygons:
            for a,b in zip(polygon,polygon[1:]+polygon[:1]):
                if oracle.orient(a,b,point) or not all(min(a[k],b[k])<=point[k]<=max(a[k],b[k]) for k in range(2)):
                    continue
                for endpoint,sign in [(a,-1),(b,1)]:
                    delta=tuple(endpoint[k]-point[k] for k in range(2))
                    divisor=next((abs(v) for v in delta if v),None)
                    if divisor is not None:
                        key=tuple(v/divisor for v in delta)
                        rays[key]=rays.get(key,0)+sign
        return {ray:count for ray,count in rays.items() if count}

    def fixture(case,module,budget):
        regions=[helpers.points(p) for p in case['regions']]
        holes=[helpers.points(p) for p in case.get('holes',[])]
        barriers=[(a,b) for polygon in regions+holes for a,b in zip(polygon,polygon[1:]+polygon[:1])]
        barriers += [(tuple(map(F,a)),tuple(map(F,b))) for a,b in case.get('extraSegments',[])]
        arr=module.PlanarArrangement(helpers.points(frozen['domain']),budget,kernel.UnionFailure)
        cells=arr.cells(barriers,lambda p:any(contains(r,p) for r in regions) and not any(contains(h,p) for h in holes))
        return arr,cells

    class Controls(unittest.TestCase):
        def test_frozen_regions_mutual_coverage_contacts_and_full_work(self):
            pins=json.loads((refdir/'FROZEN.json').read_text())
            for name,pin in pins['files'].items():
                data=(refdir/name).read_bytes();self.assertEqual(len(data),pin['bytes']);self.assertEqual(hashlib.sha256(data).hexdigest(),pin['sha256'])
            for case in frozen['cases']:
                with self.subTest(case=case['id']):
                    before=kernel.Budget();start=time.monotonic();_,expected=fixture(case,baseline,before)
                    # Reference full planar qualification includes unchanged exact ear triangulation.
                    reference_triangles=0
                    for cell in expected:
                        face=kernel.polygon([kernel.Vertex((p[0],p[1],F(0)),(p[0]+2*p[1],-p[1])) for p in cell],(0,0),'{}')
                        reference_triangles += len(kernel.triangulate(face,before))
                    reference_seconds=time.monotonic()-start
                    budget=kernel.Budget();start=time.monotonic();arr,cells=fixture(case,subject,budget)
                    actual,work=subject.FacetRegions(arr,cells).reconstruct();candidate_seconds=time.monotonic()-start
                    self.assertEqual(sum(oracle.twice_area(p) for p in actual),F(case['twiceArea']))
                    self.assertEqual(sum(oracle.twice_area(p) for p in expected),F(case['twiceArea']))
                    common=sum(oracle.twice_area(oracle.intersection(list(a),b)) for a in expected for b in actual)
                    self.assertEqual(common,F(case['twiceArea']))
                    for i,a in enumerate(actual):
                        for b in actual[i+1:]:self.assertEqual(oracle.twice_area(oracle.intersection(list(a),b)),0)
                    self.assertLessEqual(len(actual),case['maximumTriangles'])
                    vertices=set(p for triangle in actual for p in triangle)
                    for point in arr.region_protected:
                        self.assertEqual(boundary_rays(expected,point),boundary_rays(actual,point))
                        if any(contains(cell,point) for cell in expected):self.assertIn(point,vertices)
                    if 'requiredPoint' in case:self.assertIn(tuple(map(F,case['requiredPoint'])),vertices)
                    if 'requiredSegment' in case:
                        a,b=map(lambda p:tuple(map(F,p)),case['requiredSegment'])
                        self.assertTrue(any((a,b) in list(zip(t,t[1:]+t[:1])) or (b,a) in list(zip(t,t[1:]+t[:1])) for t in actual))
                    self.assertLess(candidate_seconds,10*reference_seconds)
                    records.append(dict(fixture=case['id'],kind='exact_facet_regions',referenceSeconds=reference_seconds,candidateSeconds=candidate_seconds,referenceOperations=before.operations,candidateOperations=budget.operations,referenceTriangles=reference_triangles,candidateTriangles=len(actual),work=work))

        def test_complete_original_solids_source_fields_contacts_and_witness(self):
            cases=json.loads((refdir.parent/'neutral-constraint-reference/cases.json').read_text())['cases']
            for case in cases:
                with self.subTest(case=case['id']):
                    owner=1 if case.get('box') else 0
                    shells=[helpers.box(kernel,(-2,-2,-2,2,2,0),owner)]
                    if case.get('box'):
                        shells.append(helpers.box(kernel,(-1,-1,-1,1,1,0),0))
                    elif case.get('aboveBox'):
                        shells.append(helpers.box(kernel,(-1,-1,0,0,0,1),1))
                    else:
                        z=F(case['baseZ']);vertices=[(F(-1,2),F(0),z),(F(1,2),F(0),z),(F(0),F(1),z),tuple(map(F,case['apex']))];faces=[]
                        for omitted in range(4):
                            face=[i for i in range(4) if i!=omitted];a,b,c=[vertices[i] for i in face]
                            if kernel.dot(kernel.cross(kernel.subtract(b,a),kernel.subtract(c,a)),kernel.subtract(vertices[omitted],a))>0:face.reverse()
                            faces.append(face)
                        textures=[dict(index=0,uv=[[vertices[i][0]+2*vertices[i][1],vertices[i][2]-vertices[i][1]] for i in face]) for face in faces]
                        shells.append(kernel.input_shell(vertices,faces,[0]*4,textures,1))
                    if 'matrix' in case:
                        matrix=case['matrix'];shells=[[kernel.polygon([kernel.Vertex(tuple(sum(row[k]*v.point[k] for k in range(3)) for row in matrix),v.uv) for v in face.vertices],face.source,face.style) for face in shell] for shell in shells]
                    outputs=[];runs=[]
                    for label,module in [('reference',baseline),('candidate',subject)]:
                        budget=kernel.Budget();start=time.monotonic();boundary,work=module.surface_union(shells,budget,kernel=kernel)
                        volume=sum(kernel.dot(face.vertices[0].point,kernel.cross(face.vertices[i].point,face.vertices[i+1].point)) for face in boundary for i in range(1,len(face.vertices)-1))/6
                        self.assertEqual(volume,F(case['volume']));triangles=None
                        if case['singular']:
                            with self.assertRaisesRegex(kernel.UnionFailure,'manifold|disconnected'):kernel.boundary_mesh(boundary,budget)
                        else:triangles=len(kernel.boundary_mesh(boundary,budget)['faces'])
                        if label=='candidate':
                            witness=budget.union_work['facetRegionWitness'];self.assertTrue(witness)
                            self.assertEqual({tuple(w['source']) for w in witness},{f.source for f in boundary})
                            for w in witness:
                                self.assertTrue(w['sameOriginalSourceAndCompleteAffineStyle']);self.assertTrue(w['allPhysicalConstraintsRetained'])
                                if w['construction']=='unchanged_original_triangle':
                                    original=next(f for shell in shells for f in shell if f.source==tuple(w['source']))
                                    current=[f for f in boundary if f.source==original.source]
                                    self.assertEqual(current,[original])
                                    self.assertTrue(w['resolveGeometryAndFieldsFromPinnedOriginalSource'])
                                    continue
                                points=[tuple(map(F,p)) for p in w['points']]
                                old=[[points[i] for i in c] for c in w['classifiedCells']]
                                new=[[points[i] for i in c] for c in w['outputPieces']]
                                area=sum(oracle.twice_area(c) for c in old)
                                self.assertEqual(area,F(w['twiceArea']))
                                self.assertEqual(sum(oracle.twice_area(c) for c in new),area)
                                common=sum(oracle.twice_area(oracle.intersection(list(a),b)) for a in old for b in new)
                                self.assertEqual(common,area)
                                for ai,bi,left,right in w['cancelledAuxiliaryEdges']:
                                    a,b=points[ai],points[bi]
                                    for c,x,y in [(old[left],a,b),(old[right],b,a)]:
                                        self.assertTrue(any(oracle.orient(u,v,x)==oracle.orient(u,v,y)==0 and all(min(u[k],v[k])<=min(x[k],y[k])<=max(x[k],y[k])<=max(u[k],v[k]) for k in range(2)) for u,v in zip(c,c[1:]+c[:1])))
                                    self.assertFalse(any(oracle.orient(points[x],points[y],a)==oracle.orient(points[x],points[y],b)==0 and all(min(points[x][k],points[y][k])<=min(a[k],b[k])<=max(a[k],b[k])<=max(points[x][k],points[y][k]) for k in range(2)) for x,y in w['physicalSegments'] if x!=y))
                                protected=set(w['protectedPoints'])
                                for index in protected:
                                    point=points[index]
                                    self.assertEqual(boundary_rays(old,point),boundary_rays(new,point))
                                    if any(contains(cell,point) for cell in old):self.assertTrue(any(point in cell for cell in new))
                                for star in w['removedVertexStars']:
                                    self.assertNotIn(star['point'],protected)
                                    graph={}
                                    for a,b in star['sectors']:graph.setdefault(a,[]).append(b);graph.setdefault(b,[]).append(a)
                                    self.assertTrue(all(len(n) in (1,2) for n in graph.values()))
                                    ends=[i for i,n in graph.items() if len(n)==1]
                                    if star['kind']=='cycle':self.assertEqual(ends,[])
                                    else:
                                        self.assertEqual(len(ends),2)
                                        self.assertEqual(oracle.orient(points[ends[0]],points[ends[1]],points[star['point']]),0)
                        runs.append(dict(lane=label,seconds=time.monotonic()-start,operations=budget.operations,triangles=triangles));outputs.append(boundary)
                    coverage.assert_source_equivalent(self,kernel,*outputs,shells,helpers)
                    self.assertLess(runs[1]['seconds'],10*runs[0]['seconds'])
                    records.append(dict(fixture=case['id'],kind='facet_region_full_solid',runs=runs))

            # Exercise the actual composed recipe adapter with small authenticated geometry.
            composed=helpers.load('region_composed_adapter',Path(__file__).with_name('exact-surface-union-source-arrangement.py'))
            budget=composed.Budget();shells=[helpers.box(composed,(0,0,0,1,1,1),0)]
            boundary,_=composed.surface_union(shells,budget)
            self.assertTrue(budget.union_work['facetRegionWitnessComplete'])
            self.assertEqual(len(budget.union_work['sourceArrangementPerFacetWork']),12)
            self.assertEqual(len(budget.union_work['facetRegionWitness']),12)
            self.assertEqual(len(composed.boundary_mesh(boundary,budget)['faces']),12)

        def test_refuse_work_storage_and_unproved_overlapping_incidence(self):
            case=frozen['cases'][0]
            budget=kernel.Budget();arr,cells=fixture(case,subject,budget)
            budget.maximum_operations=budget.operations
            with self.assertRaisesRegex(kernel.UnionFailure,'operation budget'):subject.FacetRegions(arr,cells).reconstruct()
            budget=kernel.Budget();arr,cells=fixture(case,subject,budget);budget.maximum_polygons=1
            with self.assertRaisesRegex(kernel.UnionFailure,'polygon budget'):subject.FacetRegions(arr,cells)
            budget=kernel.Budget();arr,cells=fixture(case,subject,budget)
            with self.assertRaisesRegex(kernel.UnionFailure,'reciprocal'):subject.FacetRegions(arr,cells+cells).reconstruct()
            correction=json.loads((refdir/'RATIONAL-GUARD-CORRECTION.json').read_text())
            self.assertIn('before',correction['scope'])
            budget=kernel.Budget();arr,cells=fixture(case,subject,budget)
            region=subject.FacetRegions(arr,cells)
            with self.assertRaisesRegex(kernel.UnionFailure,'4096'):
                region.sample([(F(1,(1<<2050)-1),F(0)),(F(1,(1<<2050)-3),F(0)),(F(0),F(0))])
            # A conflicting constraint must never erase any already protected edge.
            budget=kernel.Budget();arr,cells=fixture(case,subject,budget)
            region=subject.FacetRegions(arr,cells);region.triangles=[tuple(arr.domain)]
            region.insert((F(2),F(2)));region.insert((F(2),F(6)))
            points=set(p for t in region.triangles for p in t)
            edges={tuple(sorted((a,b))) for t in region.triangles for a,b in zip(t,t[1:]+t[:1])}
            pair=next((a,b) for a in sorted(points) for b in sorted(points) if a<b and (a,b) not in edges and not any(p not in (a,b) and region.on(a,b,p) for p in points))
            region.locked=set(edges)
            with self.assertRaisesRegex(kernel.UnionFailure,'locked physical constraint'):region.recover(*pair)
            # Witness sorting itself must charge its rational comparisons, not just encoding.
            from functools import cmp_to_key
            original_ordered=subject.PlanarArrangement.ordered
            calls=[]
            exhaust=False
            def tracked_ordered(arrangement, values):
                kind=('physical' if values is getattr(arrangement,'region_constraints',None)
                      else 'protected' if values is getattr(arrangement,'region_protected',None) else None)
                comparisons=0
                def independent_compare(a,b):
                    nonlocal comparisons
                    comparisons+=1
                    return (a>b)-(a<b)
                if kind is not None:
                    sorted(values,key=cmp_to_key(independent_compare))
                    calls.append(kind)
                before=arrangement.budget.operations
                if exhaust and kind=='physical':
                    self.assertGreater(comparisons,0)
                    arrangement.budget.maximum_operations=before+comparisons-1
                ordered=original_ordered(arrangement,values)
                if kind is not None:self.assertEqual(arrangement.budget.operations-before,comparisons)
                return ordered
            shells=[helpers.box(kernel,(0,0,0,2,2,1),0),helpers.box(kernel,(1,1,0,3,3,1),1)]
            subject.PlanarArrangement.ordered=tracked_ordered
            try:
                subject.surface_union(shells,kernel.Budget(),kernel=kernel)
                self.assertEqual(set(calls),{'physical','protected'})
                calls.clear();exhaust=True;limited=kernel.Budget()
                with self.assertRaisesRegex(kernel.UnionFailure,'operation budget'):
                    subject.surface_union(shells,limited,kernel=kernel)
                self.assertIn('physical',calls)
                self.assertEqual(limited.operations,limited.maximum_operations+1)
            finally:
                subject.PlanarArrangement.ordered=original_ordered
    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
