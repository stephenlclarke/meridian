//! Pilot-selected position for transient flight feedback.
use super::{Vertex, label, quad};

pub(super) const FOOTER_HEIGHT: f32 = 34.0;
const SCALE: f32 = 1.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Placement {
    #[default]
    Instruments,
    UpperLeft,
    UpperRight,
    StatusOnly,
}

impl Placement {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Instruments => Self::UpperLeft,
            Self::UpperLeft => Self::UpperRight,
            Self::UpperRight => Self::StatusOnly,
            Self::StatusOnly => Self::Instruments,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Instruments => "Instruments",
            Self::UpperLeft => "Upper left",
            Self::UpperRight => "Upper right",
            Self::StatusOnly => "Status only",
        }
    }

    pub(crate) fn effective(self, urgent: bool, target_visible: bool) -> Self {
        if urgent || (target_visible && matches!(self, Self::UpperLeft | Self::UpperRight)) {
            Self::Instruments
        } else {
            self
        }
    }

    pub(crate) fn draw(self, message: &str, width: u32, height: u32, out: &mut Vec<Vertex>) {
        if self == Self::StatusOnly || width < 96 || height < 80 || message.is_empty() {
            return;
        }
        let size = [width as f32, height as f32];
        let char_width = 6.0 * SCALE;
        let limit = if self == Self::Instruments {
            size[0] - 24.
        } else {
            (size[0] * 0.5 - 28.).min(375.)
        };
        let columns = (limit / char_width).floor() as usize;
        if columns < 3 {
            return;
        }
        let mut rest = message.trim();
        let mut rows = ["", ""];
        for row in &mut rows {
            if rest.is_empty() {
                break;
            }
            let end = rest
                .char_indices()
                .nth(columns)
                .map_or(rest.len(), |(i, _)| i);
            let split = if end == rest.len() {
                end
            } else {
                rest[..end].rfind(' ').filter(|i| *i != 0).unwrap_or(end)
            };
            *row = &rest[..split];
            rest = rest[split..].trim_start();
        }
        let text_width = rows
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or_default() as f32
            * char_width;
        let x = if self == Self::UpperRight {
            size[0] - 12. - text_width
        } else {
            12.
        };
        let y = match self {
            Self::Instruments => size[1] - FOOTER_HEIGHT + 5.,
            Self::UpperLeft => 56., // Below the three station bearing rows.
            Self::UpperRight => 12.,
            Self::StatusOnly => return,
        };
        if self != Self::Instruments {
            let h = if rows[1].is_empty() { 10. } else { 22. };
            quad(
                out,
                [x - 4., y - 3., x + text_width + 3., y + h + 2.],
                [0.006, 0.012, 0.025],
                size,
            );
        }
        for (index, row) in rows.iter().enumerate() {
            label(
                out,
                row,
                x,
                y + index as f32 * 12.,
                SCALE,
                [0.82, 0.94, 1.0],
                size,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{cockpit::Cockpit, vertices_with_cockpit};
    use meridian_core::{FlightInput, World};

    #[test]
    fn launch_notice_stays_out_of_the_scene_and_moves_only_to_the_selected_corner() {
        let mut world = World::new_commander(11405).unwrap_or_else(|e| panic!("{e:?}"));
        world.request_launch(0).unwrap_or_else(|e| panic!("{e:?}"));
        world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut cockpit =
            Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        let message = ["Launch admitted. Follow the exit corridor.".to_owned()];
        for [width, height] in [[640, 360], [1280, 720], [853, 533]] {
            let mut frame = Vec::new();
            vertices_with_cockpit(&[], width, height, Some(&cockpit), &mut frame);
            let scene = frame.len();
            for (position, target, urgent, expected) in [
                (Placement::Instruments, false, false, Placement::Instruments),
                (Placement::UpperLeft, false, false, Placement::UpperLeft),
                (Placement::UpperRight, false, false, Placement::UpperRight),
                (Placement::StatusOnly, false, false, Placement::StatusOnly),
                (Placement::UpperRight, true, false, Placement::Instruments),
                (Placement::UpperLeft, true, false, Placement::Instruments),
                (Placement::StatusOnly, true, true, Placement::Instruments),
            ] {
                cockpit.notice_placement = Some(position.effective(urgent, target));
                vertices_with_cockpit(&message, width, height, Some(&cockpit), &mut frame);
                let extra = &frame[scene..];
                assert_eq!(extra.is_empty(), expected == Placement::StatusOnly);
                for v in extra {
                    let x = (v.position[0] + 1.) * width as f32 / 2.;
                    let y = (1. - v.position[1]) * height as f32 / 2.;
                    match expected {
                        Placement::Instruments => assert!(
                            y >= height as f32 - FOOTER_HEIGHT && v.color != [0.006, 0.012, 0.025]
                        ),
                        Placement::UpperLeft => {
                            assert!(x < width as f32 / 2. && (52.0..84.0).contains(&y))
                        }
                        Placement::UpperRight => assert!(x > width as f32 / 2. && y < 40.0),
                        Placement::StatusOnly => unreachable!(),
                    }
                }
            }
        }
    }
}
