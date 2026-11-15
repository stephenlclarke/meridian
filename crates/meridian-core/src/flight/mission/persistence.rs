use super::*;
use crate::save::{Reader, SaveError, Writer};

impl FreightSession {
    pub(crate) fn encode_save(&self, w: &mut Writer) {
        self.state.encode_save(w);
        w.u64(self.epoch);
        w.bool(self.paused);
        w.option(self.roster.as_ref(), |w, npc| {
            npc.dock.encode_save(w);
            w.u64(npc.actor.entity);
            w.u64(npc.actor.generation);
            w.u8(match npc.definition {
                ContactDefinition::Arlen => 0,
                ContactDefinition::RenasUtilityCrew => 1,
            });
            w.u64(npc.roster_revision);
        });
    }
    pub(crate) fn decode_save(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            state: g02::State::decode_save(r)?,
            epoch: r.u64()?,
            paused: r.bool()?,
            roster: r.option(|r| {
                let dock = DockContext::decode_save(r)?;
                let actor = ActorId {
                    entity: r.u64()?,
                    generation: r.u64()?,
                };
                let definition = match r.u8()? {
                    0 => ContactDefinition::Arlen,
                    1 => ContactDefinition::RenasUtilityCrew,
                    _ => return Err(SaveError::InvalidEncoding),
                };
                Ok(StationNpc {
                    dock,
                    actor,
                    definition,
                    roster_revision: r.u64()?,
                })
            })?,
            contact: None,
        })
    }
    pub(crate) fn validate_save(&self, dock: Option<DockContext>) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("station roster or session");
        if self.epoch == 0 {
            return Err(invalid);
        }
        let expected = dock.and_then(|d| match d.system() {
            SystemId::AVENOR => Some((2, ContactDefinition::Arlen)),
            SystemId::RENAS => Some((3, ContactDefinition::RenasUtilityCrew)),
            _ => None,
        });
        match (expected, self.roster) {
            (None, None) => Ok(()),
            (Some((entity, definition)), Some(npc))
                if Some(npc.dock) == dock
                    && npc.actor.entity == entity
                    && npc.definition == definition
                    && npc.actor.generation > 0
                    && npc.actor.generation <= self.epoch
                    && npc.roster_revision == npc.actor.generation =>
            {
                Ok(())
            }
            _ => Err(invalid),
        }
    }
    pub(crate) fn continue_floor(&self) -> (u64, u64) {
        (self.epoch, self.state.saved_review_sequence())
    }
    pub(crate) fn invalidate_after_continue(&mut self, floor: (u64, u64)) -> Result<(), SaveError> {
        let epoch = self
            .epoch
            .max(floor.0)
            .checked_add(1)
            .ok_or(SaveError::SequenceExhausted)?;
        self.state.invalidate_saved_review(floor.1)?;
        self.epoch = epoch;
        self.contact = None;
        // The existing roster/dock/owned items are retained, not minted from presentation state.
        Ok(())
    }
}
