use crate::SourceId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub operations: u64,
    pub storage_memberships: usize,
    pub rational_bits: u64,
    pub triangles: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            operations: 2_000_000,
            storage_memberships: 100_000,
            rational_bits: 4096,
            triangles: 512,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureKind {
    Operations,
    Storage,
    Rational,
    TriangleLimit,
    InvalidInput,
    Degenerate,
    ConflictingFields,
    NonManifold,
    UncertifiedContact,
    InvalidWitness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub kind: FailureKind,
    pub source: Option<SourceId>,
    pub operations: u64,
    pub detail: &'static str,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} at {:?} after {} operations: {}",
            self.kind, self.source, self.operations, self.detail
        )
    }
}
impl std::error::Error for Failure {}

/// Fixed control state used to identify the first rejected resource charge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetPhase {
    Other,
    Decode,
    InputAudit,
    Index,
    Sections,
    ArrangementSetup,
    ArrangementPairs,
    ArrangementSlabs,
    Occupancy,
    Fields,
    GlobalPoints,
    Packing,
    Noding,
    Triangulation,
    ContactStars,
    Witness,
    Topology,
    DependentChecks,
}
impl BudgetPhase {
    pub const ALL: [Self; 18] = [
        Self::Other,
        Self::Decode,
        Self::InputAudit,
        Self::Index,
        Self::Sections,
        Self::ArrangementSetup,
        Self::ArrangementPairs,
        Self::ArrangementSlabs,
        Self::Occupancy,
        Self::Fields,
        Self::GlobalPoints,
        Self::Packing,
        Self::Noding,
        Self::Triangulation,
        Self::ContactStars,
        Self::Witness,
        Self::Topology,
        Self::DependentChecks,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Other => "other",
            Self::Decode => "decode",
            Self::InputAudit => "input_audit",
            Self::Index => "index",
            Self::Sections => "sections",
            Self::ArrangementSetup => "arrangement_setup",
            Self::ArrangementPairs => "arrangement_pairs",
            Self::ArrangementSlabs => "arrangement_slabs",
            Self::Occupancy => "occupancy",
            Self::Fields => "fields",
            Self::GlobalPoints => "global_points",
            Self::Packing => "packing",
            Self::Noding => "noding",
            Self::Triangulation => "triangulation",
            Self::ContactStars => "contact_stars",
            Self::Witness => "witness",
            Self::Topology => "topology",
            Self::DependentChecks => "dependent_checks",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetRejection {
    pub kind: FailureKind,
    pub source: Option<SourceId>,
    pub phase: BudgetPhase,
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    pub operations_before: u64,
    pub operations_after: u64,
    pub attempted_operations: u64,
    pub retained_before: usize,
    pub attempted_scratch: usize,
    pub required_memberships: Option<usize>,
    pub phase_operations: [u64; 18],
    /// Stage-local counters named by the reporting adapter's inventory schema.
    pub state: [usize; 8],
    pub state_at_operations: u64,
    pub limits: Limits,
}

#[derive(Debug)]
pub struct Budget {
    limits: Limits,
    operations: u64,
    retained: usize,
    peak: usize,
    admitted_peak: usize,
    source: Option<SourceId>,
    phase: BudgetPhase,
    state: [usize; 8],
    state_at_operations: u64,
    phase_operations: [u64; 18],
    rejection: Option<BudgetRejection>,
}

impl Budget {
    pub fn new(limits: Limits) -> Result<Self, Failure> {
        let original = Limits::default();
        if limits.operations > original.operations
            || limits.storage_memberships > original.storage_memberships
            || limits.rational_bits > original.rational_bits
            || limits.triangles > original.triangles
        {
            return Err(Failure {
                kind: FailureKind::InvalidInput,
                source: None,
                operations: 0,
                detail: "limits may tighten the original guards only",
            });
        }
        Ok(Self {
            limits,
            operations: 0,
            retained: 0,
            peak: 0,
            admitted_peak: 0,
            source: None,
            phase: BudgetPhase::Other,
            state: [0; 8],
            state_at_operations: 0,
            phase_operations: [0; 18],
            rejection: None,
        })
    }
    pub fn rejection(&self) -> Option<BudgetRejection> {
        self.rejection
    }
    pub fn phase_operations(&self) -> &[u64; 18] {
        &self.phase_operations
    }
    pub fn with_phase<T>(
        &mut self,
        phase: BudgetPhase,
        operation: impl FnOnce(&mut Self) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        self.with_phase_state(phase, [0; 8], operation)
    }
    pub(crate) fn with_phase_state<T>(
        &mut self,
        phase: BudgetPhase,
        state: [usize; 8],
        operation: impl FnOnce(&mut Self) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        let previous = (self.phase, self.state, self.state_at_operations);
        self.phase = phase;
        self.state = state;
        self.state_at_operations = self.operations;
        let result = operation(self);
        (self.phase, self.state, self.state_at_operations) = previous;
        result
    }
    pub(crate) fn phase(&mut self, phase: BudgetPhase) {
        self.phase = phase;
    }
    pub(crate) fn state(&mut self, values: [usize; 8]) {
        self.state = values;
        self.state_at_operations = self.operations;
    }
    pub(crate) fn state_values(&self) -> [usize; 8] {
        self.state
    }
    fn record_rejection(
        &mut self,
        kind: FailureKind,
        caller: &'static std::panic::Location<'static>,
        operations_before: u64,
        attempted_operations: u64,
        attempted_scratch: usize,
        required_memberships: Option<usize>,
    ) {
        if self.rejection.is_none() {
            self.rejection = Some(BudgetRejection {
                kind,
                source: self.source,
                phase: self.phase,
                file: caller.file(),
                line: caller.line(),
                column: caller.column(),
                operations_before,
                operations_after: self.operations,
                attempted_operations,
                retained_before: self.retained,
                attempted_scratch,
                required_memberships,
                phase_operations: self.phase_operations,
                state: self.state,
                state_at_operations: self.state_at_operations,
                limits: self.limits,
            });
        }
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    pub fn operations(&self) -> u64 {
        self.operations
    }
    pub fn peak_memberships(&self) -> usize {
        self.peak
    }
    /// Largest successful logical reservation, distinct from refused attempts.
    pub fn peak_admitted_memberships(&self) -> usize {
        self.admitted_peak
    }
    pub fn retained_memberships(&self) -> usize {
        self.retained
    }
    pub(crate) fn at(&mut self, source: SourceId) {
        self.source = Some(source);
    }
    pub(crate) fn fail(&self, kind: FailureKind, detail: &'static str) -> Failure {
        Failure {
            kind,
            source: self.source,
            operations: self.operations,
            detail,
        }
    }
    #[track_caller]
    pub fn step(&mut self, amount: u64) -> Result<(), Failure> {
        let before = self.operations;
        self.operations = self.operations.saturating_add(amount);
        self.phase_operations[self.phase as usize] =
            self.phase_operations[self.phase as usize].saturating_add(amount);
        if self.operations > self.limits.operations {
            self.record_rejection(
                FailureKind::Operations,
                std::panic::Location::caller(),
                before,
                amount,
                0,
                None,
            );
            return Err(self.fail(FailureKind::Operations, "exact operation budget exhausted"));
        }
        Ok(())
    }
    #[track_caller]
    pub fn guard(&mut self, scratch: usize) -> Result<(), Failure> {
        let caller = std::panic::Location::caller();
        let required = self.retained.checked_add(scratch).ok_or_else(|| {
            self.record_rejection(
                FailureKind::Storage,
                caller,
                self.operations,
                0,
                scratch,
                None,
            );
            self.fail(FailureKind::Storage, "membership count overflow")
        })?;
        self.peak = self.peak.max(required);
        if required > self.limits.storage_memberships {
            self.record_rejection(
                FailureKind::Storage,
                caller,
                self.operations,
                0,
                scratch,
                Some(required),
            );
            return Err(self.fail(FailureKind::Storage, "exact storage budget exhausted"));
        }
        self.admitted_peak = self.admitted_peak.max(required);
        Ok(())
    }
    #[track_caller]
    pub(crate) fn retain(&mut self, members: usize) -> Result<(), Failure> {
        self.guard(members)?;
        self.retained += members;
        Ok(())
    }
    #[track_caller]
    pub(crate) fn with_retained<T>(
        &mut self,
        members: usize,
        operation: impl FnOnce(&mut Self) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        self.retain(members)?;
        let result = operation(self);
        self.release(members);
        result
    }
    pub(crate) fn release(&mut self, members: usize) {
        self.retained -= members;
    }
}
