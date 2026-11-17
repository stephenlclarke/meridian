//! Explicit testing menu; no readout or navigation grants gameplay state.
use crate::help::Panel;
use meridian_core::{World, XyzzyAction};

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
    pub reviewing: bool,
}

const ACTIONS: [(XyzzyAction, &str); 4] = [
    (XyzzyAction::Enter, "Activate Commander CLARKE"),
    (
        XyzzyAction::MaximumSupportedHardware,
        "Refill / refit supported maximum outfit",
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

impl Menu {
    pub fn action(&self) -> XyzzyAction {
        ACTIONS[self.selected].0
    }
    pub fn select(&mut self, forward: bool) {
        if !self.reviewing {
            self.selected =
                (self.selected + if forward { 1 } else { ACTIONS.len() - 1 }) % ACTIONS.len();
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
        lines.extend([
            "Supported: front Naval laser, Elite energy, Advanced navigation".into(),
            "Cargo bay, fitted scoops, finite nine-pylon missile outfit".into(),
            "No unlimited missiles/cargo or unavailable shields/drive/automation".into(),
            "Scoops fitting does not implement physical collection".into(),
            "Docked, unpaused actions only; ordinary paid history is retained".into(),
            if self.reviewing {
                "Enter CONFIRM AND SAVE | Escape / B Cancel review"
            } else {
                "Up / Down Choose | Enter Review | Escape / B Close"
            }
            .into(),
            "First activation SETS cash to 1,000,000 CR once".into(),
            "Confirm closes old interaction reviews; other profiles stay unchanged".into(),
            "H / ? Help | Close this menu before 1..5 / Tab station navigation".into(),
        ]);
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
                "Up / Down Choose | Enter Review | Escape / B Close"
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
                "Maximum and starter refit use the currently supported SH025 outfit",
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
