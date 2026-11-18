use super::*;
use crate::profile::tests::{TestDirectory, open};
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing public state"))
}
fn paint(menu: &mut Docked, profile: &Profile) -> ui::Frame {
    let frame = ok(menu.build_frame(&profile.world, some(ui::Viewport::new(640.0, 360.0)), 0.0));
    menu.presented(frame.clone());
    frame
}
fn key(menu: &mut Docked, profile: &mut Profile, code: winit::keyboard::KeyCode) {
    let key = some(super::super::key(code, false, ui::Screen::Equipment));
    ok(menu.input(profile, ui::Input::Key(key)));
}
#[test]
fn post_store_handoff_expires_on_other_input_focus_and_actual_anchor_drift() {
    use winit::keyboard::KeyCode as K;
    for variant in 0..4 {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(ok(profile.new_commander(97031 + variant)));
        ok(profile.xyzzy(meridian_core::XyzzyAction::Enter));
        ok(profile.command(Action::Module(ModuleAction::PurchaseLaser {
            kind: ModuleKind::BeamLaser,
            facing: LaserFacing::Front,
        })));
        let beam = some(
            some(profile.world.modular_equipment_snapshot())
                .items
                .into_iter()
                .find(|i| i.kind == ModuleKind::BeamLaser),
        )
        .id;
        let mut menu = ok(Docked::new(&profile.world));
        ok(menu.open(&profile.world, ui::Screen::Equipment));
        paint(&mut menu, &profile);
        for _ in 0..2 {
            key(&mut menu, &mut profile, K::ArrowDown);
            paint(&mut menu, &profile);
        }
        key(&mut menu, &mut profile, K::KeyU);
        let review = paint(&mut menu, &profile);
        assert!(review.labels.iter().any(|l| l.text == "Equipment review"));
        key(&mut menu, &mut profile, K::Enter);
        let stored = ok(profile.world.save_bytes());
        assert!(menu.stored_fit.is_some());
        if variant == 0 {
            // An unseen next review cannot be confirmed by a second immediate Enter.
            key(&mut menu, &mut profile, K::Enter);
            assert!(menu.stored_fit.is_none());
        } else {
            key(&mut menu, &mut profile, K::KeyY);
            assert!(menu.stored_fit.as_ref().is_some_and(|h| h.requested));
            match variant {
                1 => menu.focus_lost(),
                2 => {
                    ok(profile.command(Action::Module(ModuleAction::PurchaseLaser {
                        kind: ModuleKind::PulseLaser,
                        facing: LaserFacing::Left,
                    })));
                }
                _ => menu.close(),
            }
        }
        let before_build = ok(profile.world.save_bytes());
        let frame = paint(&mut menu, &profile);
        assert!(!frame.labels.iter().any(|l| l.text == "Equipment review"));
        assert!(menu.pending.is_none() && menu.stored_fit.is_none());
        assert_eq!(ok(profile.world.save_bytes()), before_build);
        assert_eq!(
            some(
                some(profile.world.modular_equipment_snapshot())
                    .items
                    .into_iter()
                    .find(|i| i.id == beam)
            )
            .slot,
            None
        );
        if variant == 2 {
            assert_ne!(before_build, stored);
            assert!(
                frame
                    .labels
                    .iter()
                    .any(|l| l.text.contains("Values changed; open a fresh review"))
            );
        } else {
            assert_eq!(before_build, stored);
        }
    }
}
