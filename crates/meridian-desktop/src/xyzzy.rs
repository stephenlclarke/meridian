//! Explicit testing menu; no readout or navigation grants gameplay state.
use crate::help::Panel;
use meridian_core::{ShieldGrade, World, XyzzyAction};

pub fn provenance(value: meridian_core::ModuleProvenance) -> &'static str {
    match value {
        meridian_core::ModuleProvenance::StandardBirth => "Born module",
        meridian_core::ModuleProvenance::Paid { .. } => "Paid module; original receipt retained",
        meridian_core::ModuleProvenance::XYZzy { .. } => "XYZZY grant; no paid purchase",
    }
}

#[derive(Default)]
pub struct Menu {
    selected: usize,
    shield_index: usize,
    pub reviewing: bool,
}

const ACTIONS: [(XyzzyAction, &str); 5] = [
    (XyzzyAction::ActivateMedusa, "Activate / refit SH171 Medusa"),
    (
        XyzzyAction::MaximumSupportedHardware,
        "Refill and fit maximum equipment",
    ),
    (
        XyzzyAction::RestoreStarter,
        "Refill / refit current equipment",
    ),
    (
        XyzzyAction::ToggleUnlimitedFuel,
        "Toggle accounted unlimited fuel",
    ),
    (XyzzyAction::FitFuelScoops, "Fit Fuel Scoops only"),
];
const SHIELD_ACTION: usize = ACTIONS.len();
const MAX_TANK_ACTION: usize = SHIELD_ACTION + 1;
const ECM_FIT_ACTION: usize = MAX_TANK_ACTION + 1;
const ECM_AUTO_ACTION: usize = ECM_FIT_ACTION + 1;
const ACTION_COUNT: usize = ACTIONS.len() + 4;

