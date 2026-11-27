//! In-flight route review uses the same candidates and saved plans as the station chart.
use super::{App, KeyCode};
use crate::help::Panel;
use meridian_core::World;
use meridian_core::navigation::{OriginalNavigation, RoutePlan};

struct Choice {
    plan: RoutePlan,
    path: String,
    distance_ly: f64,
    preferences: String,
}

pub(super) struct Planner {
    choices: Vec<Choice>,
    selected: usize,
}

impl Planner {
    fn open(world: &World) -> Result<Self, String> {
        let routes = world
            .flight_route_alternatives()
            .map_err(|e| e.to_string())?;
        let catalogue = OriginalNavigation::bundled().map_err(|e| e.to_string())?;
        let mut choices = Vec::new();
        for route in routes {
            let names = route
                .plan
                .systems()
                .iter()
                .map(|&id| catalogue.system(id).map(|system| system.name))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            choices.push(Choice {
                path: names.join(" > "),
                distance_ly: route.distance_ly,
                preferences: route
                    .preferences
                    .iter()
                    .map(|pref| pref.label())
                    .collect::<Vec<_>>()
                    .join(" / "),
                plan: route.plan,
            });
        }
        Ok(Self {
            choices,
            selected: 0,
        })
    }

    pub(super) fn panel(&self) -> Panel {
        let choice = &self.choices[self.selected];
        let mut lines: Vec<_> = self
            .choices
            .iter()
            .enumerate()
            .map(|(i, choice)| {
                format!(
                    "{} {}",
                    if i == self.selected { ">" } else { " " },
                    choice.path
                )
            })
            .collect();
        lines.push(format!(
            "{} computer | {:.2} LY total",
            choice.plan.grade().label(),
            choice.distance_ly
        ));
        lines.push(choice.preferences.clone());
        Panel {
            title: "Flight navigation".into(),
            lines,
            footer: "Up/Down Select | Enter Save route | Escape Back",
        }
    }
}

impl App {
    pub(super) fn open_flight_routes(&mut self) {
        if !self.simulation_active() {
            return;
        }
        match Planner::open(&self.game.world) {
            Ok(planner) => {
                self.clear_held_input();
                self.flight_routes = Some(planner);
            }
            Err(message) => self.notice.set(message),
        }
        self.request_redraw();
    }

    pub(super) fn flight_route_key(&mut self, code: KeyCode, repeat: bool) -> bool {
        let Some(planner) = self.flight_routes.as_mut() else {
            return false;
        };
        if repeat {
            return true;
        }
        match code {
            KeyCode::ArrowUp => planner.selected = planner.selected.saturating_sub(1),
            KeyCode::ArrowDown => {
                planner.selected = (planner.selected + 1).min(planner.choices.len() - 1)
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let result = self
                    .game
                    .navigation_route(Some(&planner.choices[planner.selected].plan));
                self.flight_routes = None;
                self.notice.set(result.unwrap_or_else(|error| error));
            }
            KeyCode::Escape => self.flight_routes = None,
            _ => return true,
        }
        self.request_redraw();
        true
    }
}
