"""Exact original-facet arrangement; independent of CSG fragments and edge walks.

This experimental corrector is not imported by any fleet consumer. It constructs
finite triangle-plane constraints from authenticated original shells, decomposes
one original facet into exact open cells, and retains only exposed source cells.
No field is averaged, no original facets are merged, and contacts are not erased.
"""

from collections.abc import Sequence
from itertools import islice
from fractions import Fraction
from functools import cmp_to_key
import json


def _retained_region_slots(budget):
    return (budget.union_work.get("facetRegionWitnessSlots", 0)
            +budget.union_work.get("sourceArrangementWorkSlots", 0))


def _sub(a, b):
    return tuple(x-y for x, y in zip(a, b))


def _cross2(a, b):
    return a[0]*b[1]-a[1]*b[0]


class PlanarArrangement:
    """Finite-segment vertical decomposition of a convex original-facet domain."""
    def __init__(self, domain, budget, failure):
        self.budget, self.failure = budget, failure
        self.work = dict(segmentPairs=0, slabs=0, classifiedCells=0, retainedCells=0,
                         pointContactChords=0, verticalContactChords=0,
                         phase="domain_validation")
        self.budget.union_work["currentArrangementWork"] = self.work
        self.budget.step()  # Initialize current-facet accounting before any refusal.
        self.domain = tuple(self.point(p) for p in domain)
        self.budget.polygons(len(self.domain))
        if len(self.domain) < 3:
            raise failure("Arrangement requires a positive convex domain")
        for i, a in enumerate(self.domain):
            b = self.domain[(i+1) % len(self.domain)]
            if a == b or any(self.orient(a, b, p) < 0 for p in self.domain):
                raise failure("Arrangement domain is not a simple positive convex polygon")
        if self.area(self.domain) <= 0:
            raise failure("Arrangement domain has no positive area")
        self.work["phase"] = "awaiting_constraints"

    def point(self, point):
        self.budget.step()
        if len(point) != 2 or any(type(v) not in (int, Fraction) for v in point):
            raise self.failure("Arrangement requires exact rational points")
        result = tuple(Fraction(v) for v in point)
        if any(max(v.numerator.bit_length(), v.denominator.bit_length()) > 4096 for v in result):
            raise self.failure("Arrangement rational point exceeds original 4096-bit bound")
        return result

    def orient(self, a, b, p):
        self.budget.step()
        return _cross2(_sub(b, a), _sub(p, a))

    def area(self, points):
        self.budget.step(len(points))
        return sum(_cross2(a, b) for a, b in zip(points, points[1:]+points[:1]))

    def ordered(self, values):
        def compare(a, b):
            self.budget.step()
            return (a > b)-(a < b)
        return sorted(values, key=cmp_to_key(compare))

    def clip_segment(self, first, second):
        first, second = self.point(first), self.point(second)
        return self._clip_points(first, second)

    def _clip_points(self, first, second):
        lower, upper = Fraction(0), Fraction(1)
        for a, b in zip(self.domain, self.domain[1:]+self.domain[:1]):
            x, y = self.orient(a, b, first), self.orient(a, b, second)
            if x < 0 and y < 0:
                return None
            if x < 0 <= y:
                lower = max(lower, x/(x-y))
            elif y < 0 <= x:
                upper = min(upper, x/(x-y))
        if lower > upper:
            return None
        delta = _sub(second, first)
        return (self.point(tuple(first[k]+lower*delta[k] for k in range(2))),
                self.point(tuple(first[k]+upper*delta[k] for k in range(2))))

    def cells(self, barriers, visible):
        segments, events, protected, raw_seen = set(), set(), set(), set()
        self.work.update(phase="cell_arrangement", rawConstraintCacheHits=0, rawConstraintClipCalls=0,
                         retainedAtomicCells=0, stripsExtended=0,
                         stripsContactBlocked=0, stripsConvexityRejected=0, endpointTouchesRetained=0,
                         artificialCollinearPointsRemoved=0, protectedPoints=0,
                         physicalSegmentMemberships=0, lastRequiredGeometrySlots=0,
                         peakAdmittedMembershipSlots=0)
        self.budget.union_work["currentArrangementWork"] = self.work
        result, active = [], {}
        chain_members, result_members, vertical_count, physical_members = 0, 0, 0, 0
        def required_slots(extra=0):
            return (len(segments)+len(events)+len(protected)+vertical_count
                    +physical_members+chain_members+result_members+extra
                    +_retained_region_slots(self.budget))
        def guard(extra=0):
            # Optional dedup state yields to required geometry, never extra room.
            required = required_slots(extra)
            self.work["lastRequiredGeometrySlots"] = required
            if required+len(raw_seen) > self.budget.maximum_polygons and raw_seen:
                self.budget.step(len(raw_seen))
                raw_seen.clear()
            self.budget.polygons(required+len(raw_seen))
            self.work["peakAdmittedMembershipSlots"] = max(
                self.work["peakAdmittedMembershipSlots"], required+len(raw_seen))
        def add(first, second, physical=True):
            first, second = self.point(first), self.point(second)
            self.budget.step()  # Exact raw-key construction/lookup, including hits.
            key = tuple(sorted((first, second)))
            if key in raw_seen:
                self.work["rawConstraintCacheHits"] += 1
                return
            self.work["rawConstraintClipCalls"] += 1
            clipped = self._clip_points(first, second)
            if clipped is not None:
                self.budget.step()
                a, b = sorted(clipped)
                segments.add((a, b))
                events.update((a[0], b[0]))
                if physical:
                    self.budget.step(2)
                    protected.update((a, b))
            guard()
            if required_slots()+len(raw_seen)+1 <= self.budget.maximum_polygons:
                self.budget.step()  # Retained dedup-key construction/storage.
                raw_seen.add(key)
                guard()
        for a, b in zip(self.domain, self.domain[1:]+self.domain[:1]):
            add(a, b)
        for a, b in barriers:
            add(a, b)
        guard(len(segments))  # Admit the additional physical membership set first.
        self.budget.step(len(segments))  # Charge every copied membership.
        physical_segments = set(segments)
        physical_members = len(physical_segments)
        self.work["physicalSegmentMemberships"] = physical_members
        vertical = {}
        for a, b in physical_segments:
            self.budget.step()
            if a[0] == b[0]:
                vertical.setdefault(a[0], []).append((a[1], b[1]))
                vertical_count += 1
                for point in set((a, b)):
                    lower = min(p[0] for p in self.domain)
                    upper = max(p[0] for p in self.domain)
                    add((lower, point[1]), (upper, point[1]), physical=False)
                    key = "pointContactChords" if a == b else "verticalContactChords"
                    self.work[key] += 1
        guard(len(segments))  # The sorted list temporarily coexists with its input.
        segments = self.ordered(segments)
        for i, (a, b) in enumerate(segments):
            for c, d in segments[i+1:]:
                self.budget.step()
                self.work["segmentPairs"] += 1
                delta, other = _sub(b, a), _sub(d, c)
                denominator = _cross2(delta, other)
                if denominator == 0:
                    continue
                t = _cross2(_sub(c, a), other)/denominator
                u = _cross2(_sub(c, a), delta)/denominator
                if 0 <= t <= 1 and 0 <= u <= 1:
                    intersection = self.point(tuple(a[k]+t*delta[k] for k in range(2)))
                    events.add(intersection[0])
                    self.budget.step(2)
                    if (a, b) in physical_segments and (c, d) in physical_segments:
                        protected.add(intersection)
                    guard()
        self.work["protectedPoints"] = len(protected)
        guard(len(events))
        events = self.ordered(events)

        def finish(run):
            nonlocal chain_members, result_members
            self.budget.step(len(run["lower"])+len(run["upper"]))
            guard(len(run["lower"])+len(run["upper"]))
            points = run["lower"]+list(reversed(run["upper"]))
            cleaned = []
            for point in points:
                self.budget.step()
                if not cleaned or point != cleaned[-1]:
                    cleaned.append(point)
            if len(cleaned) > 1 and cleaned[0] == cleaned[-1]:
                cleaned.pop()
            if len(cleaned) < 3 or self.area(cleaned) <= 0:
                raise self.failure("Canonical strip lost a positive exposed region")
            chain_members -= len(run["lower"])+len(run["upper"])
            result_members += len(cleaned)
            result.append(tuple(cleaned))
            self.work["retainedCells"] += 1
            guard()

        def extend(chain, point):
            nonlocal chain_members
            while len(chain) >= 2:
                self.budget.step()  # Protected-point lookup, including retained hits.
                if self.orient(chain[-2], chain[-1], point) != 0 or chain[-1] in protected:
                    break
                chain.pop();chain_members -= 1
                self.work["artificialCollinearPointsRemoved"] += 1
            self.budget.step()
            chain.append(point);chain_members += 1

        for left, right in zip(events, events[1:]):
            self.budget.step()
            self.work["slabs"] += 1
            middle, levels = (left+right)/2, {}
            for a, b in segments:
                self.budget.step()
                if not a[0] < middle < b[0]:
                    continue
                slope = (b[1]-a[1])/(b[0]-a[0])
                intercept = a[1]-slope*a[0]
                levels[slope*middle+intercept] = (slope, intercept)
            levels = [(y, levels[y]) for y in self.ordered(levels)]
            following = {}
            for (bottom, lower), (top, upper) in zip(levels, levels[1:]):
                sample = self.point((middle, (bottom+top)/2))
                if any(self.orient(a, b, sample) <= 0
                       for a, b in zip(self.domain, self.domain[1:]+self.domain[:1])):
                    continue
                self.budget.step()
                self.work["classifiedCells"] += 1
                decision = visible(sample)
                if type(decision) is not bool:
                    raise self.failure("Arrangement classifier must return an exact boolean")
                if not decision:
                    continue
                self.work["retainedAtomicCells"] += 1
                lo_left = self.point((left, lower[0]*left+lower[1]))
                lo_right = self.point((right, lower[0]*right+lower[1]))
                hi_left = self.point((left, upper[0]*left+upper[1]))
                hi_right = self.point((right, upper[0]*right+upper[1]))
                self.budget.step()  # Full-side continuation lookup, never a point join.
                key = (left, lo_left[1], hi_left[1])
                run = active.pop(key, None) if lo_left[1] < hi_left[1] else None
                if run is not None:
                    self.budget.step()  # Vertical-contact interval-index lookup.
                    blocked = False
                    for low, high in vertical.get(left, ()):
                        self.budget.step()
                        overlap_low, overlap_high = max(low, lo_left[1]), min(high, hi_left[1])
                        if (overlap_low < overlap_high
                                or low == high and lo_left[1] < low < hi_left[1]):
                            blocked = True
                            break
                        if overlap_low == overlap_high:
                            self.budget.step()  # Endpoint-only contact protection lookup.
                            if (left, overlap_low) not in protected:
                                blocked = True
                                break
                            self.work["endpointTouchesRetained"] += 1
                    convex = (self.orient(run["lower"][-2], run["lower"][-1], lo_right) >= 0
                              and self.orient(run["upper"][-2], run["upper"][-1], hi_right) <= 0)
                    if blocked or not convex:
                        self.work["stripsContactBlocked" if blocked else "stripsConvexityRejected"] += 1
                        finish(run);run = None
                if run is None:
                    self.budget.step(4)
                    run = {"lower": [lo_left, lo_right], "upper": [hi_left, hi_right]}
                    chain_members += 4
                else:
                    extend(run["lower"], lo_right)
                    extend(run["upper"], hi_right)
                    self.work["stripsExtended"] += 1
                guard(4)
                if lo_right[1] < hi_right[1]:
                    self.budget.step()
                    next_key = (right, lo_right[1], hi_right[1])
                    if next_key in following:
                        raise self.failure("Canonical strips overlap a complete vertical side")
                    following[next_key] = run
                else:
                    finish(run)
            for run in active.values():
                finish(run)
            active = following
        for run in active.values():
            finish(run)
        self.region_constraints, self.region_protected = physical_segments, protected
        self.work["phase"] = "complete"
        return result


