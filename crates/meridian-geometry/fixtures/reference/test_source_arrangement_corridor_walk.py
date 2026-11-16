"""Complete, unforced indexed corridor work and exact candidate-order controls."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import time
import unittest


def corridor_walk_suite(kernel, subject, helpers, records):
    ref = Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/corridor-walk-reference'
    before = helpers.load('corridor_walk_exact_6f96d88', ref/'before-6f96d88.py')

    class Controls(unittest.TestCase):
        def test_complete_corridor_reuse_preserves_candidates_fields_and_contacts(self):
            pins = json.loads((ref/'FROZEN.json').read_text())['inputs']
            for name, source in [('before-6f96d88.py', 'exact-source-arrangement.py'), ('CASES.json', 'CASES.json')]:
                self.assertEqual(hashlib.sha256((ref/name).read_bytes()).hexdigest(), pins[source])
            cases = json.loads((ref/'CASES.json').read_text())['cases']
            for case in cases:
                if case['id'] not in ('ordinary_overlap', 'dense_exposed_grid_3', 'dense_exposed_grid_4'):
                    continue
                with self.subTest(fixture=case['id']):
                    shells = [helpers.box(kernel, bounds, i) for i, bounds in enumerate(case['boxes'])]
                    if matrix := case['matrix']:
                        shells = [[kernel.polygon([
                            kernel.Vertex(tuple(sum(F(row[k])*v.point[k] for k in range(3)) for row in matrix), v.uv)
                            for v in face.vertices], face.source, face.style) for face in shell] for shell in shells]
                    runs, outputs = [], []
                    for module in (before, subject):
                        cls = module.FacetRegions
                        old_corridor, old_recover, old_flip = cls.local_corridor, cls.recover_indexed, cls.flip_candidate
                        budget = kernel.Budget()
                        work_cost = dict(corridorCalls=0, corridorWalks=0, corridorWalkOperations=0, corridorReuses=0,
                                         indexedRecoveryOperations=0, longestWalk=0)
                        candidates = []
                        def corridor(region, *args):
                            entry = region.budget.operations
                            reused = getattr(region, 'recovery_corridor', None) is not None
                            found = old_corridor(region, *args)
                            work_cost['corridorCalls'] += 1
                            work_cost['corridorWalks'] += int(not reused)
                            work_cost['corridorReuses'] += int(reused)
                            work_cost['corridorWalkOperations'] += region.budget.operations-entry
                            work_cost['longestWalk'] = max(work_cost['longestWalk'], len(found or ()))
                            return found
                        def recover(region, a, b):
                            entry = region.budget.operations
                            candidates.append(('target', a, b))
                            try:
                                return old_recover(region, a, b)
                            finally:
                                work_cost['indexedRecoveryOperations'] += region.budget.operations-entry
                        def flip(region, edge, entries):
                            found = old_flip(region, edge, entries)
                            candidates.append(('candidate', edge, tuple(entries), found))
                            return found
                        cls.local_corridor, cls.recover_indexed, cls.flip_candidate = corridor, recover, flip
                        start = time.monotonic()
                        try:
                            polygons, work = module.surface_union(shells, budget, kernel=kernel)
                            mesh = kernel.boundary_mesh(polygons, budget)
                        finally:
                            cls.local_corridor, cls.recover_indexed, cls.flip_candidate = old_corridor, old_recover, old_flip
                        self.assertEqual(F(mesh['exactUnionVolumeSourceUnitsCubed']), F(case['volume']))
                        outputs.append((polygons, mesh, work['facetRegionWitness'], candidates))
                        runs.append(dict(seconds=time.monotonic()-start, completeOperations=budget.operations,
                                         triangles=len(mesh['faces']), candidateEvents=len(candidates), **work_cost))
                    self.assertEqual(outputs[0], outputs[1])
                    old, new = runs
                    self.assertEqual(old['corridorCalls'], new['corridorCalls'])
                    self.assertEqual(old['corridorWalks']-new['corridorWalks'], new['corridorReuses'])
                    if case['id'] == 'ordinary_overlap':
                        self.assertEqual(old['completeOperations'], new['completeOperations'])
                        self.assertEqual(old['corridorWalks'], new['corridorWalks'])
                    else:
                        self.assertGreaterEqual(old['longestWalk'], 5)
                        for key in ('corridorWalks', 'corridorWalkOperations', 'indexedRecoveryOperations', 'completeOperations'):
                            self.assertLess(new[key], old[key])
                        self.assertEqual(old['indexedRecoveryOperations']-new['indexedRecoveryOperations'],
                                         old['completeOperations']-new['completeOperations'])
                    records.append(dict(kind='corridor_walk_complete_work', fixture=case['id'], runs=runs))

    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
