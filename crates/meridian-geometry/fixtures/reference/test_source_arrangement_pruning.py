"""Dynamic independent component-pruning parity/work/refusal controls."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
import unittest


def pruning_suite(kernel, subject, helpers, records):
    reference_dir=Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/component-pruning-reference'
    frozen=json.loads((reference_dir/'FROZEN.json').read_text())
    unpruned=helpers.load('frozen_f365_unpruned_arrangement',reference_dir/'unpruned-source-arrangement-f365.py')
    cases=json.loads((reference_dir/'cases.json').read_text())['cases']
    scale_frozen=json.loads((reference_dir/'FROZEN-SCALE-EXPANSION.json').read_text())
    cases += json.loads((reference_dir/'scale-case.json').read_text())['cases']

    def shells_for(case):
        owners=case.get('owners',list(range(len(case['boxes']))))
        grouped={}
        for bounds,owner in zip(case['boxes'],owners):
            grouped.setdefault(owner,[]).extend(helpers.box(kernel,[F(x) for x in bounds],owner))
        shells=[]
        for owner,polygons in grouped.items():
            # Authenticate the combined original owner as input_shell actually
            # does; do not assume separate connected boundaries per source owner.
            vertices,lookup,faces,materials,textures=[],{},[],[],[]
            for polygon in polygons:
                face=[]
                for vertex in polygon.vertices:
                    if vertex.point not in lookup:
                        lookup[vertex.point]=len(vertices);vertices.append(vertex.point)
                    face.append(lookup[vertex.point])
                style=json.loads(polygon.style)
                faces.append(face);materials.append(style['material'])
                textures.append({**style['texture'],'uv':[list(v.uv) for v in polygon.vertices]})
            shells.append(kernel.input_shell(vertices,faces,materials,textures,owner))
        return shells

    class Controls(unittest.TestCase):
        def test_frozen_reference_and_cases_are_unchanged(self):
            for name,pin in {**frozen['files'],**scale_frozen['files']}.items():
                data=(reference_dir/name).read_bytes()
                self.assertEqual(len(data),pin['bytes'])
                self.assertEqual(hashlib.sha256(data).hexdigest(),pin['sha256'])

        def test_inclusive_component_bounds_keep_face_edge_vertex(self):
            shells=[helpers.box(kernel,(0,0,0,1,1,1),0),helpers.box(kernel,(1,0,0,2,1,1),1)]
            budget=kernel.Budget();index=subject.OriginalFaceIndex([f for s in shells for f in s],budget,kernel)
            for point in [(F(1),F(1,2),F(1,2)),(F(1),F(1),F(1,2)),(F(1),F(1),F(1))]:
                self.assertEqual(set(index.containing_owners(point,[0,1])),{0,1})
            self.assertEqual(index.containing_owners((F(1000001,1000000),F(1,2),F(1,2)),[0,1]),[1])
            self.assertEqual(index.containing_owners((F(999999,1000000),F(1,2),F(1,2)),[0,1]),[0])
            for face in shells[0]:
                if all(v.point[0]==1 for v in face.vertices):
                    self.assertEqual(set(index.possible_owners(face)),{0,1})

        def test_every_bound_query_and_local_index_build_is_charged(self):
            shells=[helpers.box(kernel,(0,0,0,1,1,1),0),helpers.box(kernel,(8,8,8,9,9,9),1)]
            budget=kernel.Budget();index=subject.OriginalFaceIndex([f for s in shells for f in s],budget,kernel)
            self.assertEqual(index.work['componentBoundsBuilt'],2)
            self.assertEqual(index.work['localIndexRootsBuilt'],2)
            self.assertGreater(index.nodes,1)
            point=(F(1,2),F(1,2),F(1,2));before=budget.operations
            self.assertEqual(index.containing_owners(point,[0,1]),[0])
            self.assertEqual(index.containing_owners(point,[0,1]),[0])
            self.assertEqual(budget.operations-before,4)
            before=budget.operations;index.possible_owners(shells[0][0])
            self.assertEqual(budget.operations-before,3)
            exhausted=kernel.Budget(maximum_operations=3);exhausted.step(2)
            with self.assertRaisesRegex(kernel.UnionFailure,'operation budget'):
                subject.OriginalFaceIndex([f for s in shells for f in s],exhausted,kernel)

        def test_budget_refusal_keeps_live_membership_counters(self):
            shells=[helpers.box(kernel,(0,0,0,1,1,1),0)]
            budget=kernel.Budget(maximum_operations=1500)
            with self.assertRaisesRegex(kernel.UnionFailure,'operation budget') as caught:
                subject.surface_union(shells,budget,kernel=kernel)
            self.assertEqual(budget.union_work['stage'],'source_arrangement')
            self.assertGreater(budget.union_work['componentPruningWork']['ownerRayQueries'],0)
            notes=[json.loads(n) for n in caught.exception.__notes__ if n.startswith('{')]
            note=next(n for n in notes if 'operations' in n)
            self.assertEqual(note['unionWork']['componentPruningWork'],budget.union_work['componentPruningWork'])

        def test_one_owner_exact_global_index_reuse_preserves_all_facets(self):
            shell=helpers.box(kernel,(0,0,0,1,1,1),0)
            budget=kernel.Budget();index=subject.OriginalFaceIndex(shell,budget,kernel)
            self.assertIs(index.owner_roots[0],index.root)
            self.assertEqual(index.work['exactGlobalRootReuses'],1)
            self.assertEqual(len(index.owner_faces[0]),len(shell))
            self.assertEqual(index.containing_owners((F(1),F(1,2),F(1,2)),[0]),[0])

    for case in cases:
        def check(self,case=case):
            shells=shells_for(case);snapshot=repr(shells)
            lanes=[]
            for lane,implementation in [('reference_f365',unpruned),('component_pruned',subject)]:
                budget=kernel.Budget();start=time.monotonic()
                boundary,work=implementation.surface_union(shells,budget,kernel=kernel)
                self.assertEqual(helpers.surface_measure(boundary,kernel),
                                 (F(case['expectedArea']),F(case['expectedVolume'])))
                refusal=None
                if case.get('mustRefuseSingular'):
                    with self.assertRaisesRegex(kernel.UnionFailure,'manifold|disconnected') as error:
                        kernel.boundary_mesh(boundary,budget)
                    refusal=str(error.exception)
                else:
                    mesh=kernel.boundary_mesh(boundary,budget)
                    self.assertEqual(F(mesh['exactUnionVolumeSourceUnitsCubed']),F(case['expectedVolume']))
                lanes.append(dict(lane=lane,seconds=time.monotonic()-start,operations=budget.operations,
                                  boundary=boundary,work=work,refusal=refusal))
            baseline,candidate=lanes
            coverage=helpers.load('pruning_strip_source_coverage',Path(__file__).with_name('test_source_arrangement_strips.py'))
            coverage.assert_source_equivalent(self,kernel,baseline['boundary'],candidate['boundary'],shells,helpers)
            self.assertEqual(candidate['refusal'],baseline['refusal'])
            self.assertEqual(repr(shells),snapshot)
            ratio=candidate['seconds']/baseline['seconds']
            row=dict(fixture=case['id'],kind='frozen_component_pruning_exact_full_pipeline_parity',
                     referenceSeconds=baseline['seconds'],candidateSeconds=candidate['seconds'],ratio=ratio,
                     referenceOperations=baseline['operations'],candidateOperations=candidate['operations'],
                     operationDelta=candidate['operations']-baseline['operations'],
                     boundaryCells=len(candidate['boundary']),area=case['expectedArea'],volume=case['expectedVolume'],
                     sourceCoverageAndFieldsExact=True,refusal=candidate['refusal'],
                     pruningWork=candidate['work']['componentPruningWork'])
            records.append(row)
            self.assertLess(ratio,10,'Full-pipeline parity slowdown >=10x')
            if case['id']=='strict_outside_far_box' or case.get('requireTotalWorkReduction'):
                self.assertLess(candidate['operations'],baseline['operations'])
            if case.get('ownOwnerMayNotBePruned'):
                self.assertGreater(candidate['work']['componentPruningWork']['ownersRetainedAtPoint'],0)
                self.assertGreater(candidate['work']['componentPruningWork']['ownerRayQueries'],0)
        setattr(Controls,'test_frozen_pruning_'+case['id'],check)
    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
