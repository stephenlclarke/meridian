//! Couple the complete ECM command to the host's actual atomic save publisher.
use super::{EcmError, EcmOrigin, State, World};
use crate::flight::{emp, missiles, practice, weapons::PulseResources};

struct Candidate {
    ecm: State,
    missiles: missiles::State,
    pulse: PulseResources,
    practice: Option<practice::Practice>,
    emp: emp::State,
}
impl Candidate {
    fn replace(self, world: &mut World) -> Self {
        Self {
            ecm: std::mem::replace(&mut world.ecm, self.ecm),
            missiles: std::mem::replace(&mut world.missiles, self.missiles),
            pulse: std::mem::replace(&mut world.pulse, self.pulse),
            practice: std::mem::replace(&mut world.practice, self.practice),
            emp: std::mem::replace(&mut world.emp, self.emp),
        }
    }
}

pub struct PreparedEcm<'a> {
    world: &'a mut World,
    candidate: Candidate,
    bytes: Vec<u8>,
    origin: EcmOrigin,
}
impl PreparedEcm<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn origin(&self) -> EcmOrigin {
        self.origin
    }
    pub fn publish_with<E>(
        self,
        writer: impl FnOnce(&[u8]) -> Result<(), E>,
    ) -> Result<EcmOrigin, E> {
        writer(&self.bytes)?;
        Ok(self.commit())
    }
    pub fn commit(self) -> EcmOrigin {
        self.candidate.replace(self.world);
        self.origin
    }
}

pub(super) fn prepare(
    world: &mut World,
    ecm: State,
    missiles: missiles::State,
    pulse: PulseResources,
    practice: Option<practice::Practice>,
    emp: emp::State,
    origin: EcmOrigin,
) -> Result<PreparedEcm<'_>, EcmError> {
    let original = Candidate {
        ecm,
        missiles,
        pulse,
        practice,
        emp,
    }
    .replace(world);
    let bytes = world.save_bytes();
    // Restore the exact live values even if whole-save admission fails. No cold decode,
    // input invalidation, timer closure or field ownership transition is used for staging.
    let candidate = original.replace(world);
    Ok(PreparedEcm {
        world,
        candidate,
        bytes: bytes.map_err(EcmError::Save)?,
        origin,
    })
}