impl Menu {
    pub fn action(&self) -> XyzzyAction {
        if self.selected == SHIELD_ACTION {
            XyzzyAction::FitShield(ShieldGrade::ALL[self.shield_index])
        } else if self.selected == MAX_TANK_ACTION {
            XyzzyAction::MaximumFuelStorage
        } else if self.selected == ECM_FIT_ACTION {
            XyzzyAction::FitEcm
        } else if self.selected == ECM_AUTO_ACTION {
            XyzzyAction::ToggleAutoEcm
        } else {
            ACTIONS[self.selected].0
        }
    }
    pub fn select(&mut self, forward: bool) {
        if !self.reviewing {
            self.selected =
                (self.selected + if forward { 1 } else { ACTION_COUNT - 1 }) % ACTION_COUNT;
        }
    }
    pub fn select_shield(&mut self, forward: bool) {
        if !self.reviewing && self.selected == SHIELD_ACTION {
            let count = ShieldGrade::ALL.len();
            self.shield_index = (self.shield_index + if forward { 1 } else { count - 1 }) % count;
        }
    }
    pub fn back(&mut self) -> bool {
        if self.reviewing {
            self.reviewing = false;
            false
        } else {
            true
        }
    }
    pub fn panel(&self, world: &World, notice: Option<&str>) -> Panel {
        let snapshot = world.xyzzy_snapshot();
        let mut lines = vec![snapshot.map_or("NO COMMANDER", |s| s.label).into()];
        if let Some(state) = snapshot {
            lines.push(format!(
                "Cash {}.{} CR | Fuel {} | Current reach {:.1} LY",
                state.cash_tenths / 10,
                state.cash_tenths % 10,
                if state.unlimited_fuel {
                    "UNLIMITED"
                } else {
                    "NORMAL"
                },
                state.jump_limit_ly
            ));
        }
        lines.extend(ACTIONS.iter().enumerate().map(|(i, (_, label))| {
            format!("{} {}", if i == self.selected { ">" } else { " " }, label)
        }));
        lines.push(format!(
            "{} Test shield: {} (Left/Right)",
            if self.selected == SHIELD_ACTION {
                ">"
            } else {
                " "
            },
            ShieldGrade::ALL[self.shield_index].name()
        ));
        lines.push(format!(
            "{} TEST MAX: fit all four fuel tanks",
            if self.selected == MAX_TANK_ACTION {
                ">"
            } else {
                " "
            }
        ));
        lines.push(format!(
            "{} Fit E.C.M. only",
            if self.selected == ECM_FIT_ACTION {
                ">"
            } else {
                " "
            }
        ));
        lines.push(format!(
            "{} Toggle automatic E.C.M. ({})",
            if self.selected == ECM_AUTO_ACTION {
                ">"
            } else {
                " "
            },
            if snapshot.is_some_and(|s| s.automatic_ecm) {
                "ON"
            } else {
                "OFF"
            }
        ));
        if let Some(combat) = world.combat_snapshot() {
            lines.push(format!(
                "Fitted shield: {}",
                combat.player.shield_grade.name()
            ));
        }
        if let Some(emp) = world.owned_emp().filter(|item| item.fitted) {
            lines.push(format!(
                "EMP power {} / capacity {} / recharge {}",
                emp.grades.power, emp.grades.capacity, emp.grades.recharge
            ));
        }
        if let Some(tier) = world.targeting_tier() {
            lines.push(format!("Targeting computer: {}", tier.label()));
        }
        lines.push(
            match self.action() {
                XyzzyAction::ActivateMedusa if snapshot.is_some_and(|s| !s.active) => {
                    "Issue a test Medusa. First activation sets cash to 1,000,000 CR."
                }
                XyzzyAction::ToggleUnlimitedFuel => "Switch between normal and unlimited fuel.",
                XyzzyAction::FitShield(_) => {
                    "Fits only the selected shield grade; no cash or fuel grant."
                }
                XyzzyAction::FitFuelScoops => {
                    "Fits only an eligible scoop in a free ordinary mount; no refuel."
                }
                XyzzyAction::FitEcm => {
                    "Fits only stock E.C.M. in a free ordinary mount; no refuel."
                }
                XyzzyAction::ToggleAutoEcm => {
                    "Save automatic intent; incoming NPC missiles are not yet supported."
                }
                XyzzyAction::MaximumFuelStorage => {
                    "Test-only 4 tanks; exceeds normal mounts. No active custody."
                }
                _ => "Fits maximum compatible equipment regardless of local technology.",
            }
            .into(),
        );
        if let Some(notice) = notice {
            // Bound the text like every other overlay row; the renderer clips to its card.
            lines.push(notice.chars().take(crate::help::MAX_COLUMNS).collect());
        }
        Panel {
            title: if self.reviewing {
                "XYZZY testing - confirm change"
            } else {
                "XYZZY testing"
            }
            .into(),
            lines,
            footer: if self.reviewing {
                "Enter Confirm and save | Escape / B Cancel"
            } else {
                "Up / Down Choose | Enter Review | H Help | Escape / B Close"
            },
        }
    }
    pub fn help(&self) -> Panel {
        Panel {
            title: "XYZZY testing help".into(),
            lines: [
                "Up / Down chooses; Enter reviews, then confirms; choose again after save",
                "Escape / B cancels a review, then closes this testing menu",
                "Actions publish the exact save before replacing the live commander",
                "First activation names CLARKE and SETS cash to 1,000,000 CR once",
                "Activation fits maximum compatible equipment at any station berth",
                "Test shield: Left / Right chooses one grade; no paid or earned receipt",
                "TEST MAX fits 4 tanks beyond normal SH025 mounts; no active custody",
                "Store test tanks to meet ordinary fitting limits before contracts",
                "Limited outfits keep tanks; E.C.M. needs a free mount or an existing fit",
                "Auto E.C.M. setting only; incoming NPC missiles remain unavailable",
                "Fuel mode is accounted separately; paid transactions still charge",
                "Missiles and cargo remain finite; full original parity is incomplete",
                "Close this menu first: 1 Galaxy | 2 System | 3 Showroom",
                "4 Market | 5 Equipment | Tab / Shift+Tab Next / previous screen",
                "Escape Back in station menus | H / ? Close help",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            footer: "Escape / H / ? Close help",
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ecm_help_keeps_all_controls_and_footer_with_audio_line() {
        let panel = super::Menu::default().help();
        assert!(panel.lines.len() + 3 <= crate::help::MAX_LINES);
        assert!(
            panel
                .lines
                .iter()
                .all(|line| line.len() <= crate::help::MAX_COLUMNS)
        );
        assert!(
            panel
                .lines
                .iter()
                .any(|line| line.contains("incoming NPC missiles"))
        );
    }
}