class FacetRegions:
    """Exact constrained triangulation of one classified original-facet complex.

    Physical contacts are retained conservatively. Only auxiliary paired edges
    and unprotected vertices with a complete cycle/straight-path link disappear.
    Recovery uses certified two-triangle local flips and preserves all vertices.
    """
    def __init__(self, arrangement, cells):
        self.arrangement, self.budget = arrangement, arrangement.budget
        self.failure = arrangement.failure
        self.domain, self.cells = arrangement.domain, cells
        self.physical = arrangement.region_constraints
        self.protected = arrangement.region_protected
        self.triangles = []
        self.locked = set()
        self.active_slots=self.search_slots=self.temporary_slots=self.recovery_slots=0
        self.recovery_index=None
        self.recovery_corridor=None
        self.active={'source':arrangement.work.get('sourceFacet'),
                     'projectionAxes':getattr(arrangement,'region_axes',None),
                     'stage':'initial_state_admission','geometryEncoded':False,
                     'omissions':['Active geometry not yet admitted']}
        self.budget.union_work['activeFacetRegion']=self.active
        self.work = {"pairedAuxiliaryEdgesRemoved": 0, "unprotectedPointsRemoved": 0,
                     "protectedPointsRetained": 0, "pointInsertions": 0,
                     "constraintRecoveries": 0, "cavityTrianglesReplaced": 0,
                     "boundaryIntervalsVerified": 0, "peakSlots": 0,
                     "localDiagonalFlips":0,"recoveryBacktracks":0,
                     "indexedMonotoneFlips":0,"indexedDirectLocks":0,"indexedFallbacks":0,"indexedRollbackFlips":0}
        self.cancellations, self.removed_stars = [], []
        self.base_slots = (sum(len(c) for c in cells)+len(self.physical)*3+
                           len(self.protected)+len(self.domain))
        self.active["counters"]=self.work
        self.guard()
        self.prepare_active()

    def guard(self, extra=0):
        slots = self.current_slots()+extra
        self.active['requiredSlots'] = slots
        self.work['peakSlots'] = max(self.work['peakSlots'], slots)
        self.budget.polygons(slots)


    def orient(self, a, b, p):
        return self.arrangement.orient(a, b, p)

    def area(self, points):
        return self.arrangement.area(points)

    def edge(self, a, b):
        self.budget.step()
        return (a, b) if a < b else (b, a)

    def on(self, a, b, p):
        self.budget.step()
        return self.orient(a, b, p) == 0 and all(min(a[k], b[k]) <= p[k] <= max(a[k], b[k]) for k in range(2))

    def ordered(self, values):
        self.guard(len(values))
        return self.arrangement.ordered(values)

    def inside(self, polygon, point):
        return all(self.orient(a, b, point) >= 0 for a, b in zip(polygon, polygon[1:]+polygon[:1]))

    def sample(self, points):
        return self.arrangement.point(tuple(sum(p[k] for p in points)/len(points) for k in range(2)))

    def exposed(self, point):
        for cell in self.cells:
            self.budget.step()
            if self.inside(cell, point):
                return True
        return False

    def support_line(self, a, b):
        self.budget.step(4)
        dx, dy = _sub(b, a)
        if dx:
            return (0, dy/dx, a[1]-dy/dx*a[0])
        return (1, a[0])

    def physical_lines(self):
        self.guard(9*len(self.physical))
        lines = {}
        for c, d in self.physical:
            self.budget.step()
            if c != d:
                key = self.support_line(c, d)
                axis = key[0]
                lo, hi = sorted((c[axis], d[axis]))
                lines.setdefault(key, []).append((lo, hi))
        return lines

    def indexed_physical_edge(self, a, b, lines):
        key = self.support_line(a, b)
        self.budget.step()
        axis = key[0]
        lo, hi = sorted((a[axis], b[axis]))
        for left, right in lines.get(key, ()):
            self.budget.step()
            if left <= lo and hi <= right:
                return True
        return False

    def point_ranges(self, points):
        self.guard(8*len(points))
        return (self.ordered(points), self.ordered([(p[1], p[0]) for p in points]))

    def range_cuts(self, a, b, ranges):
        # Either coordinate bounds the search; order on that coordinate is exact.
        axis = int(a[0] == b[0])
        rows = ranges[axis]
        low, high = sorted((a[axis], b[axis]))
        def bound(value, inclusive):
            left, right = 0, len(rows)
            while left < right:
                self.budget.step()
                middle = (left+right)//2
                if rows[middle][0] < value or inclusive and rows[middle][0] == value:
                    left = middle+1
                else:
                    right = middle
            return left
        left, right = bound(low, False), bound(high, True)
        cuts = []
        for i in range(left, right):
            self.budget.step()
            p = rows[i] if axis == 0 else (rows[i][1], rows[i][0])
            if self.on(a, b, p):
                cuts.append(p)
        return cuts

    def build_recovery_index(self):
        self.guard(48*len(self.triangles))
        edges = self.mesh_edges()
        stars = {}
        for i, triangle in enumerate(self.triangles):
            for p in triangle:
                self.budget.step()
                stars.setdefault(p, set()).add(i)
        self.temporary_slots = 0
        self.recovery_slots = 4*len(edges)+15*len(self.triangles)+3*len(stars)
        self.recovery_index = edges, stars
        self.guard()

    def publish_target_lock(self, target):
        self.budget.step()
        if target not in self.locked:
            self.guard(6)
            self.budget.step()
            ids = [self.point_id(p) for p in target]
            self.locked.add(target)
            self.active['lockedConstraints'].append(ids)
            self.active_slots += 3
        self.active.update(stage='constraint_recovered', lastAction='target_present')

    def local_corridor(self, a, b, edges, stars):
        self.budget.step()
        if self.recovery_corridor is not None:
            return self.recovery_corridor
        if a not in stars or b not in stars:
            return None
        # A noded target does not pass through another mesh vertex. If there
        # is no unambiguous transverse corridor, let exact fallback handle it.
        first = None
        for i in stars[a]:
            self.budget.step()
            triangle = self.triangles[i]
            u, v = [p for p in triangle if p != a]
            if self.crosses(a, b, u, v):
                if first is not None:
                    return None
                first = i, self.edge(u, v)
        if first is None:
            return None
        i, edge = first
        corridor, seen = [], set()
        while True:
            self.guard(7*len(corridor)+10)
            self.budget.step()
            if edge in seen:
                return None
            seen.add(edge)
            if edge in self.locked:
                self.active['candidateEdge'] = [self.point_id(p) for p in edge]
                raise self.refusal('Facet region target crosses a locked physical constraint')
            entries = edges.get(edge, ())
            if len(entries) != 2 or not any(entry[0]==i for entry in entries):
                return None
            corridor.append(edge)
            i = entries[1][0] if entries[0][0] == i else entries[0][0]
            triangle = self.triangles[i]
            if b in triangle:
                return corridor
            exits = []
            for u, v in zip(triangle, triangle[1:]+triangle[:1]):
                other = self.edge(u, v)
                if other != edge and self.crosses(a, b, u, v):
                    exits.append(other)
            if len(exits) != 1:
                return None
            edge = exits[0]

    def commit_indexed_flip(self, candidate, edges, stars):
        i, j, t, u, new_edge = candidate
        self.guard(80)
        encoded = [[self.point_id(p) for p in triangle] for triangle in (t, u)]
        pending = {}
        star_updates = {}
        for k, triangle in ((i, self.triangles[i]), (j, self.triangles[j])):
            for a, b in zip(triangle, triangle[1:]+triangle[:1]):
                edge = self.edge(a, b)
                self.budget.step(3)
                pending[edge] = [entry for entry in pending.get(edge, edges[edge]) if entry[0] != k]
            for p in triangle:
                self.budget.step()
                star_updates.setdefault(p, [[], []])[0].append(k)
        for k, triangle in ((i,t),(j,u)):
            for a, b in zip(triangle, triangle[1:]+triangle[:1]):
                edge = self.edge(a, b)
                self.budget.step()
                pending.setdefault(edge, []).append((k,a,b))
            for p in triangle:
                self.budget.step()
                star_updates.setdefault(p, [[], []])[1].append(k)
        for entries in pending.values():
            self.budget.step()
            if len(entries)>2 or len(entries)==2 and entries[0][1:]!=entries[1][1:][::-1]:
                raise self.refusal('Facet region indexed flip lacks reciprocal incidence')
        # All charged work/admission precedes every coupled update.
        self.budget.step(20)
        for edge, entries in pending.items():
            if entries: edges[edge] = entries
            else: del edges[edge]
        for p, (remove, add) in star_updates.items():
            stars[p].difference_update(remove)
            stars[p].update(add)
        self.triangles[i], self.triangles[j] = t,u
        self.active['meshTriangles'][i], self.active['meshTriangles'][j] = encoded
        self.work['localDiagonalFlips'] += 1

    def recover_indexed(self, a, b):
        self.active.update(stage='constraint_recovery', target=[self.point_ids.get(a),self.point_ids.get(b)],
                           visitedStates=0, searchDepth=0)
        self.active['target']=[self.point_id(a),self.point_id(b)]
        target=self.edge(a,b)
        if self.recovery_index is None:
            self.build_recovery_index()
        edges, stars = self.recovery_index
        undo = []
        while True:
            self.budget.step()
            if target in edges:
                self.recovery_corridor = None
                self.temporary_slots = 0
                self.publish_target_lock(target)
                self.work['indexedDirectLocks'] += int(not undo)
                self.work['constraintRecoveries'] += int(bool(undo))
                self.search_slots = 0
                return
            corridor = self.local_corridor(a,b,edges,stars)
            self.recovery_corridor = None
            choice = None
            if corridor is not None:
                self.temporary_slots = 4*len(corridor)
                for position, edge in enumerate(corridor):
                    self.budget.step()
                    candidate = self.flip_candidate(edge,edges[edge])
                    if candidate is not None and candidate[-1] not in edges and not self.crosses(a,b,*candidate[-1]):
                        choice = position,edge,candidate
                        break
            if choice is None:
                # Reverse edits do not use the corridor. Release its list
                # before dropping the reservation or admitting rollback work.
                del corridor
                self.temporary_slots = 0
                # A successful monotone prefix must not narrow fallback's
                # reachable states. Restore the original target-entry mesh.
                while undo:
                    self.commit_indexed_flip(undo[-1],edges,stars)
                    undo.pop()
                    self.search_slots -= 9
                    self.work['indexedRollbackFlips'] += 1
                    self.active['lastAction'] = 'indexed_fallback_rollback'
                self.recovery_index = None
                self.recovery_slots = 0
                del edges, stars
                self.work['indexedFallbacks'] += 1
                self.recover(a,b)
                return
            position,old_edge,candidate = choice
            # A convex flip changes exactly one edge. The old diagonal
            # crosses the target and the replacement does not; every other
            # transverse edge and its target order survives unchanged.
            # Walk afresh for a remaining single crossing.
            retain = len(corridor) > 2
            if retain:
                self.budget.step(len(corridor)-position)
            i,j,_,_,_ = candidate
            self.guard(9)
            self.budget.step(5)
            undo.append((i,j,self.triangles[i],self.triangles[j],old_edge))
            self.search_slots += 9
            self.commit_indexed_flip(candidate,edges,stars)
            self.work['indexedMonotoneFlips'] += 1
            self.active['lastAction'] = 'indexed_monotone_flip'
            if retain:
                del corridor[position]
                self.recovery_corridor = corridor
                self.temporary_slots = 4*len(corridor)
            else:
                self.temporary_slots = 0
            del corridor

    def complex(self):
        self.active['stage']='classified_complex_noding'
        total=sum(len(c) for c in self.cells)
        self.guard(total)
        self.budget.step(total)
        points=set(p for cell in self.cells for p in cell)
        ranges=self.point_ranges(points)
        self.temporary_slots=4*len(points)
        physical_lines=self.physical_lines()
        self.temporary_slots+=9*len(self.physical)
        incidence,links={},{}
        incid_members=link_members=removed_members=0
        boundary,keep=[],set()
        def graph_guard(extra=0):
            self.guard(len(points)+4*len(incidence)+4*incid_members+3*len(links)
                       +3*link_members+2*len(boundary)+len(keep)
                       +5*len(self.cancellations)+removed_members+extra)
        graph_guard(len(self.protected))
        keep.update(self.protected)
        for cell_id,cell in enumerate(self.cells):
            if self.area(cell)<=0:raise self.refusal('Facet region requires positive classified cells')
            noded=[]
            for a,b in zip(cell,cell[1:]+cell[:1]):
                graph_guard(4*len(points)+len(noded))
                cuts=self.range_cuts(a,b,ranges)
                if b<a:cuts.reverse()
                added = len(cuts)-1
                # Old noded memberships, live cuts, the slice and the
                # destination list's new memberships coexist on extension.
                graph_guard(len(noded)+len(cuts)+2*added)
                noded.extend(cuts[:-1])
                del cuts
            for i,a in enumerate(noded):
                b=noded[(i+1)%len(noded)]
                self.active['lastCellEdge']=[cell_id,self.point_id(a),self.point_id(b)]
                graph_guard(len(noded)+16)
                key=self.edge(a,b);entries=incidence.setdefault(key,[])
                entries.append((a,b,cell_id));incid_members+=1
                if len(entries)>2 or len(entries)==2 and entries[0][:2]!=(b,a):
                    raise self.refusal('Facet region lacks reciprocal positive-area edge incidence')
                links.setdefault(a,[]).append((noded[i-1],b));link_members+=1
            del noded
        for (a,b),entries in incidence.items():
            self.budget.step();graph_guard(8)
            if len(entries)==1:boundary.append(entries[0][:2])
            elif not self.indexed_physical_edge(a,b,physical_lines):self.cancellations.append((a,b,entries[0][2],entries[1][2]))
        for point,sectors in links.items():
            self.budget.step()
            if point in self.protected:continue
            self.active.update(stage='classified_vertex_star',pendingStarPoint=self.point_id(point),
                               pendingStarSectors=len(sectors))
            graph_guard(8*len(sectors)+len(points))
            graph={}
            for a,b in sectors:
                self.budget.step();graph.setdefault(a,[]).append(b);graph.setdefault(b,[]).append(a)
            ends=[p for p,n in graph.items() if len(n)==1]
            valid=all(len(n) in (1,2) for n in graph.values())
            reached,pending=set(),[next(iter(graph))]
            while pending:
                self.budget.step();value=pending.pop()
                if value not in reached:
                    reached.add(value);self.budget.step(len(graph[value]))
                    pending.extend(p for p in graph[value] if p not in reached)
            cycle=valid and not ends and len(reached)==len(graph)
            path=valid and len(ends)==2 and len(reached)==len(graph) and self.on(ends[0],ends[1],point)
            if cycle or path:
                graph_guard(3+3*len(sectors))
                self.removed_stars.append((point,tuple(sectors),'cycle' if cycle else 'straight_boundary_path'))
                removed_members+=3+3*len(sectors)
            else:graph_guard(1);keep.add(point)
        self.work['pairedAuxiliaryEdgesRemoved']=len(self.cancellations)
        self.work['unprotectedPointsRemoved']=len(self.removed_stars)
        graph_guard()
        self.base_slots+=2*len(boundary)+len(keep)+5*len(self.cancellations)+removed_members
        self.temporary_slots=0
        self.boundary,self.keep=boundary,keep
        return boundary,keep


    def insert(self, point):
        self.budget.step()
        self.budget.step(len(self.triangles))
        if any(point in triangle for triangle in self.triangles):
            return
        replaced, generated = [], []
        self.guard(40*len(self.triangles)+20)
        for i, triangle in enumerate(self.triangles):
            self.budget.step()
            if not self.inside(triangle, point):
                continue
            replaced.append(i)
            for a, b in zip(triangle, triangle[1:]+triangle[:1]):
                self.budget.step()
                if self.orient(a, b, point) > 0:
                    generated.append((a, b, point))
        if not replaced or len(replaced) > 2:
            raise self.refusal("Facet region point lacks an authenticated triangle star")
        self.guard(4*len(generated)+len(replaced))
        self.budget.step(len(self.triangles)*(len(replaced)+1))
        self.replace_mesh([t for i,t in enumerate(self.triangles) if i not in replaced]+generated)
        self.work["pointInsertions"] += 1



    def recover(self, a, b):
        self.active.update(stage='constraint_recovery',target=[self.point_ids.get(a),self.point_ids.get(b)])
        self.point_id(a);self.point_id(b)
        self.replace_mesh(self.triangles)
        self.sync_locked()
        target=self.edge(a,b)
        if a==b:return
        if not all(any(p in t for t in self.triangles) for p in (a,b)):
            raise self.refusal('Facet region target endpoint is absent from current mesh')
        for c,d in self.locked:
            self.budget.step()
            if self.crosses(a,b,c,d):
                raise self.refusal('Facet region target crosses a locked physical constraint')
        initial=self.state_key();self.guard(4*len(self.triangles)+2)
        self.budget.step(3*len(self.triangles)+1)
        visited={initial};stack=[initial]
        width=4*len(self.triangles)+1
        self.search_slots=width+1
        initial_missing=True
        while stack:
            self.guard()
            self.active.update(searchDepth=len(stack),visitedStates=len(visited))
            edges=self.mesh_edges()
            if target in edges:
                if target not in self.locked:
                    self.guard(6);self.budget.step()
                    prepared_ids=[self.point_id(a),self.point_id(b)]
                    # No budgeted/admission work may interrupt joint publication.
                    self.locked.add(target)
                    self.active['lockedConstraints'].append(prepared_ids)
                    self.active_slots+=3
                self.active.update(stage='constraint_recovered',lastAction='target_present')
                self.search_slots=0;self.temporary_slots=0
                return
            chosen=None
            for edge in self.ordered(edges):
                self.budget.step()
                if not self.crosses(a,b,*edge):continue
                if edge in self.locked:
                    raise self.refusal('Facet region crossed recovery edge is locked')
                entries=edges[edge]
                if len(entries)!=2:
                    raise self.refusal('Facet region target crosses original triangular boundary')
                candidate=self.flip_candidate(edge,entries)
                if candidate is None:continue
                i,j,t,u,new_edge=candidate
                if new_edge in edges:
                    raise self.refusal('Facet region local flip would duplicate an existing diagonal')
                self.guard(12*len(self.triangles))
                key=self.replacement_state_key(stack[-1],candidate)
                self.budget.step(3*len(self.triangles)+1)
                if key in visited:continue
                score=int(self.crosses(a,b,*new_edge))
                if chosen is None or score<chosen[0]:chosen=(score,candidate,key)
                if score==0:break
            del edges
            self.temporary_slots=0
            if chosen is None:
                stack.pop()
                self.search_slots=len(visited)*width+len(stack)
                if not stack:break
                self.active['lastAction']='backtrack'
                self.restore_state(stack[-1]);self.work['recoveryBacktracks']+=1
                continue
            _,candidate,key=chosen
            i,j,t,u,_=candidate
            self.guard(width+1)
            self.budget.step(3*len(self.triangles)+4)
            # Both replacements and their diagnostic IDs are prepared before mutation.
            encoded_t=[self.point_id(p) for p in t];encoded_u=[self.point_id(p) for p in u]
            self.triangles[i],self.triangles[j]=t,u
            self.active['meshTriangles'][i],self.active['meshTriangles'][j]=encoded_t,encoded_u
            visited.add(key);stack.append(key)
            self.search_slots=len(visited)*width+len(stack)
            self.work['localDiagonalFlips']+=1
            self.work['constraintRecoveries']+=int(initial_missing)
            initial_missing=False
            self.active['lastAction']='strict_convex_local_flip'
        self.search_slots=0;self.temporary_slots=0
        raise self.refusal('Facet region has no unvisited admissible local recovery state')


    def boundary_support(self, edges):
        lines = {}
        for a, b in edges:
            self.budget.step(4)
            delta = _sub(b, a)
            normal = (-delta[1], delta[0])
            divisor = next(abs(v) for v in normal if v)
            key = (tuple(v/divisor for v in normal), _cross2(a, b)/divisor)
            axis = 0 if delta[0] else 1
            low, high = sorted((a[axis], b[axis]))
            lines.setdefault(key, []).append((low, high))
        result = {}
        for key, intervals in lines.items():
            merged = []
            for low, high in self.ordered(intervals):
                self.budget.step()
                if merged and low <= merged[-1][1]:
                    merged[-1] = (merged[-1][0], max(high, merged[-1][1]))
                else:
                    merged.append((low, high))
            result[key] = tuple(merged)
            self.work["boundaryIntervalsVerified"] += len(merged)
        return result

    def reconstruct(self):
        entry=self.budget.operations
        boundary, keep = self.complex()
        self.work["indexedComplexWork"]=self.budget.operations-entry
        entry=self.budget.operations
        self.replace_mesh([tuple(self.domain)])
        for point in self.ordered(keep):
            self.insert(point)
        ranges=self.point_ranges(keep)
        self.temporary_slots=4*len(keep)
        constraints = set()
        for group in (self.physical,boundary):
            for a,b in group:
                self.budget.step()
                if a==b:
                    continue
                self.guard(3*len(constraints)+3*len(keep))
                cuts=self.range_cuts(a,b,ranges)
                for c,d in zip(cuts,cuts[1:]):
                    self.guard(3*len(constraints)+3*len(cuts)+3)
                    constraints.add(self.edge(c,d))
                del cuts
        del ranges
        self.temporary_slots=0
        self.base_slots += len(constraints)*3
        self.work["indexedPointAndConstraintBuildWork"]=self.budget.operations-entry
        entry=self.budget.operations
        ordered_constraints=self.ordered(constraints)
        self.base_slots+=len(ordered_constraints)
        for a,b in ordered_constraints:
            self.recover_indexed(a,b)
        self.recovery_index=None
        self.recovery_slots=0
        self.base_slots-=len(ordered_constraints)
        del ordered_constraints
        self.work["indexedRecoveryWork"]=self.budget.operations-entry
        selected = []
        self.active.update(stage="select_exposed_triangles",selectedTriangleIndices=[])
        for triangle_id,triangle in enumerate(self.triangles):
            self.budget.step()
            center = self.sample(triangle)
            if self.exposed(center):
                self.guard(1)
                selected.append(triangle)
                self.active["selectedTriangleIndices"].append(triangle_id)
                self.active_slots+=1
        if sum(self.area(t) for t in selected) != sum(self.area(c) for c in self.cells):
            raise self.refusal("Facet region reconstruction changed total exposed area")
        edges = {}
        for triangle in selected:
            for a, b in zip(triangle, triangle[1:]+triangle[:1]):
                key = self.edge(a, b)
                entries = edges.setdefault(key, [])
                entries.append((a, b))
                if len(entries) > 2 or len(entries) == 2 and entries[0] != (b, a):
                    raise self.refusal("Facet region output has nonreciprocal positive-area incidence")
        result_boundary = [v[0] for v in edges.values() if len(v) == 1]
        self.guard(len(edges)*4+len(selected)*4+len(result_boundary)*2)
        if self.boundary_support(result_boundary) != self.boundary_support(boundary):
            raise self.refusal("Facet region reconstruction changed oriented exposed boundary support")
        output_points = set(p for t in selected for p in t)
        for point in self.protected:
            self.budget.step()
            if self.exposed(point):
                if point not in output_points:
                    raise self.refusal("Facet region reconstruction erased a protected contact star")
                self.work["protectedPointsRetained"] += 1
        for a, b in constraints:
            self.budget.step()
            center = self.sample((a,b))
            if self.exposed(center) and self.edge(a, b) not in edges:
                raise self.refusal("Facet region reconstruction erased an exposed finite contact interval")
        self.work.update(classifiedCells=len(self.cells), regionTriangles=len(selected),
                         exactOrientedBoundarySupport=True, exactArea=True,
                         allProtectedContactStarsRetained=True)
        self.active.update(stage="local_region_complete",complete=True)
        return selected, self.work

    def current_slots(self):
        return (self.base_slots+4*len(self.triangles)+3*len(self.locked)
                +self.active_slots+self.search_slots+self.temporary_slots+self.recovery_slots
                +_retained_region_slots(self.budget))

    def refusal(self, message):
        error = self.failure(message)
        error.add_note(json.dumps({'operations':self.budget.operations,
                                   'unionWork':self.budget.union_work},sort_keys=True))
        return error

    def prepare_active(self):
        members = sum(len(c) for c in self.cells)+len(self.protected)+len(self.domain)
        self.guard(12*members+3*len(self.physical)+64)
        self.budget.step(members)
        points = set(p for cell in self.cells for p in cell)
        points.update(self.protected);points.update(self.domain)
        ordered = self.arrangement.ordered(points)
        self.points, self.point_ids = [], {}
        for point in ordered:
            self.budget.step(3)
            point = self.arrangement.point(point)
            self.point_ids[point] = len(self.points)
            self.points.append(point)
        self.active.update(points=[[str(v) for v in p] for p in self.points],
                           domain=[self.point_ids[p] for p in self.domain],
                           classifiedCells=[[self.point_ids[p] for p in c] for c in self.cells],
                           physicalSegments=[[self.point_ids[a],self.point_ids[b]] for a,b in self.physical],
                           protectedPoints=[self.point_ids[p] for p in self.protected],
                           meshTriangles=[],lockedConstraints=[],geometryEncoded=True,
                           omissions=[],stage='classified_complex')
        self.active_slots = (8*len(self.points)+sum(len(c) for c in self.cells)
                             +3*len(self.physical)+len(self.protected)+64)
        self.guard()
        self.budget.step(members+2*len(self.physical))

    def point_id(self, point):
        self.budget.step()
        if point not in self.point_ids:
            point = self.arrangement.point(point)
            self.guard(8)
            self.budget.step(3)
            self.point_ids[point] = len(self.points)
            self.points.append(point)
            self.active['points'].append([str(v) for v in point])
            self.active_slots += 8
        return self.point_ids[point]

    def replace_mesh(self, triangles):
        self.guard(4*len(triangles))
        self.budget.step(3*len(triangles))
        encoded = [[self.point_id(p) for p in t] for t in triangles]
        old = len(self.active['meshTriangles'])
        self.triangles = triangles
        self.active['meshTriangles'] = encoded
        self.active_slots += 4*(len(encoded)-old)
        self.guard()

    def sync_locked(self):
        self.guard(3*len(self.locked))
        self.budget.step(2*len(self.locked))
        encoded = [[self.point_id(a),self.point_id(b)] for a,b in self.locked]
        old = len(self.active['lockedConstraints'])
        self.active['lockedConstraints'] = encoded
        self.active_slots += 3*(len(encoded)-old)

    def crosses(self, a, b, c, d):
        return (self.orient(a,b,c)*self.orient(a,b,d)<0
                and self.orient(c,d,a)*self.orient(c,d,b)<0)

    def mesh_edges(self):
        self.guard(36*len(self.triangles))
        edges = {}
        for i, triangle in enumerate(self.triangles):
            self.budget.step()
            for a,b in zip(triangle,triangle[1:]+triangle[:1]):
                key = self.edge(a,b)
                entries = edges.setdefault(key,[])
                entries.append((i,a,b))
                if len(entries)>2 or len(entries)==2 and entries[0][1:]!=(b,a):
                    self.active['candidateEdge']=[self.point_id(a),self.point_id(b)]
                    raise self.refusal('Facet region local recovery has nonreciprocal mesh incidence')
        self.temporary_slots = 4*len(edges)+12*len(self.triangles)
        return edges

    def state_key(self, replacement=None):
        self.guard(6*len(self.triangles))
        replacement = replacement or {}
        rows = []
        for i,triangle in enumerate(self.triangles):
            self.budget.step(4)
            ids = [self.point_id(p) for p in replacement.get(i,triangle)]
            # Fixed size three: four comparisons cover its maximum sort cost.
            rows.append(tuple(sorted(ids)))
        return tuple(self.arrangement.ordered(rows))

    def replacement_state_key(self, current, candidate):
        """Edit a canonical fallback key after an exact two-triangle flip."""
        i,j,t,u,_=candidate
        self.guard(10*len(current)+24)
        # Active mesh IDs are published with their actual triangles.
        self.budget.step(20)
        removed = [tuple(sorted(self.active["meshTriangles"][k])) for k in (i,j)]
        inserted = sorted(tuple(sorted(self.point_id(p) for p in triangle)) for triangle in (t,u))
        result = []
        next_insert = 0
        removed_count = 0
        for row in current:
            self.budget.step(3)
            if row in removed:
                removed_count += 1
                continue
            while next_insert < len(inserted):
                self.budget.step(2)
                if row < inserted[next_insert]:
                    break
                result.append(inserted[next_insert])
                next_insert += 1
            result.append(row)
        if removed_count != 2:
            raise self.refusal("Facet region fallback key lacks both indexed triangles")
        while next_insert < len(inserted):
            self.budget.step()
            result.append(inserted[next_insert])
            next_insert += 1
        return tuple(result)

    def restore_state(self, key):
        self.active['pendingParentMesh'] = key
        self.guard(8*len(key))
        triangles = []
        for row in key:
            self.budget.step()
            a,b,c = [self.points[i] for i in row]
            sign = self.orient(a,b,c)
            if sign==0:raise self.refusal('Facet region local recovery cannot restore a degenerate state')
            triangles.append((a,b,c) if sign>0 else (a,c,b))
        self.replace_mesh(triangles)
        self.active.pop('pendingParentMesh',None)

    def flip_candidate(self, edge, entries):
        (i,_,_),(j,_,_) = entries
        a,b = edge
        p = next(v for v in self.triangles[i] if v not in edge)
        q = next(v for v in self.triangles[j] if v not in edge)
        self.active['candidateEdge']=[self.point_id(a),self.point_id(b)]
        self.active['candidateOppositeVertices']=[self.point_id(p),self.point_id(q)]
        if self.orient(p,q,a)*self.orient(p,q,b)>=0:
            return None
        result=[]
        for triangle in [(p,q,a),(q,p,b)]:
            sign=self.orient(*triangle)
            if sign==0:raise self.refusal('Facet region local flip is not strictly positive')
            result.append(triangle if sign>0 else (triangle[0],triangle[2],triangle[1]))
        old_area=self.area(self.triangles[i])+self.area(self.triangles[j])
        if self.area(result[0])+self.area(result[1])!=old_area:
            raise self.refusal('Facet region local flip changes exact covered area')
        new_edge=self.edge(p,q)
        old_outer={self.edge(c,d) for t in [self.triangles[i],self.triangles[j]] for c,d in zip(t,t[1:]+t[:1]) if self.edge(c,d)!=edge}
        new_outer={self.edge(c,d) for t in result for c,d in zip(t,t[1:]+t[:1]) if self.edge(c,d)!=new_edge}
        if old_outer!=new_outer or len(old_outer)!=4:
            raise self.refusal('Facet region local flip lacks an unchanged four-edge disk boundary')
        return i,j,result[0],result[1],new_edge













