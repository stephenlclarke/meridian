//! Isolated G02 domain rules. No receiver, consent, wallet, save or trusted-receipt authority.
//! Integration must validate those boundaries before applying a returned completion delta.

pub mod g02;

pub use crate::flight::{
    BondedAction, BondedOffer, BondedRecord, BondedUnit, DispatchAction, DispatchOffer,
    DispatchRecord,
};

pub const REWARD_TENTHS: u32 = 7300;
pub const CLUE_ID: &str = "consented-freight-family-trace";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitId {
    source_id: u64,
    index: u8,
}

impl UnitId {
    pub fn source_id(self) -> u64 {
        self.source_id
    }

    pub fn index(self) -> u8 {
        self.index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreightError {
    SourceExhausted,
    ForeignUnit,
    AlreadyReceived,
    AlreadyComplete,
}

/// A transition delta, not proof of an actual receiver or permission to mint a trusted clue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Completion {
    pub source_id: u64,
    pub job: u8,
    pub credits_tenths: u32,
    pub clue_id: &'static str,
}

/// One isolated sixteen-unit batch. Allocated units never return to this source.
#[derive(Debug, PartialEq, Eq)]
pub struct SupplyBatch {
    source_id: u64,
    allocated_jobs: u8,
}

impl SupplyBatch {
    /// The caller owns stable source identity and must not recreate an existing source as fresh.
    pub fn new(source_id: u64) -> Self {
        Self {
            source_id,
            allocated_jobs: 0,
        }
    }

    pub fn available_units(&self) -> u8 {
        16 - self.allocated_jobs * 8
    }

    pub fn allocate(&mut self) -> Result<FreightJob, FreightError> {
        if self.allocated_jobs >= 2 {
            return Err(FreightError::SourceExhausted);
        }
        let first = self.allocated_jobs * 8;
        let job = FreightJob {
            source_id: self.source_id,
            job: self.allocated_jobs,
            issued: core::array::from_fn(|index| UnitId {
                source_id: self.source_id,
                index: first + index as u8,
            }),
            received: 0,
        };
        self.allocated_jobs += 1;
        Ok(job)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FreightJob {
    source_id: u64,
    job: u8,
    issued: [UnitId; 8],
    received: u8,
}

impl FreightJob {
    pub fn issued_units(&self) -> &[UnitId; 8] {
        &self.issued
    }

    pub fn remaining_units(&self) -> u8 {
        8 - self.received.count_ones() as u8
    }

    /// Caller must supply actual receiver/custody authority at the integration boundary.
    /// Only the final original unit produces a completion delta; replay returns an error.
    pub fn receive(&mut self, unit: UnitId) -> Result<Option<Completion>, FreightError> {
        if self.received == u8::MAX {
            return Err(FreightError::AlreadyComplete);
        }
        let index = self
            .issued
            .iter()
            .position(|issued| *issued == unit)
            .ok_or(FreightError::ForeignUnit)?;
        let bit = 1_u8 << index;
        if self.received & bit != 0 {
            return Err(FreightError::AlreadyReceived);
        }
        self.received |= bit;
        Ok((self.received == u8::MAX).then_some(Completion {
            source_id: self.source_id,
            job: self.job,
            credits_tenths: REWARD_TENTHS,
            clue_id: CLUE_ID,
        }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn state(job: &FreightJob) -> (u64, u8, [UnitId; 8], u8) {
        (job.source_id, job.job, job.issued, job.received)
    }

    #[test]
    fn two_disjoint_issues_exhaust_original_sixteen_unit_source() {
        let mut source = SupplyBatch::new(7);
        let first = source.allocate().unwrap();
        assert_eq!(source.available_units(), 8);
        let second = source.allocate().unwrap();
        assert_eq!(source.available_units(), 0);
        for unit in first.issued_units() {
            assert!(!second.issued_units().contains(unit));
        }
        let before = source.allocated_jobs;
        assert_eq!(source.allocate(), Err(FreightError::SourceExhausted));
        assert_eq!(source.allocated_jobs, before);
        // Allocations never offer a return-to-source operation, even after job loss.
        assert_eq!(source.available_units(), 0);
    }

    #[test]
    fn seven_then_eighth_settles_once_and_preserves_original_unit_identity() {
        let mut source = SupplyBatch::new(7);
        let mut job = source.allocate().unwrap();
        let units = *job.issued_units();
        for (index, unit) in units.iter().take(7).enumerate() {
            assert_eq!(job.receive(*unit), Ok(None));
            assert_eq!(job.remaining_units(), 7 - index as u8);
        }
        let completion = job.receive(units[7]).unwrap().unwrap();
        assert_eq!(completion.source_id, 7);
        assert_eq!(completion.credits_tenths, 7300);
        assert_eq!(completion.clue_id, "consented-freight-family-trace");
        assert_eq!(job.remaining_units(), 0);
        assert_eq!(job.issued_units(), &units);
        let before = state(&job);
        assert_eq!(job.receive(units[7]), Err(FreightError::AlreadyComplete));
        assert_eq!(state(&job), before);
        assert_eq!(source.available_units(), 8);
    }

    #[test]
    fn duplicate_or_other_jobs_units_cannot_advance_or_pay() {
        let mut source = SupplyBatch::new(7);
        let mut first = source.allocate().unwrap();
        let second = source.allocate().unwrap();
        let original = first.issued_units()[0];
        assert_eq!(first.receive(original), Ok(None));
        let before = state(&first);
        assert_eq!(first.receive(original), Err(FreightError::AlreadyReceived));
        assert_eq!(state(&first), before);
        assert_eq!(
            first.receive(second.issued_units()[0]),
            Err(FreightError::ForeignUnit)
        );
        assert_eq!(state(&first), before);
    }

    #[test]
    fn separate_sources_cannot_cross_receive_the_same_local_unit_index() {
        let mut first_source = SupplyBatch::new(501);
        let mut second_source = SupplyBatch::new(502);
        let mut first_job = first_source.allocate().unwrap();
        let second_job = second_source.allocate().unwrap();
        let first_unit = first_job.issued_units()[0];
        let foreign_unit = second_job.issued_units()[0];
        assert_eq!(first_unit.index(), foreign_unit.index());
        assert_eq!(first_unit.source_id(), 501);
        assert_eq!(foreign_unit.source_id(), 502);
        assert_ne!(first_unit, foreign_unit);
        let before = state(&first_job);
        assert_eq!(
            first_job.receive(foreign_unit),
            Err(FreightError::ForeignUnit)
        );
        assert_eq!(state(&first_job), before);
        assert_eq!(first_job.receive(first_unit), Ok(None));
    }
}
