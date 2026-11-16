"""Matched exact complete work and signed section boundary controls."""
from fractions import Fraction as F
import hashlib
import itertools
import json
from pathlib import Path
from types import SimpleNamespace
import unittest


def section_sign_suite(kernel, subject, helpers, records):
    ref = Path(__file__).resolve().parents[2]/'docs/fleet-physical/source-arrangement/section-sign-reference'
    before = helpers.load('section_sign_exact_311b3212', ref/'before-311b3212.py')
    manifest = json.loads((ref/'FROZEN.json').read_text())
    cases = json.loads((ref/'CASES.json').read_text())['cases']

    class Controls(unittest.TestCase):
        def test_signed_triangles_and_cache_admission_boundaries(self):
            plane = SimpleNamespace(normal=(F(0), F(0), F(1)), offset=F(0))
            xy = [(0, 0), (2, 0), (0, 2)]
            for heights in itertools.product((-2, 0, 3), repeat=3):
                with self.subTest(heights=heights):
                    points = [(F(x), F(y), F(z)) for (x, y), z in zip(xy, heights)]
                    face = kernel.polygon([kernel.Vertex(p, (F(0), F(0))) for p in points], (0, 0), 'test')
                    contacts = set(p for p in points if p[2] == 0)
                    for m, n in itertools.combinations(points, 2):
                        if min(m[2], n[2]) < 0 < max(m[2], n[2]):
                            t = m[2]/(m[2]-n[2])
                            contacts.add(tuple((1-t)*a+t*b for a, b in zip(m, n)))
                    expected = (list(zip(points, points[1:]+points[:1])), True) if heights == (0, 0, 0) else (
                        [(min(contacts), max(contacts))] if contacts else [], False)
                    old, new = kernel.Budget(), kernel.Budget()
                    self.assertEqual(before._section(plane, face, old, kernel), expected)
                    self.assertEqual(subject._section(plane, face, new, kernel, classify_strict_side=True), expected)
                    self.assertEqual(old.operations-new.operations, 2 if not contacts else 0)

            shell = helpers.box(kernel, (0, 0, 2, 1, 1, 3), 0)
            for room in (0, 1):
                with self.subTest(cacheRoom=room):
                    budget = kernel.Budget()
                    index = subject.OriginalFaceIndex(shell, budget, kernel)
                    budget.maximum_polygons = index.nodes+room
                    key = index.section_plane(plane)
                    self.assertEqual(index.section_for(plane, 0, key), ((), False))
                    self.assertEqual(len(index.section_cache), room)
                    self.assertEqual(index.section_work['uncachedAtStorageBound'], 1-room)
            budget = kernel.Budget(maximum_operations=3)
            with self.assertRaisesRegex(kernel.UnionFailure, 'operation budget'):
                subject._section(plane, shell[0], budget, kernel, classify_strict_side=True)
            self.assertEqual(budget.operations, 4)

        def test_complete_oblique_collection_saves_charged_work_at_both_storage_regimes(self):
            for name, pin in manifest['sha256'].items():
                self.assertEqual(hashlib.sha256((ref/name).read_bytes()).hexdigest(), pin)
            pressures = manifest['supplementalPressureControls']['maximumPolygonsByCase']
            for case in cases:
                shells = [helpers.box(kernel, bounds, i) for i, bounds in enumerate(case['boxes'])]
                if matrix := case['matrix']:
                    shells = [[kernel.polygon([
                        kernel.Vertex(tuple(sum(F(row[k])*v.point[k] for k in range(3)) for row in matrix), v.uv)
                        for v in face.vertices], face.source, face.style) for face in shell] for shell in shells]
                modes = [100000]+([pressures[case['id']]] if case['id'] in pressures else [])
                for storage in modes:
                    with self.subTest(case=case['id'], storage=storage):
                        runs, outputs = [], []
                        for module in (before, subject):
                            index_type = module.OriginalFaceIndex
                            old_lookup, old_section = index_type.section_for, module._section
                            budget = kernel.Budget(maximum_polygons=storage)
                            calls = []
                            cost = dict(sectionLookupConstruction=0, signedMisses=0, separatedSignedMisses=0,
                                        noRoomOnlySignedMisses=0)
                            def measured_lookup(index, face, ident, plane, **options):
                                entry = index.budget.operations
                                no_room_only = (module is subject and not options.get('classify_strict_side') and
                                                index.nodes+len(index.section_cache)+index.cached_section_endpoints+
                                                module._retained_region_slots(budget) >= storage)
                                signed_before = cost['signedMisses']
                                result = old_lookup(index, face, ident, plane, **options)
                                if no_room_only:
                                    cost['noRoomOnlySignedMisses'] += cost['signedMisses']-signed_before
                                cost['sectionLookupConstruction'] += index.budget.operations-entry
                                calls.append((face.source, index.faces[ident].source, result))
                                return result
                            def measured_section(*args, **options):
                                result = old_section(*args, **options)
                                if options.get('classify_strict_side'):
                                    cost['signedMisses'] += 1
                                    cost['separatedSignedMisses'] += int(result == ([], False))
                                return result
                            index_type.section_for, module._section = measured_lookup, measured_section
                            try:
                                polygons, work = module.surface_union(shells, budget, kernel=kernel)
                                mesh = kernel.boundary_mesh(polygons, budget)
                            finally:
                                index_type.section_for, module._section = old_lookup, old_section
                            self.assertEqual(F(mesh['exactUnionVolumeSourceUnitsCubed']), F(case['volume']))
                            outputs.append((polygons, mesh, work['facetRegionWitness'], calls, work['sectionReuseWork']))
                            runs.append(dict(completeOperations=budget.operations, triangles=len(mesh['faces']),
                                             sections=len(calls), reuse=work['sectionReuseWork'], **cost))
                        self.assertEqual(outputs[0], outputs[1])
                        old, new = runs
                        if case['id'] == 'ordinary_overlap':
                            self.assertEqual(old, new)
                        else:
                            self.assertGreater(new['signedMisses'], 0)
                            self.assertGreater(new['separatedSignedMisses'], 0)
                            self.assertLess(new['completeOperations'], old['completeOperations'])
                            self.assertEqual(old['sectionLookupConstruction']-new['sectionLookupConstruction'],
                                             2*new['separatedSignedMisses'])
                        if storage < 100000:
                            self.assertGreater(new['reuse']['uncachedAtStorageBound'], 0)
                            self.assertGreater(new['noRoomOnlySignedMisses'], 0)
                        records.append(dict(kind='section_sign_complete_work', fixture=case['id'], storage=storage, runs=runs))

    return unittest.defaultTestLoader.loadTestsFromTestCase(Controls)