_SECTION_INDEX_LEAF_SIZE = 8
_SECTION_DENSE_CANDIDATES = 4 * _SECTION_INDEX_LEAF_SIZE


class OriginalFaceIndex:
    """Charged exact original-triangle boxes for section and ray candidates."""
    def __init__(self, faces, budget, kernel):
        self.faces, self.budget, self.kernel = tuple(faces), budget, kernel
        self.work = budget.union_work.setdefault("componentPruningWork", {
            "componentBoundsBuilt": 0, "localIndexRootsBuilt": 0, "exactGlobalRootReuses": 0,
            "ownerFacetBoundsTests": 0, "ownersRejectedByFacetBounds": 0,
            "pointBoundsTests": 0, "ownersRejectedByPointBounds": 0,
            "ownersRetainedAtPoint": 0, "ownerRayQueries": 0,
            "rayFaceVisits": 0, "rayDirectionsTried": 0,
            "rayDirectionsAmbiguous": 0})
        self.boxes, self.owner_faces, self.owner_boxes = [], {}, {}
        for ident, face in enumerate(self.faces):
            budget.step()
            self.boxes.append((tuple(min(v.point[i] for v in face.vertices) for i in range(3)),
                               tuple(max(v.point[i] for v in face.vertices) for i in range(3))))
            budget.step()  # Charge ownership grouping, including existing-owner lookup.
            self.owner_faces.setdefault(face.source[0], []).append(ident)
        for owner, ids in self.owner_faces.items():
            budget.step(len(ids)+1)
            self.owner_boxes[owner] = (
                tuple(min(self.boxes[i][0][k] for i in ids) for k in range(3)),
                tuple(max(self.boxes[i][1][k] for i in ids) for k in range(3)))
            self.work["componentBoundsBuilt"] += 1
        self.section_cache = {}
        self.cached_section_endpoints = 0
        self.section_work = budget.union_work.setdefault("sectionReuseWork", {
            "planeKeysBuilt": 0, "lookups": 0, "hits": 0, "misses": 0,
            "storedEntries": 0, "storedEndpoints": 0, "uncachedAtStorageBound": 0})
        self.nodes = 0
        self.root = self.build(list(range(len(faces))))
        self.owner_roots = {}
        for owner, ids in self.owner_faces.items():
            budget.step()
            if len(self.owner_faces) == 1:
                # Identical complete face set, not an occupancy/regularity guess.
                self.owner_roots[owner] = self.root
                self.work["exactGlobalRootReuses"] += 1
            else:
                self.owner_roots[owner] = self.build(ids)
                self.work["localIndexRootsBuilt"] += 1

    def section_plane(self, face):
        self.budget.step()
        self.section_work["planeKeysBuilt"] += 1
        return self.kernel.oriented_plane(face.normal, face.offset)

    def section_for(self, face, ident, plane, *, classify_strict_side=False):
        self.budget.step()  # Every exact key/lookup, including successful reuse.
        self.section_work["lookups"] += 1
        key = (plane, ident)
        if key in self.section_cache:
            self.section_work["hits"] += 1
            return self.section_cache[key]
        self.section_work["misses"] += 1
        room = (self.budget.maximum_polygons-self.nodes-len(self.section_cache)
                -self.cached_section_endpoints-_retained_region_slots(self.budget))
        sections, same_plane = _section(face, self.faces[ident], self.budget, self.kernel,
                                       classify_strict_side=classify_strict_side or room < 1)
        endpoints = 2*len(sections)
        if 1+endpoints <= room:
            self.budget.step()  # Bounded immutable entry construction/retention.
            value = (tuple(sections), same_plane)
            self.section_cache[key] = value
            self.cached_section_endpoints += endpoints
            self.section_work.update(storedEntries=len(self.section_cache),
                                     storedEndpoints=self.cached_section_endpoints)
            return value
        self.section_work["uncachedAtStorageBound"] += 1
        return tuple(sections), same_plane

    def possible_owners(self, face):
        """Closed facet-box overlap is a conservative per-source candidate set."""
        self.budget.step()
        lower = tuple(min(v.point[k] for v in face.vertices) for k in range(3))
        upper = tuple(max(v.point[k] for v in face.vertices) for k in range(3))
        result = []
        for owner, (a, b) in self.owner_boxes.items():
            self.budget.step()
            self.work["ownerFacetBoundsTests"] += 1
            if all(a[k] <= upper[k] and lower[k] <= b[k] for k in range(3)):
                result.append(owner)
            else:
                self.work["ownersRejectedByFacetBounds"] += 1
        return result

    def containing_owners(self, point, possible):
        """Strict exclusion only; equality keeps face/edge/point contacts."""
        result = []
        for owner in possible:
            self.budget.step()  # Every owner-bound lookup/test, not just misses.
            lower, upper = self.owner_boxes[owner]
            self.work["pointBoundsTests"] += 1
            if all(lower[k] <= point[k] <= upper[k] for k in range(3)):
                result.append(owner)
                self.work["ownersRetainedAtPoint"] += 1
            else:
                self.work["ownersRejectedByPointBounds"] += 1
        return result

    def build(self, ids):
        self.budget.step(len(ids)+1)
        self.nodes += 1
        self.budget.polygons(self.nodes)
        lower = tuple(min(self.boxes[i][0][k] for i in ids) for k in range(3))
        upper = tuple(max(self.boxes[i][1][k] for i in ids) for k in range(3))
        if len(ids) <= _SECTION_INDEX_LEAF_SIZE:
            return lower, upper, None, None, tuple(ids)
        axis = max(range(3), key=lambda k: upper[k]-lower[k])
        def compare(a, b):
            self.budget.step()
            x, y = self.boxes[a][0][axis]+self.boxes[a][1][axis], self.boxes[b][0][axis]+self.boxes[b][1][axis]
            return (x > y)-(x < y) or (a > b)-(a < b)
        ordered = sorted(ids, key=cmp_to_key(compare))
        middle = len(ids)//2
        return lower, upper, self.build(ordered[:middle]), self.build(ordered[middle:]), ()

    def query(self, meets, root=None):
        pending = [self.root if root is None else root]
        result = []
        while pending:
            self.budget.step()
            lower, upper, left, right, ids = pending.pop()
            if not meets(lower, upper):
                continue
            if left is None:
                for index in ids:
                    self.budget.step()
                    if meets(*self.boxes[index]):
                        result.append(index)
            else:
                pending.extend((left, right))
        return result

    def near(self, face):
        lower = tuple(min(v.point[k] for v in face.vertices) for k in range(3))
        upper = tuple(max(v.point[k] for v in face.vertices) for k in range(3))
        return self.query(lambda a, b: all(a[k] <= upper[k] and lower[k] <= b[k] for k in range(3)))

    def ray(self, point, direction, owner):
        def meets(lower, upper):
            self.budget.step()
            enter, leave = Fraction(0), None
            for k in range(3):
                if direction[k] == 0:
                    if not lower[k] <= point[k] <= upper[k]:
                        return False
                else:
                    a, b = sorted(((lower[k]-point[k])/direction[k], (upper[k]-point[k])/direction[k]))
                    enter = max(enter, a)
                    leave = b if leave is None else min(leave, b)
                    if enter > leave:
                        return False
            return True
        self.budget.step()  # Component-root lookup/query entry is always charged.
        self.work["ownerRayQueries"] += 1
        return self.query(meets, self.owner_roots[owner])


