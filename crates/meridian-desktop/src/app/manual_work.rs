//! The pilot controls the approach. Each press operates the fitted tool once.
use super::{App, asteroid_render};
use meridian_core::{equipment::WorkTargetId, mining::MiningError};

/// The same Z binding drives held fire, one-shot work and the cockpit sight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlightTrigger {
    MountedLaser,
    Manual(WorkTargetId),
    OccupiedReview,
}

impl FlightTrigger {
    pub(crate) fn current(world: &meridian_core::World) -> Self {
        if world.selected_laser_view() == meridian_core::FlightView::Front {
            if world.pilot_operates_selected_tool()
                && let Some(target) = world.selected_work_target()
            {
                return Self::Manual(target.id);
            }
            if super::occupied_cut::ready_target(world).is_some() {
                return Self::OccupiedReview;
            }
        }
        Self::MountedLaser
    }
}

impl App {
    pub(super) fn use_manual_tool(&mut self) -> bool {
        let world = &mut self.game.world;
        let FlightTrigger::Manual(work) = FlightTrigger::current(world) else {
            return false;
        };
        if let Some(target) = world.selected_disabled_salvage_target() {
            let result = world.review_occupied_cut(target, world.snapshot().tick);
            match result {
                Ok(review) => {
                    self.clear_held_input();
                    self.occupied_cut = Some(review);
                }
                Err(err) => self.notice.set(err.to_string()),
            }
        } else {
            let message = match work {
                WorkTargetId::Wreck(_) => world.request_manual_wreck_recovery(),
                WorkTargetId::Ore(_) => match world.request_manual_ore_collection() {
                    Ok(()) => "Ore collected",
                    Err(MiningError::NoEligibleOreBody) => {
                        "Match speed and align the lower scoop aperture"
                    }
                    Err(MiningError::NoScoops) => "Fit fuel scoops",
                    Err(MiningError::HoldFull) => "Cargo hold full",
                    Err(err) => {
                        self.notice.set(err.to_string());
                        return true;
                    }
                },
                WorkTargetId::Asteroid(_) => {
                    let opening = world.snapshot();
                    match world.request_manual_mining() {
                        Ok(shot) => {
                            self.mining = Some(asteroid_render::MiningVisual { opening, shot });
                            self.publish_audio(true);
                            "Mining pulse"
                        }
                        Err(MiningError::NoCompatibleFittedTool) => "Fit a forward mining laser",
                        Err(MiningError::NoTarget | MiningError::Blocked) => {
                            "Aim at the selected asteroid within mining range"
                        }
                        Err(err) => {
                            self.notice.set(err.to_string());
                            return true;
                        }
                    }
                }
            };
            self.notice.set(message.into());
        }
        self.request_redraw();
        true
    }
}
