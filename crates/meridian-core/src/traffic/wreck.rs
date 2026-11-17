//! Ordinary manifest settlement stops before eligible cargo geometry and actor issuance.
use super::{Actor, BeamSource, CommodityId, NpcId, NpcPhase, RigidPose, State, Vec3};
use crate::{ProjectileId, station_services::CargoUnit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WreckManifestError {
    NoCommander,
    ForeignActor,
    OutsideSystem,
    UnknownActor,
    SourceAlive,
    ManifestSold,
    NoOrdinaryManifest,
    MissingTerminalContact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WreckDamageSource {
    Pulse(BeamSource),
    Missile(ProjectileId),
}

/// Evidence of ordinary NPC cargo leaving the live trader at its actual terminal contact.
/// This describes source custody only. Planned quantities are not issued payloads or assets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WreckManifest {
    pub source: NpcId,
    pub hull_id: &'static str,
    pub commodity: CommodityId,
    pub quantity: u8,
    pub paid_at_tick: u64,
    pub destroyed_at_tick: u64,
    pub destroyed_at_local_tick: u64,
    pub cause: WreckDamageSource,
    pub pose: RigidPose,
    pub impact_point: Vec3,
}
impl WreckManifest {
    /// Original chunk limits, for the source plan. This never allocates a pod or payload ID.
    pub fn planned_lot_quantities(self) -> impl Iterator<Item = u32> {
        let limit: u32 = match self.commodity.unit() {
            CargoUnit::Tonne => 5,
            CargoUnit::Kilogram => 1000,
            CargoUnit::Gram => 1_000_000,
        };
        (0..u32::from(self.quantity))
            .step_by(limit as usize)
            .map(move |offset| (u32::from(self.quantity) - offset).min(limit))
    }
}

/// The encoded actor already owns the paid manifest and its terminal sale/death phase.
/// Interpret that single state for flight cargo, death settlement and save validation.
pub(super) enum ManifestCustody {
    Aboard,
    Sold,
    SettledAtDeath,
    None,
}
impl Actor {
    pub(super) fn manifest_custody(self) -> ManifestCustody {
        if self.id.serial != 1 || self.units == 0 {
            ManifestCustody::None
        } else if self.sold || self.phase == NpcPhase::Complete {
            ManifestCustody::Sold
        } else if self.phase == NpcPhase::Destroyed {
            ManifestCustody::SettledAtDeath
        } else {
            ManifestCustody::Aboard
        }
    }
}

#[derive(Clone, Copy)]
struct TerminalContact {
    world: u64,
    local: u64,
    cause: WreckDamageSource,
    pose: RigidPose,
    point: Vec3,
}
impl State {
    pub(crate) fn wreck_manifest(
        &self,
        source: NpcId,
    ) -> Result<WreckManifest, WreckManifestError> {
        let actor = self
            .actor_index(source)
            .and_then(|i| self.actors[i])
            .ok_or(WreckManifestError::UnknownActor)?;
        match actor.manifest_custody() {
            ManifestCustody::Aboard => return Err(WreckManifestError::SourceAlive),
            ManifestCustody::Sold => return Err(WreckManifestError::ManifestSold),
            ManifestCustody::None => return Err(WreckManifestError::NoOrdinaryManifest),
            ManifestCustody::SettledAtDeath => {}
        }
        let contact = self
            .terminal_manifest_contact(actor)
            .ok_or(WreckManifestError::MissingTerminalContact)?;
        Ok(WreckManifest {
            source,
            hull_id: super::SH025_ID,
            commodity: actor.manifest,
            quantity: actor.units,
            paid_at_tick: actor.born_world_tick,
            destroyed_at_tick: contact.world,
            destroyed_at_local_tick: contact.local,
            cause: contact.cause,
            pose: contact.pose,
            impact_point: contact.point,
        })
    }

    pub(super) fn validate_terminal_manifest(&self, actor: Actor) -> Result<(), super::SaveError> {
        if matches!(actor.manifest_custody(), ManifestCustody::SettledAtDeath)
            && self.terminal_manifest_contact(actor).is_none()
        {
            return Err(super::INVALID);
        }
        Ok(())
    }

    fn terminal_manifest_contact(&self, actor: Actor) -> Option<TerminalContact> {
        // Same local tick/world tick/stage ordering as the authoritative resource replay.
        // Within each stage Vec order is the producer's retained order.
        let beam = self
            .shots
            .iter()
            .enumerate()
            .filter(|(_, h)| h.victim == actor.id)
            .map(|(index, h)| {
                let phase = if matches!(h.source, BeamSource::Npc(_)) {
                    0
                } else {
                    2
                };
                (
                    (h.local_tick, h.world_tick, phase, index),
                    TerminalContact {
                        world: h.world_tick,
                        local: h.local_tick,
                        cause: WreckDamageSource::Pulse(h.source),
                        pose: h.victim_pose,
                        point: h.point,
                    },
                )
            });
        let missile = self
            .missile_hits
            .iter()
            .enumerate()
            .filter(|(_, h)| h.victim == actor.id)
            .map(|(index, h)| {
                (
                    (h.local_tick, h.world_tick, 1, index),
                    TerminalContact {
                        world: h.world_tick,
                        local: h.local_tick,
                        cause: WreckDamageSource::Missile(h.origin.projectile),
                        pose: h.victim_pose,
                        point: h.point,
                    },
                )
            });
        let (_, terminal) = beam.chain(missile).max_by_key(|(key, _)| *key)?;
        // A later save cannot move the destroyed source or borrow another actor's hit.
        (actor.resources.energy == 0
            && actor.speed == 0.0
            && terminal.local == actor.resource_clock
            && terminal.world >= actor.born_world_tick
            && terminal.pose == actor.pose())
        .then_some(terminal)
    }
}