def _inside_triangle(face, point, budget, kernel):
    signs = []
    for a, b in zip(face.vertices, face.vertices[1:]+face.vertices[:1]):
        budget.step()
        signs.append(kernel.dot(kernel.cross(kernel.subtract(b.point, a.point),
                                             kernel.subtract(point, a.point)), face.normal))
    return all(value >= 0 for value in signs), any(value == 0 for value in signs)


def _outside_occupied(point, normal, index, budget, kernel, possible=None):
    # A strict point-box exclusion remains outside for all sufficiently small
    # outward displacements. Inclusive equality retains every contact case.
    if possible is None:
        possible = tuple(index.owner_boxes)
    owners = index.containing_owners(point, possible)
    # Do not omit the current source owner: input_shell does not establish that
    # one owner contains only one connected regular boundary. Nested/disconnected
    # sheets keep the exact original component-parity semantics here.
    for owner in owners:
        attempt = 0
        while True:
            attempt += 1
            budget.step()
            index.work["rayDirectionsTried"] += 1
            direction = (Fraction(1), Fraction(attempt), Fraction(attempt*attempt))
            orientation = kernel.dot(direction, normal)
            if orientation == 0:
                continue
            if orientation < 0:
                direction = kernel.scale(direction, -1)
            parity, ambiguous = False, False
            for ident in index.ray(point, direction, owner):
                budget.step()
                index.work["rayFaceVisits"] += 1
                face = index.faces[ident]
                denominator = kernel.dot(face.normal, direction)
                numerator = face.offset-kernel.dot(face.normal, point)
                if denominator == 0:
                    if numerator == 0 and _inside_triangle(face, point, budget, kernel)[0]:
                        ambiguous = True
                        break
                    continue
                amount = numerator/denominator
                if amount <= 0:
                    continue
                hit = kernel.add(point, kernel.scale(direction, amount))
                inside, on_edge = _inside_triangle(face, hit, budget, kernel)
                if not inside:
                    continue
                if on_edge:
                    ambiguous = True
                    break
                parity = not parity
            if not ambiguous:
                if parity:
                    return True
                break
            index.work["rayDirectionsAmbiguous"] += 1
    return False


