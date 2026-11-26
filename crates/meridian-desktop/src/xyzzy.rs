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

const ACTIONS: [(XyzzyAction, &str); 4] = [
    (
        XyzzyAction::Enter,
        "Activate CLARKE and fit maximum equipment",
    ),
    (
        XyzzyAction::MaximumSupportedHardware,
        "Refill and fit maximum equipment",
    ),
    (
        XyzzyAction::RestoreStarter,
        "Refill / refit current SH025 starter outfit",
    ),
    (
        XyzzyAction::ToggleUnlimitedFuel,
        "Toggle accounted unlimited fuel",
    ),
];
const SHIELD_ACTION: usize = ACTIONS.len();
const MAX_TANK_ACTION: usize = SHIELD_ACTION + 1;
const ACTION_COUNT: usize = ACTIONS.len() + 2;

impl Menu {
    pub fn action(&self) -> XyzzyAction {
        if self.selected == SHIELD_ACTION {
            XyzzyAction::FitShield(ShieldGrade::ALL[self.shield_index])
        } else if self.selected == MAX_TANK_ACTION {
            XyzzyAction::MaximumFuelStorage
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
                XyzzyAction::Enter if snapshot.is_some_and(|s| !s.active) => {
                    "Sets cash to 1,000,000 CR once and fits maximum compatible equipment."
                }
                XyzzyAction::ToggleUnlimitedFuel => "Switch between normal and unlimited fuel.",
                XyzzyAction::FitShield(_) => {
                    "Fits only the selected shield grade; no cash or fuel grant."
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
                "Up / Down selects an action; Enter reviews, then Enter confirms",
                "Escape / B cancels a review, then closes this testing menu",
                "Actions publish the exact save before replacing the live commander",
                "First activation names CLARKE and SETS cash to 1,000,000 CR once",
                "Activation fits maximum compatible equipment at any station berth",
                "Test shield: choose the row, then Left / Right selects one grade",
                "Test shield fits only that grade, without a paid or earned receipt",
                "TEST MAX fits 4 tanks beyond normal SH025 mounts; no active custody",
                "Store test tanks to meet ordinary fitting limits before contracts",
                "After confirmation you can choose another action here",
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
