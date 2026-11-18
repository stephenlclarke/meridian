//! Owned host harness. Filesystem and monotonic timing stay outside the geometry library.
use meridian_geometry::{
    Budget, Facet, Failure, FailureKind, Hull, Limits, OriginalAsset, Point3, Rational, integer,
    reconstruct,
};
use meridian_geometry::{BudgetPhase, BudgetRejection};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

const BODIES: [&str; 5] = [
    "bound_original_decode",
    "original_construction",
    "source_and_affine_fields",
    "complete_facet_certificates",
    "closed_topology_and_limits",
];
const ASSETS: [OriginalAsset; 5] = [
    OriginalAsset::Sh010,
    OriginalAsset::Sh011,
    OriginalAsset::Sh016,
    OriginalAsset::Sh017,
    OriginalAsset::Sh166,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Pass,
    Refused,
    Fail,
    Unrun,
}
impl Status {
    fn text(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Refused => "REFUSED",
            Self::Fail => "FAIL",
            Self::Unrun => "UNRUN",
        }
    }
}
#[derive(Debug)]
struct CaseReport {
    asset: OriginalAsset,
    bodies: [Status; 5],
    body_metrics: [Option<(u64, usize)>; 5],
    failure: Option<Failure>,
    budget_rejection: Option<(BudgetRejection, u64, usize)>,
    operations: u64,
    peak: usize,
    admitted_peak: usize,
    retained_at_terminal: usize,
    elapsed_nanos: u128,
    candidate: bool,
}
impl CaseReport {
    fn pending(asset: OriginalAsset) -> Self {
        Self {
            asset,
            bodies: [Status::Unrun; 5],
            body_metrics: [None; 5],
            failure: None,
            budget_rejection: None,
            operations: 0,
            peak: 0,
            admitted_peak: 0,
            retained_at_terminal: 0,
            elapsed_nanos: 0,
            candidate: false,
        }
    }
    fn record_failure(&mut self, stage: usize, error: Failure) {
        self.bodies[stage] = check_status(&error);
        self.failure = Some(error);
    }
    fn json(&self) -> String {
        let bodies = BODIES
            .iter()
            .zip(self.bodies)
            .enumerate()
            .map(|(index, (name, status))| {
                format!(
                    "{{\"name\":{},\"sourceAsset\":{},\"sourceInputSHA256\":{},\"status\":{},\"chargedOperationDelta\":{},\"peakStorageAtBodyTerminal\":{},\"criteria\":\"WHOLE-FIVE-CRITERIA.json\"}}",
                    quote(name),
                    quote(self.asset.id()),
                    quote(input_sha(self.asset)),
                    quote(status.text()),
                    self.body_metrics[index].map_or_else(||"null".into(), |(ops,_)|ops.to_string()),
                    self.body_metrics[index].map_or_else(||"null".into(), |(_,peak)|peak.to_string())
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let failure = self
            .failure
            .as_ref()
            .map_or_else(|| "null".into(), failure_json);
        format!(
            "{{\"id\":{},\"bodies\":[{}],\"failure\":{},\"budgetRejection\":{},\"operations\":{},\"peakStorageMemberships\":{},\"peakAdmittedReservationMemberships\":{},\"retainedBudgetMembershipsAtTerminal\":{},\"storageAccounting\":\"logical reservations; attempted peaks include refused admissions, not measured allocated bytes\",\"monotonicNanoseconds\":{},\"completeCandidateReturned\":{},\"allCandidateChecksPassed\":{},\"limits\":{{\"operations\":2000000,\"storageMemberships\":100000,\"rationalBits\":4096,\"triangles\":512}},\"sourceBinding\":\"EXECUTION-BINDING.json\",\"inputBindings\":\"ORIGINAL-BINDINGS.json\",\"fleetAdmission\":false}}",
            quote(self.asset.id()),
            bodies,
            failure,
            self.budget_rejection.map_or_else(
                || "null".into(),
                |(context, base, storage)| budget_rejection_json(context, base, storage)
            ),
            self.operations,
            self.peak,
            self.admitted_peak,
            self.retained_at_terminal,
            self.elapsed_nanos,
            self.candidate,
            self.bodies.iter().all(|s| *s == Status::Pass)
        )
    }
}
fn quote(text: &str) -> String {
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(result, "\\u{:04x}", u32::from(c));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
fn failure_json(f: &Failure) -> String {
    let source = f.source.map_or_else(
        || "null".into(),
        |s| format!("[{},{}]", s.component, s.facet),
    );
    format!(
        "{{\"kind\":{},\"source\":{},\"operationsAtRefusal\":{},\"detail\":{}}}",
        quote(&format!("{:?}", f.kind)),
        source,
        f.operations,
        quote(f.detail)
    )
}
fn budget_rejection_json(r: BudgetRejection, operation_base: u64, storage_base: usize) -> String {
    let ledger = BudgetPhase::ALL
        .iter()
        .zip(r.phase_operations)
        .map(|(phase, count)| format!("{}:{count}", quote(phase.name())))
        .collect::<Vec<_>>()
        .join(",");
    let counters = r
        .state
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let source = r.source.map_or_else(
        || "null".into(),
        |s| format!("[{},{}]", s.component, s.facet),
    );
    format!(
        "{{\"kind\":{},\"source\":{},\"phase\":{},\"caller\":{{\"file\":{},\"line\":{},\"column\":{}}},\"operationBase\":{},\"storageBase\":{},\"operationsBefore\":{},\"operationsAfter\":{},\"attemptedOperations\":{},\"retainedBeforeCleanup\":{},\"attemptedScratch\":{},\"requiredMemberships\":{},\"phaseOperations\":{{{}}},\"stageCounters\":[{}],\"stageCountersAtOperations\":{},\"activeLimits\":{{\"operations\":{},\"storageMemberships\":{},\"rationalBits\":{},\"triangles\":{}}}}}",
        quote(&format!("{:?}", r.kind)),
        source,
        quote(r.phase.name()),
        quote(r.file),
        r.line,
        r.column,
        operation_base,
        storage_base,
        r.operations_before,
        r.operations_after,
        r.attempted_operations,
        r.retained_before,
        r.attempted_scratch,
        r.required_memberships
            .map_or_else(|| "null".into(), |v| v.to_string()),
        ledger,
        counters,
        r.state_at_operations,
        r.limits.operations,
        r.limits.storage_memberships,
        r.limits.rational_bits,
        r.limits.triangles
    )
}

fn input_sha(asset: OriginalAsset) -> &'static str {
    match asset {
        OriginalAsset::Sh010 => "6d12e9ed28b49feb2dbec71bd7deec437ba4cb5f8a2d4e0acbf9a2f76502d6dc",
        OriginalAsset::Sh011 => "9df2a67302da6d07c46ae56dd2cfcdf6d03eebf6773bc260c9f12699e8ea393c",
        OriginalAsset::Sh016 => "72b8aa1bc9b194f0007e884047a4a8bc27a2ec7aaa14ea7eee1abf4cf00f173e",
        OriginalAsset::Sh017 => "6ca509057ad2412df95d66ad55d4b8ea7fb09bbc8b1abb3811a63239eb14cb3a",
        OriginalAsset::Sh166 => "0e12a2c1cbb548ee6fdd0bb351a7f2bce460214ae1cce122d8d073b537b51a8f",
    }
}
fn check_status(error: &Failure) -> Status {
    match error.kind {
        FailureKind::Operations
        | FailureKind::Storage
        | FailureKind::Rational
        | FailureKind::TriangleLimit => Status::Refused,
        _ => Status::Fail,
    }
}
fn tree_work(size: usize, b: &mut Budget) -> Result<(), Failure> {
    b.step((size.saturating_add(1).ilog2() + 1).into())
}
fn failed(b: &Budget, detail: &'static str) -> Failure {
    Failure {
        kind: FailureKind::InvalidWitness,
        source: None,
        operations: b.operations(),
        detail,
    }
}
fn sub(a: &Point3, b: &Point3) -> Point3 {
    std::array::from_fn(|i| &a[i] - &b[i])
}
fn cross(a: &Point3, b: &Point3) -> Point3 {
    [
        &a[1] * &b[2] - &a[2] * &b[1],
        &a[2] * &b[0] - &a[0] * &b[2],
        &a[0] * &b[1] - &a[1] * &b[0],
    ]
}
fn dot(a: &Point3, b: &Point3) -> Rational {
    (0..3).map(|i| &a[i] * &b[i]).sum()
}
fn fields(hull: &Hull, originals: &[Facet], b: &mut Budget) -> Result<(), Failure> {
    for triangle in hull.triangles() {
        b.step(originals.len() as u64)?;
        let original = originals
            .iter()
            .find(|f| f.id() == triangle.id())
            .ok_or_else(|| failed(b, "candidate lost original facet identity"))?;
        if original.style() != triangle.style() {
            return Err(failed(
                b,
                "candidate changed complete material/texture field",
            ));
        }
        for vertex in triangle.vertices() {
            if original.uv_at(&vertex.point, b)? != vertex.uv {
                return Err(failed(b, "candidate changed original affine UV"));
            }
        }
        let a = original.vertices();
        let c = triangle.vertices();
        b.step(1)?;
        let normal = cross(
            &sub(&a[1].point, &a[0].point),
            &sub(&a[2].point, &a[0].point),
        );
        let actual = cross(
            &sub(&c[1].point, &c[0].point),
            &sub(&c[2].point, &c[0].point),
        );
        if dot(&normal, &actual) <= integer(0) {
            return Err(failed(b, "candidate reversed original winding"));
        }
    }
    Ok(())
}
fn certificates(hull: &Hull, originals: &[Facet], b: &mut Budget) -> Result<(), Failure> {
    for witness in hull.witnesses() {
        b.step(originals.len() as u64)?;
        let original = originals
            .iter()
            .find(|f| f.id() == witness.source)
            .ok_or_else(|| failed(b, "witness lost original identity"))?;
        witness.verify(original, b)?;
        let normal = cross(
            &sub(&original.vertices()[1].point, &original.vertices()[0].point),
            &sub(&original.vertices()[2].point, &original.vertices()[0].point),
        );
        let mut axis = 0;
        for i in 1..3 {
            if &normal[i] * &normal[i] > &normal[axis] * &normal[axis] {
                axis = i;
            }
        }
        let mut expected = BTreeMap::new();
        let mut actual = BTreeMap::new();
        b.guard(32 * (witness.output_triangles.len() + hull.triangles().len() + 1))?;
        for triangle in &witness.output_triangles {
            b.step(3)?;
            let mut key = triangle.clone();
            key.sort();
            tree_work(expected.len(), b)?;
            *expected.entry(key).or_insert(0usize) += 1;
        }
        b.step(hull.triangles().len() as u64)?;
        for triangle in hull.triangles().iter().filter(|f| f.id() == witness.source) {
            b.step(3)?;
            let mut projected = triangle.vertices().each_ref().map(|v| {
                let axes: Vec<_> = (0..3).filter(|i| *i != axis).collect();
                [v.point[axes[0]].clone(), v.point[axes[1]].clone()]
            });
            projected.sort();
            tree_work(actual.len(), b)?;
            *actual.entry(projected).or_insert(0usize) += 1;
        }
        b.step((expected.len() + actual.len()) as u64)?;
        if expected != actual {
            return Err(failed(
                b,
                "candidate triangles differ from complete facet witness",
            ));
        }
    }
    Ok(())
}
fn topology(hull: &Hull, b: &mut Budget) -> Result<(), Failure> {
    if hull.triangles().is_empty() || hull.triangles().len() > 512 {
        return Err(failed(b, "complete candidate violates original face bound"));
    }
    b.guard(128 * hull.triangles().len() + 64)?;
    let mut edges: BTreeMap<[Point3; 2], Vec<bool>> = BTreeMap::new();
    let mut links: BTreeMap<Point3, Vec<[Point3; 2]>> = BTreeMap::new();
    let mut volume = integer(0);
    for face in hull.triangles() {
        b.step(1)?;
        let v = face.vertices();
        volume += dot(&v[0].point, &cross(&v[1].point, &v[2].point)) / integer(6);
        for i in 0..3 {
            b.step(3)?;
            let a = &v[i].point;
            let c = &v[(i + 1) % 3].point;
            let d = &v[(i + 2) % 3].point;
            let ordered = if a < c {
                [a.clone(), c.clone()]
            } else {
                [c.clone(), a.clone()]
            };
            tree_work(edges.len(), b)?;
            edges.entry(ordered).or_default().push(a < c);
            tree_work(links.len(), b)?;
            links
                .entry(a.clone())
                .or_default()
                .push([d.clone(), c.clone()]);
        }
    }
    for values in edges.values() {
        b.step(1)?;
        if values.len() != 2 || values[0] == values[1] {
            return Err(failed(
                b,
                "complete candidate lacks reciprocal edge incidence",
            ));
        }
    }
    for sectors in links.values() {
        b.guard(128 * hull.triangles().len() + 24 * sectors.len() + 64)?;
        let mut graph: BTreeMap<Point3, Vec<Point3>> = BTreeMap::new();
        for [a, c] in sectors {
            b.step(2)?;
            tree_work(graph.len(), b)?;
            graph.entry(a.clone()).or_default().push(c.clone());
            tree_work(graph.len(), b)?;
            graph.entry(c.clone()).or_default().push(a.clone());
        }
        b.step(graph.len() as u64)?;
        if graph.values().any(|v| v.len() != 2) {
            return Err(failed(b, "complete candidate link is not a cycle"));
        }
        let first = graph
            .keys()
            .next()
            .ok_or_else(|| failed(b, "empty complete candidate link"))?;
        let mut pending = vec![first.clone()];
        let mut seen = BTreeSet::new();
        while let Some(p) = pending.pop() {
            b.step(1)?;
            tree_work(seen.len(), b)?;
            if seen.insert(p.clone()) {
                tree_work(graph.len(), b)?;
                for q in &graph[&p] {
                    b.step(1)?;
                    tree_work(seen.len(), b)?;
                    if !seen.contains(q) {
                        pending.push(q.clone());
                    }
                }
            }
        }
        if seen.len() != graph.len() {
            return Err(failed(b, "complete candidate link has disconnected sheets"));
        }
    }
    if volume <= integer(0) || &volume != hull.exact_volume() {
        return Err(failed(
            b,
            "candidate volume differs from audited exact volume",
        ));
    }
    Ok(())
}
fn remaining_limits(operations: u64, held: usize) -> Result<Limits, Failure> {
    let original = Limits::default();
    let operations = original.operations.checked_sub(operations).ok_or(Failure {
        kind: FailureKind::Operations,
        source: None,
        operations: 0,
        detail: "no original work budget remains for dependent checks",
    })?;
    let storage_memberships = original
        .storage_memberships
        .checked_sub(held)
        .ok_or(Failure {
            kind: FailureKind::Storage,
            source: None,
            operations: 0,
            detail: "live candidate and source exceed original storage bound",
        })?;
    Ok(Limits {
        operations,
        storage_memberships,
        ..original
    })
}
fn candidate_wire(hull: &Hull) -> String {
    let mut text = format!("meridian-exact-facets-v1 {}\n", hull.triangles().len());
    for triangle in hull.triangles() {
        let mut style = String::new();
        for byte in triangle.style().bytes() {
            let _ = write!(style, "{byte:02x}");
        }
        let _ = write!(
            text,
            "{} {} {}",
            triangle.id().component,
            triangle.id().facet,
            style
        );
        for vertex in triangle.vertices() {
            for value in vertex.point.iter().chain(&vertex.uv) {
                let _ = write!(text, " {}/{}", value.numer(), value.denom());
            }
        }
        text.push('\n');
    }
    text
}
fn p2(p: &[Rational; 2]) -> String {
    format!(
        "[{},{}]",
        quote(&p[0].to_string()),
        quote(&p[1].to_string())
    )
}
fn cells_json(cells: &[Vec<[Rational; 2]>]) -> String {
    format!(
        "[{}]",
        cells
            .iter()
            .map(|cell| format!("[{}]", cell.iter().map(p2).collect::<Vec<_>>().join(",")))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn witness_wire(hull: &Hull) -> String {
    let rows=hull.witnesses().iter().map(|w| {
        let physical=w.physical_segments.iter().map(|s|format!("[{},{}]",p2(&s[0]),p2(&s[1]))).collect::<Vec<_>>().join(",");
        let triangles=w.output_triangles.iter().map(|t|format!("[{},{},{}]",p2(&t[0]),p2(&t[1]),p2(&t[2]))).collect::<Vec<_>>().join(",");
        let stars=w.contact_stars.iter().map(|star|format!("{{\"point\":{},\"boundaryPath\":{},\"sectors\":[{}]}}",p2(&star.point),star.boundary_path,
            star.sectors.iter().map(|s|format!("[{},{}]",p2(&s[0]),p2(&s[1]))).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(",");
        format!("{{\"source\":[{},{}],\"twiceArea\":{},\"classifiedCells\":{},\"physicalSegments\":[{}],\"protectedPoints\":[{}],\"outputTriangles\":[{}],\"contactStars\":[{}]}}",
            w.source.component,w.source.facet,quote(&w.twice_area.to_string()),cells_json(&w.classified_cells),physical,w.protected_points.iter().map(p2).collect::<Vec<_>>().join(","),triangles,stars)
    }).collect::<Vec<_>>().join(",");
    format!(
        "{{\"exactVolume\":{},\"witnesses\":[{}]}}\n",
        quote(&hull.exact_volume().to_string()),
        rows
    )
}
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
type CandidateCheck = fn(&Hull, &[Facet], &mut Budget) -> Result<(), Failure>;

fn run_case(asset: OriginalAsset, out: &Path) -> io::Result<CaseReport> {
    let start = Instant::now();
    let mut report = CaseReport::pending(asset);
    let mut budget = Budget::new(Limits::default()).map_err(io::Error::other)?;
    let originals = match asset.decode(asset.pinned_bytes(), &mut budget) {
        Ok(value) => {
            report.bodies[0] = Status::Pass;
            report.body_metrics[0] = Some((budget.operations(), budget.peak_memberships()));
            value
        }
        Err(error) => {
            report.record_failure(0, error);
            report.budget_rejection = budget.rejection().map(|r| (r, 0, 0));
            report.body_metrics[0] = Some((budget.operations(), budget.peak_memberships()));
            report.operations = budget.operations();
            report.peak = budget.peak_memberships();
            report.admitted_peak = budget.peak_admitted_memberships();
            report.retained_at_terminal = budget.retained_memberships();
            report.elapsed_nanos = start.elapsed().as_nanos();
            return Ok(report);
        }
    };
    let construction_start = budget.operations();
    match reconstruct(&originals, &mut budget) {
        Err(error) => {
            report.record_failure(1, error);
            report.budget_rejection = budget.rejection().map(|r| (r, 0, 0));
            report.body_metrics[1] = Some((
                budget.operations() - construction_start,
                budget.peak_memberships(),
            ));
        }
        Ok(hull) => {
            report.bodies[1] = Status::Pass;
            report.body_metrics[1] = Some((
                budget.operations() - construction_start,
                budget.peak_memberships(),
            ));
            report.candidate = true;
            let held = budget.retained_memberships()
                + 40 * originals.len()
                + 64
                + originals.iter().map(|f| f.style().len()).sum::<usize>();
            let spent = budget.operations();
            let checks = remaining_limits(spent, held).and_then(Budget::new);
            match checks {
                Err(mut error) => {
                    error.operations = spent.saturating_add(error.operations);
                    report.bodies[2] = check_status(&error);
                    report.body_metrics[2] = Some((0, held));
                    report.peak = held;
                    report.failure = Some(error);
                }
                Ok(mut checks) => {
                    let stages: [CandidateCheck; 3] =
                        [fields, certificates, |h, _, b| topology(h, b)];
                    for (i, check) in stages.into_iter().enumerate() {
                        let entry = checks.operations();
                        let result = checks.with_phase(BudgetPhase::DependentChecks, |b| {
                            check(&hull, &originals, b)
                        });
                        report.body_metrics[i + 2] = Some((
                            checks.operations() - entry,
                            held.saturating_add(checks.peak_memberships()),
                        ));
                        if let Err(mut error) = result {
                            error.operations = spent.saturating_add(error.operations);
                            report.bodies[i + 2] = check_status(&error);
                            report.failure = Some(error);
                            report.budget_rejection = checks.rejection().map(|r| (r, spent, held));
                            break;
                        }
                        report.bodies[i + 2] = Status::Pass;
                    }
                    report.operations = spent.saturating_add(checks.operations());
                    report.peak = budget
                        .peak_memberships()
                        .max(held.saturating_add(checks.peak_memberships()));
                    report.admitted_peak = budget
                        .peak_admitted_memberships()
                        .max(held.saturating_add(checks.peak_admitted_memberships()));
                }
            }
            write_new(
                &out.join(format!("{}-UNQUALIFIED-CANDIDATE.facets", asset.id())),
                candidate_wire(&hull).as_bytes(),
            )?;
            write_new(
                &out.join(format!("{}-WITNESSES.json", asset.id())),
                witness_wire(&hull).as_bytes(),
            )?;
        }
    }
    report.operations = report.operations.max(budget.operations());
    report.peak = report.peak.max(budget.peak_memberships());
    report.admitted_peak = report.admitted_peak.max(budget.peak_admitted_memberships());
    report.retained_at_terminal = budget.retained_memberships();
    report.elapsed_nanos = start.elapsed().as_nanos();
    Ok(report)
}
fn parse_args() -> io::Result<(PathBuf, PathBuf)> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 || args[0] != "--out" || args[2] != "--binding" {
        return Err(io::Error::other(
            "expected --out NEW_DIRECTORY --binding VERIFIED_EXECUTION_BINDING",
        ));
    }
    Ok((PathBuf::from(&args[1]), PathBuf::from(&args[3])))
}
fn run() -> io::Result<bool> {
    let (out, binding) = parse_args()?;
    let binding = fs::read(binding)?;
    fs::create_dir(&out)?;
    write_new(&out.join("EXECUTION-BINDING.json"), &binding)?;
    write_new(
        &out.join("WHOLE-FIVE-CRITERIA.json"),
        include_bytes!("../../integration/WHOLE-FIVE-CRITERIA.json"),
    )?;
    write_new(
        &out.join("ORIGINAL-BINDINGS.json"),
        include_bytes!("../../fixtures/ORIGINAL-BINDINGS.json"),
    )?;
    let started = Instant::now();
    let mut reports = Vec::new();
    for asset in ASSETS {
        println!("ORIGINAL_CASE_START {}", asset.id());
        let report = run_case(asset, &out)?;
        write_new(
            &out.join(format!("{}-CASE.json", asset.id())),
            report.json().as_bytes(),
        )?;
        println!("ORIGINAL_CASE_TERMINAL {}", report.json());
        reports.push(report);
    }
    let passed = reports
        .iter()
        .all(|r| r.bodies.iter().all(|s| *s == Status::Pass));
    let terminal = format!(
        "{{\"status\":{},\"declaredBodies\":25,\"bodiesPass\":{},\"bodiesRefused\":{},\"bodiesFail\":{},\"bodiesUnrun\":{},\"monotonicNanoseconds\":{},\"cases\":[{}],\"fleetBaseline\":{{\"ships\":23,\"shipTotal\":162,\"stations\":4,\"stationTotal\":6}},\"all171Accepted\":false,\"nativeAcceptance\":false}}\n",
        quote(if passed {
            "COMPLETE_RUST_CANDIDATES_CHECKED"
        } else {
            "ORIGINAL_GEOMETRY_BLOCKERS_RETAINED"
        }),
        reports
            .iter()
            .flat_map(|r| r.bodies)
            .filter(|s| *s == Status::Pass)
            .count(),
        reports
            .iter()
            .flat_map(|r| r.bodies)
            .filter(|s| *s == Status::Refused)
            .count(),
        reports
            .iter()
            .flat_map(|r| r.bodies)
            .filter(|s| *s == Status::Fail)
            .count(),
        reports
            .iter()
            .flat_map(|r| r.bodies)
            .filter(|s| *s == Status::Unrun)
            .count(),
        started.elapsed().as_nanos(),
        reports
            .iter()
            .map(CaseReport::json)
            .collect::<Vec<_>>()
            .join(",")
    );
    write_new(&out.join("TERMINAL.json"), terminal.as_bytes())?;
    Ok(passed)
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(error) => {
            eprintln!("whole-five host failure: {error}");
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn refusal(kind: FailureKind) -> Failure {
        Failure {
            kind,
            source: None,
            operations: 2_000_001,
            detail: "bounded harness regression",
        }
    }
    #[test]
    fn construction_refusal_keeps_all_candidate_checks_unrun() {
        let mut report = CaseReport::pending(OriginalAsset::Sh010);
        report.bodies[0] = Status::Pass;
        report.record_failure(1, refusal(FailureKind::Operations));
        assert_eq!(
            report.bodies,
            [
                Status::Pass,
                Status::Refused,
                Status::Unrun,
                Status::Unrun,
                Status::Unrun
            ]
        );
        assert!(!report.candidate);
        assert!(report.json().contains("\"chargedOperationDelta\":null"));
    }
    #[test]
    fn semantic_decode_failure_keeps_construction_unrun() {
        let mut report = CaseReport::pending(OriginalAsset::Sh166);
        report.record_failure(0, refusal(FailureKind::InvalidInput));
        assert_eq!(
            report.bodies,
            [
                Status::Fail,
                Status::Unrun,
                Status::Unrun,
                Status::Unrun,
                Status::Unrun
            ]
        );
        assert!(!report.candidate);
    }
    #[test]
    fn typed_failures_classify_decode_and_construction_consistently() {
        let kinds = [
            (FailureKind::Operations, Status::Refused),
            (FailureKind::Storage, Status::Refused),
            (FailureKind::Rational, Status::Refused),
            (FailureKind::TriangleLimit, Status::Refused),
            (FailureKind::InvalidInput, Status::Fail),
            (FailureKind::Degenerate, Status::Fail),
            (FailureKind::ConflictingFields, Status::Fail),
            (FailureKind::NonManifold, Status::Fail),
            (FailureKind::UncertifiedContact, Status::Fail),
            (FailureKind::InvalidWitness, Status::Fail),
        ];
        for stage in [0, 1] {
            for (kind, expected) in kinds {
                let mut report = CaseReport::pending(OriginalAsset::Sh010);
                if stage == 1 {
                    report.bodies[0] = Status::Pass;
                }
                report.record_failure(stage, refusal(kind));
                assert_eq!(report.bodies[stage], expected);
                assert_eq!(report.failure.as_ref().map(|f| f.kind), Some(kind));
                assert!(
                    report.bodies[stage + 1..]
                        .iter()
                        .all(|s| *s == Status::Unrun)
                );
                assert!(!report.candidate);
            }
        }
    }
    #[test]
    fn remaining_budget_cannot_reset_the_original_case_limits() -> Result<(), Failure> {
        let remaining = remaining_limits(1_900_000, 90_000)?;
        assert_eq!(
            remaining,
            Limits {
                operations: 100_000,
                storage_memberships: 10_000,
                rational_bits: 4096,
                triangles: 512
            }
        );
        assert_eq!(
            remaining_limits(2_000_001, 0).err().map(|e| e.kind),
            Some(FailureKind::Operations)
        );
        assert_eq!(
            remaining_limits(0, 100_001).err().map(|e| e.kind),
            Some(FailureKind::Storage)
        );
        assert_eq!(
            check_status(&refusal(FailureKind::Storage)),
            Status::Refused
        );
        assert_eq!(
            check_status(&refusal(FailureKind::InvalidWitness)),
            Status::Fail
        );
        Ok(())
    }
    #[test]
    fn artifact_strings_escape_json_and_bind_the_fixed_input() {
        assert_eq!(quote("a\"b\\c\n\t\u{0001}"), "\"a\\\"b\\\\c\\n\\t\\u0001\"");
        let report = CaseReport::pending(OriginalAsset::Sh011);
        assert!(report.json().contains(input_sha(OriginalAsset::Sh011)));
        assert_eq!(input_sha(OriginalAsset::Sh011).len(), 64);
    }
    #[test]
    fn small_closed_candidate_exercises_every_dependent_public_check() -> Result<(), Failure> {
        use meridian_geometry::{SourceId, Vertex};
        let points = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
        let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let mut b = Budget::new(Limits::default())?;
        let originals = faces
            .into_iter()
            .enumerate()
            .map(|(i, face)| {
                Facet::new(
                    SourceId {
                        component: 0,
                        facet: i as u32,
                    },
                    face.map(|j| Vertex {
                        point: points[j].clone(),
                        uv: [integer(0), integer(0)],
                    }),
                    "fixture".into(),
                    &mut b,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let hull = reconstruct(&originals, &mut b)?;
        assert_eq!(hull.exact_volume(), &(integer(1) / integer(6)));
        fields(&hull, &originals, &mut b)?;
        certificates(&hull, &originals, &mut b)?;
        topology(&hull, &mut b)?;
        assert!(candidate_wire(&hull).starts_with("meridian-exact-facets-v1 4\n"));
        assert!(witness_wire(&hull).contains("\"exactVolume\":\"1/6\""));
        Ok(())
    }

    #[test]
    fn adapter_serializes_resource_rejection_without_publishing_candidate() -> Result<(), Failure> {
        use meridian_geometry::{SourceId, Vertex};
        let mut setup = Budget::new(Limits::default())?;
        let points = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
        let originals = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]
            .into_iter()
            .enumerate()
            .map(|(i, face)| {
                Facet::new(
                    SourceId {
                        component: 0,
                        facet: i as u32,
                    },
                    face.map(|j| Vertex {
                        point: points[j].clone(),
                        uv: [integer(0), integer(0)],
                    }),
                    "fixture".into(),
                    &mut setup,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut b = Budget::new(Limits {
            storage_memberships: 600,
            ..Limits::default()
        })?;
        let error = reconstruct(&originals, &mut b)
            .err()
            .ok_or_else(|| failed(&b, "expected adapter fixture rejection"))?;
        let snapshot = b
            .rejection()
            .ok_or_else(|| failed(&b, "missing adapter budget snapshot"))?;
        assert!(snapshot.retained_before > 0);
        assert_eq!(snapshot.kind, FailureKind::Storage);
        let mut report = CaseReport::pending(OriginalAsset::Sh010);
        report.bodies[0] = Status::Pass;
        report.record_failure(1, error);
        report.budget_rejection = Some((snapshot, 0, 0));
        let json = report.json();
        assert!(json.contains("\"phase\":\"input_audit\""));
        assert!(json.contains(&format!(
            "\"retainedBeforeCleanup\":{}",
            snapshot.retained_before
        )));
        assert!(json.contains(&format!(
                "\"requiredMemberships\":{}",
                snapshot
                    .required_memberships
                    .ok_or_else(|| failed(&b, "missing requirement"))?
            )));
        assert!(json.contains("\"completeCandidateReturned\":false"));
        assert_eq!(report.bodies[2..], [Status::Unrun; 3]);
        Ok(())
    }
}