def _section(face, other, budget, kernel, *, classify_strict_side=False):
    values = []
    signs = 0
    for vertex in other.vertices:
        # One charged signed vertex evaluation, including its fixed sign class.
        budget.step()
        value = kernel.dot(face.normal, vertex.point)-face.offset
        values.append(value)
        if classify_strict_side:
            signs |= 1 if value < 0 else 2 if value > 0 else 4
    if classify_strict_side:
        if signs in (1, 2):
            # An affine plane function has one strict sign on the entire triangle.
            # Charge the fixed conclusion; there is no edge intersection to visit.
            budget.step()
            return [], False
        coplanar = signs == 4
    else:
        coplanar = all(value == 0 for value in values)
    if coplanar:
        return [(a.point, b.point) for a, b in zip(other.vertices, other.vertices[1:]+other.vertices[:1])], True
    points = set()
    for i, first in enumerate(other.vertices):
        budget.step()
        second = other.vertices[(i+1) % len(other.vertices)]
        a, b = values[i], values[(i+1) % len(values)]
        if a == 0:
            points.add(first.point)
        if a*b < 0:
            points.add(kernel.add(first.point, kernel.scale(kernel.subtract(second.point, first.point), a/(a-b))))
    if not points:
        return [], False
    # A noncoplanar triangle-plane intersection is exactly a segment or point.
    ordered = sorted(points)
    budget.step(len(ordered))
    return [(ordered[0], ordered[-1])], False


