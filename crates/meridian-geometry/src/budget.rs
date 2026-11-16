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

#[derive(Debug)]
pub struct Budget {
    limits: Limits,
    operations: u64,
    retained: usize,
    peak: usize,
    source: Option<SourceId>,
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
            source: None,
        })
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
    pub fn step(&mut self, amount: u64) -> Result<(), Failure> {
        self.operations = self.operations.saturating_add(amount);
        if self.operations > self.limits.operations {
            return Err(self.fail(FailureKind::Operations, "exact operation budget exhausted"));
        }
        Ok(())
    }
    pub fn guard(&mut self, scratch: usize) -> Result<(), Failure> {
        let required = self
            .retained
            .checked_add(scratch)
            .ok_or_else(|| self.fail(FailureKind::Storage, "membership count overflow"))?;
        self.peak = self.peak.max(required);
        if required > self.limits.storage_memberships {
            return Err(self.fail(FailureKind::Storage, "exact storage budget exhausted"));
        }
        Ok(())
    }
    pub(crate) fn retain(&mut self, members: usize) -> Result<(), Failure> {
        self.guard(members)?;
        self.retained += members;
        Ok(())
    }
    pub(crate) fn release(&mut self, members: usize) {
        self.retained -= members;
    }
}