def _retain_identity_facet_witness(face, arrangement, budget):
    """An original-source reference is the exact complete unchanged triangle."""
    budget.polygons(_retained_region_slots(budget)+24)
    budget.step(3)
    budget.union_work["facetRegionWitness"].append({
        "source": list(face.source), "construction": "unchanged_original_triangle",
        "resolveGeometryAndFieldsFromPinnedOriginalSource": True,
        "sameOriginalSourceAndCompleteAffineStyle": True,
        "allPhysicalConstraintsRetained": True,
        "protectedStar": "only_original_triangle_vertices",
        "requiredBoundary": "all_three_original_triangle_edges"})
    budget.union_work["facetRegionWitnessSlots"] += 24


def _retain_facet_region_witness(face, arrangement, classified, output, region, budget):
    """Carry actual classified cells, constraints, cancellations and contact stars."""
    if not classified:
        return
    physical, protected = arrangement.region_constraints, arrangement.region_protected
    cancellations = region.cancellations if region else []
    removed = region.removed_stars if region else []
    input_members = sum(len(c) for c in classified)+sum(len(c) for c in output)
    upper = (input_members+2*len(physical)+len(protected))*8
    upper += 6*len(cancellations)+sum(4+4*len(star) for _,star,_ in removed)+64
    retained = budget.union_work.setdefault("facetRegionWitnessSlots", 0)
    # Encoding pools, lookup keys, compact lists, string coordinates and current geometry coexist.
    budget.polygons(upper+(region.current_slots() if region else
                                   arrangement.work["peakAdmittedMembershipSlots"]))
    budget.step(input_members+2*len(physical)+len(protected))
    points = set(p for c in classified for p in c)
    points.update(p for c in output for p in c)
    points.update(p for edge in physical for p in edge)
    points.update(protected)
    ordered = arrangement.ordered(points)
    indices = {point:i for i,point in enumerate(ordered)}
    budget.step(len(ordered))
    def encode(polygon):
        budget.step(len(polygon))
        return [indices[p] for p in polygon]
    record = {"source": list(face.source),
              "points": [[str(v) for v in point] for point in ordered],
              "classifiedCells": [encode(c) for c in classified],
              "physicalSegments": [encode(edge) for edge in arrangement.ordered(physical)],
              "protectedPoints": encode(arrangement.ordered(protected)),
              "outputPieces": [encode(c) for c in output],
              "cancelledAuxiliaryEdges": [encode((a,b))+[i,j] for a,b,i,j in cancellations],
              "removedVertexStars": [{"point":indices[p],"sectors":[encode(edge) for edge in star],
                                      "kind":kind} for p,star,kind in removed],
              "sameOriginalSourceAndCompleteAffineStyle": True,
              "allPhysicalConstraintsRetained": True,
              "construction": "constrained_original_triangle" if region else "unchanged_single_cell",
              "twiceArea": str(sum(arrangement.area(c) for c in classified))}
    budget.step(2*len(ordered)+len(cancellations)+len(removed))
    actual_slots = (3*len(ordered)+input_members+3*len(physical)+len(protected)
                    +5*len(cancellations)+sum(3+3*len(star) for _,star,_ in removed)+32)
    budget.union_work["facetRegionWitnessSlots"] = retained+actual_slots
    budget.union_work["facetRegionWitness"].append(record)


def reconstruct_face(face, index, budget, kernel):
    axis = max(range(3), key=lambda k: abs(face.normal[k]))
    axes = [k for k in range(3) if k != axis]
    project = lambda p: (p[axes[0]], p[axes[1]])
    domain = [project(v.point) for v in face.vertices]
    if sum(_cross2(a, b) for a, b in zip(domain, domain[1:]+domain[:1])) < 0:
        domain.reverse()
    budget.union_work.pop("activeFacetRegion",None)
    arrangement = PlanarArrangement(domain, budget, kernel.UnionFailure)
    arrangement.region_axes=tuple(axes)
    arrangement.work["sourceFacet"] = list(face.source)
    def lift(point):
        result = [Fraction(0)]*3
        for k, value in zip(axes, point):
            result[k] = value
        result[axis] = (face.offset-sum(face.normal[k]*result[k] for k in axes))/face.normal[axis]
        budget.step()
        if any(max(v.numerator.bit_length(), v.denominator.bit_length()) > 4096 for v in result):
            raise kernel.UnionFailure("Source arrangement exceeds original rational bound")
        return tuple(result)
    barriers, coplanar = [], []
    plane = index.section_plane(face)
    candidates = index.near(face)
    # Axis-aligned AABB overlap already excludes strict same-side triangles.
    # On an oblique plane with several leaves' worth of candidates, classify
    # signs during the miss computation to avoid walking wholly separated faces.
    dense_oblique = (len(candidates) >= _SECTION_DENSE_CANDIDATES
                     and sum(value != 0 for value in face.normal) > 1)
    if dense_oblique:
        budget.step()  # Fixed dense collection selection.
    for ident in candidates:
        budget.step()
        other = index.faces[ident]
        if other.source == face.source:
            continue
        sections, same_plane = index.section_for(face, ident, plane, classify_strict_side=dense_oblique)
        barriers.extend((project(a), project(b)) for a, b in sections)
        budget.polygons(len(barriers))
        if same_plane and kernel.dot(face.normal, other.normal) > 0:
            coplanar.append(other)
    possible_owners = index.possible_owners(face)
    def visible(sample):
        point = lift(sample)
        if _outside_occupied(point, face.normal, index, budget, kernel, possible_owners):
            return False
        winner = face.source
        for other in coplanar:
            budget.step()
            if not _inside_triangle(other, point, budget, kernel)[0]:
                continue
            if face.style != other.style or any(kernel.source_uv(face, v.point) != kernel.source_uv(other, v.point)
                                                 for v in face.vertices):
                raise kernel.UnionFailure("Exposed original arrangement has conflicting complete affine fields")
            winner = min(winner, other.source)
        return winner == face.source
    cells = arrangement.cells(barriers, visible)
    classified = cells
    # Exact identity, not a geometric approximation or omitted contact witness.
    # Every physical endpoint/intersection must be an original triangle corner.
    if (len(cells) == 1 and len(cells[0]) == 3 and set(cells[0]) == set(arrangement.domain)
            and arrangement.region_protected <= set(arrangement.domain)):
        _retain_identity_facet_witness(face, arrangement, budget)
        arrangement.work["originalFacetIdentityReused"] = True
        return [face], arrangement.work
    region = None
    if len(cells) > 1:
        region = FacetRegions(arrangement, cells)
        cells, region_work = region.reconstruct()
        arrangement.work["facetRegionWork"] = region_work
    _retain_facet_region_witness(face, arrangement, classified, cells, region, budget)
    result = []
    for cell in cells:
        budget.step()
        vertices = []
        for point in cell:
            budget.step()
            p = lift(point)
            uv = kernel.source_uv(face, p)
            if any(max(v.numerator.bit_length(), v.denominator.bit_length()) > 4096 for v in uv):
                raise kernel.UnionFailure("Source arrangement UV exceeds original rational bound")
            vertices.append(kernel.Vertex(p, uv))
        value = kernel.polygon(vertices, face.source, face.style)
        if value is None:
            raise kernel.UnionFailure("Source arrangement lost an exposed cell")
        # Protected collinear prefixes need the complete polygon's nonzero normal.
        if kernel.dot(value.normal, face.normal) < 0:
            budget.step(len(vertices))
            value = value.flipped()
        result.append(value)
    budget.union_work.pop("activeFacetRegion",None)
    return result, arrangement.work


# Outer work fields are identical across the three normal completed-facet
# variants. The nested region work and all independent witnesses stay intact.
_FACET_WORK_COLUMNS = (
    "artificialCollinearPointsRemoved", "classifiedCells", "endpointTouchesRetained",
    "lastRequiredGeometrySlots", "peakAdmittedMembershipSlots", "phase",
    "physicalSegmentMemberships", "pointContactChords", "protectedPoints",
    "rawConstraintCacheHits", "rawConstraintClipCalls", "retainedAtomicCells",
    "retainedCells", "segmentPairs", "slabs", "sourceFacet", "stripsContactBlocked",
    "stripsConvexityRejected", "stripsExtended", "verticalContactChords",
)
_FACET_WORK_TAILS = (None, "originalFacetIdentityReused", "facetRegionWork")
_FACET_WORK_SCHEMA = {"version": 1, "columns": _FACET_WORK_COLUMNS,
                      "tailsByTag": _FACET_WORK_TAILS, "sourceAlias": "sourceFacet"}
# Schema fields, columns, tails and owning membership; admit once.
_FACET_WORK_SCHEMA_SLOTS = 9+len(_FACET_WORK_COLUMNS)+len(_FACET_WORK_TAILS)
# A streaming reader can still hold the previous row while decoding the next.
# Equality may stream two views over one ledger. Reserve two rows per view,
# including map memberships, source aliases and iterator scratch.
_FACET_WORK_READ_SLOTS = 4*(2*(len(_FACET_WORK_COLUMNS)+2)+6)
_FACET_WORK_INITIAL_SLOTS = _FACET_WORK_SCHEMA_SLOTS + _FACET_WORK_READ_SLOTS


def decoded_facet_work(schema, rows):
    """Lazily restore exact historical field names, including the source alias."""
    if (schema["version"] != 1 or tuple(schema["columns"]) != _FACET_WORK_COLUMNS
            or tuple(schema["tailsByTag"]) != _FACET_WORK_TAILS
            or schema["sourceAlias"] != "sourceFacet"):
        raise ValueError("Unknown per-facet work encoding")
    width = len(_FACET_WORK_COLUMNS)
    for row in rows:
        tag = row[0]
        if type(tag) is not int or not 0 <= tag < len(_FACET_WORK_TAILS):
            raise ValueError("Unknown per-facet work row tag")
        if len(row) != 1+width+bool(tag):
            raise ValueError("Invalid per-facet work row width")
        restored = dict(zip(_FACET_WORK_COLUMNS, islice(row,1,width+1), strict=True))
        restored["source"] = list(restored["sourceFacet"])
        if tag:
            restored[_FACET_WORK_TAILS[tag]] = row[-1]
        yield restored


class _FacetWorkView(Sequence):
    """Versioned diagnostic sequence. Indexing and slices decode lazily."""
    def __init__(self, rows, positions=None):
        self._rows = rows
        self._positions = range(len(rows)) if positions is None else positions

    def __len__(self):
        return len(self._positions)

    def __iter__(self):
        return decoded_facet_work(_FACET_WORK_SCHEMA,
                                  (self._rows[i] for i in self._positions))

    def __getitem__(self, index):
        position = self._positions[index]
        if isinstance(index, slice):
            return _FacetWorkView(self._rows, position)
        return next(decoded_facet_work(_FACET_WORK_SCHEMA, (self._rows[position],)))

    def __eq__(self, other):
        if not isinstance(other, (list, _FacetWorkView)):
            return NotImplemented
        return len(self) == len(other) and all(a == b for a,b in zip(self,other))


def facet_work_json_default(value):
    """Adapt the diagnostic sequence to the standard JSON array encoder."""
    if not isinstance(value, _FacetWorkView):
        raise TypeError(f"Object of type {type(value).__name__} is not JSON serializable")

    class StreamingArray(list):
        # This adapter is private to JSON. Public readers receive a Sequence.
        def __iter__(self):
            return iter(value)

        def __len__(self):
            return len(value)

    return StreamingArray()


def _retain_facet_work(face, counters, cells, result, budget, failure):
    """Admit one positional row before publishing either work or output."""
    tag = 2 if "facetRegionWork" in counters else int("originalFacetIdentityReused" in counters)
    width = len(_FACET_WORK_COLUMNS)+bool(tag)
    if len(counters) != width or counters.get("sourceFacet") != list(face.source):
        raise failure("Completed facet work does not match its lossless schema")
    region = counters.get("facetRegionWork", {})
    # Row tag, owning membership, all value positions, and unchanged nested
    # key/value memberships. Current counter-map keys coexist during packing.
    retained = 2+width+2*len(region)
    work = budget.union_work
    slots = work["sourceArrangementWorkSlots"]+retained
    # The input list still owns its references when result has grown.
    budget.polygons(work["facetRegionWitnessSlots"]+slots+len(result)+2*len(cells)+2*len(counters))
    budget.step(len(counters)+1)
    # Stream over the fixed schema; no intermediate materialized row/list.
    def values():
        yield tag
        for name in _FACET_WORK_COLUMNS:
            yield counters[name]
        if tag:
            yield counters[_FACET_WORK_TAILS[tag]]
    try:
        row = tuple(values())
    except KeyError:
        raise failure("Completed facet work does not match its lossless schema") from None
    work["sourceArrangementPerFacetWork"].append(row)
    work["sourceArrangementWorkSlots"] = slots
    result.extend(cells)


def _reconstruct_original_regions(shells, budget, *, kernel):
    """Reconstruct authenticated original shells; no CSG fragment input accepted.

    Callers must use the existing input_shell authentication on original bytes.
    The existing final source/appearance and boundary_mesh guards remain required.
    This experimental API is not yet a fleet recipe or acceptance path.
    """
    original = [face for shell in shells for face in shell]
    if not original or any(not shell for shell in shells):
        raise kernel.UnionFailure("Every original component must remain present")
    if any(len(face.vertices) != 3 for face in original):
        raise kernel.UnionFailure("Source arrangement requires original triangles")
    budget.polygons(len(original))
    budget.step(len(original))
    owners = []
    for shell in shells:
        budget.step(len(shell))
        if len({face.source[0] for face in shell}) != 1:
            raise kernel.UnionFailure("Source arrangement component has inconsistent ownership")
        owners.append(shell[0].source[0])
    if len(set(owners)) != len(owners):
        raise kernel.UnionFailure("Source arrangement components require distinct ownership")
    if len({face.source for face in original}) != len(original):
        raise kernel.UnionFailure("Source arrangement requires unique original facet identities")
    budget.context.bind_originals(shells)
    budget.union_work.update(strategy="original-source-certified-facet-regions", stage="source_index",
                             originalComponents=len(shells), sourceFacets=len(original))
    index = OriginalFaceIndex(original, budget, kernel)
    budget.polygons(_FACET_WORK_INITIAL_SLOTS)
    budget.union_work.update(facetRegionWitness=[], facetRegionWitnessSlots=0,
                             sourceArrangementWorkSlots=_FACET_WORK_INITIAL_SLOTS,
                             facetRegionWitnessComplete=False,
                             sourceArrangementPerFacetWorkEncoding=_FACET_WORK_SCHEMA,
                             sourceArrangementPerFacetWork=[])
    result = []
    for face in original:
        budget.step()
        budget.union_work.update(stage="source_arrangement", currentSource=list(face.source),
                                 operationsAtCurrentSourceEntry=budget.operations)
        cells,counters=reconstruct_face(face,index,budget,kernel)
        _retain_facet_work(face,counters,cells,result,budget,kernel.UnionFailure)
        del cells,counters
        budget.polygons(len(result))
    if not result:
        raise kernel.UnionFailure("Source arrangement lost every original exterior")
    budget.union_work.update(stage="source_fragment_audit", operationsAtSourceAuditEntry=budget.operations)
    kernel.audit_source_fragments(result, original, budget)
    budget.union_work.update(stage="appearance_audit", operationsAtAppearanceAuditEntry=budget.operations)
    kernel.audit_boundary_appearance(result, original, budget)
    budget.union_work.update(stage="complete", facetRegionWitnessComplete=True,
                             operationsAtUnionReturn=budget.operations,
                             boundaryPolygonsAtUnionReturn=len(result))
    return result, {"facetRegionWitness": budget.union_work["facetRegionWitness"],
                    "facetRegionReconstruction": True, "operations": budget.operations, "indexNodes": index.nodes,
                    "sourceFacets": len(original), "boundaryCells": len(result),
                    "componentPruningWork": dict(index.work),
                    "sectionReuseWork": dict(index.section_work),
                    "perSourceEncoding": "read-only-sequence-v1",
                    "perSource": _FacetWorkView(budget.union_work["sourceArrangementPerFacetWork"])}


def surface_union(shells, budget, *, kernel):
    """Production API retains already admitted state for every strict refusal."""
    try:
        return _reconstruct_original_regions(shells,budget,kernel=kernel)
    except kernel.UnionFailure as error:
        if not any('"unionWork"' in note for note in getattr(error,'__notes__',())):
            error.add_note(json.dumps({'operations':budget.operations,
                                       'unionWork':budget.union_work},sort_keys=True))
        raise
