#![allow(clippy::unwrap_used)]
use super::*;
#[test]
fn stock_fast_definitions_and_policy_keep_source_prices_tl_owned_limits_and_reward_only_naval() {
    assert_eq!(ModuleKind::BeamLaser.shop(), Some((47000, 7)));
    assert_eq!(ModuleKind::MilitaryLaser.shop(), Some((164000, 14)));
    assert_eq!(ModuleKind::NavalLaser.shop(), None);
    for kind in [
        ModuleKind::BeamLaser,
        ModuleKind::MilitaryLaser,
        ModuleKind::NavalLaser,
    ] {
        assert_eq!(kind.maximum_owned(), 4);
        assert_eq!(kind.equipment_units(), 0);
        assert_eq!(kind.slot(), ModuleSlot::FrontLaser);
    }
    // Policy/ledger fixture only. It is not funded Military acquisition at the current TL12/13 sites.
    let world = crate::World::new_commander(17201).unwrap();
    let dock = world.current_dock_context().unwrap();
    let mut ledger = Ledger::born(17201).unwrap();
    for n in 1..=4 {
        ledger
            .apply(
                Event {
                    xyzzy: false,
                    action: ModuleAction::Purchase {
                        kind: ModuleKind::MilitaryLaser,
                    },
                    item: 117211 + u64::from(n),
                    dock,
                    tick: 0,
                    revision: u64::from(n),
                },
                n,
            )
            .unwrap();
    }
    assert_eq!(ledger.paid, 656000);
    assert_eq!(
        ledger.apply(
            Event {
                xyzzy: false,
                action: ModuleAction::Purchase {
                    kind: ModuleKind::MilitaryLaser
                },
                item: 117216,
                dock,
                tick: 0,
                revision: 5
            },
            5
        ),
        Err(ModuleError::Capacity)
    );
}
#[test]
fn real_secret_funded_four_beam_limit_counts_stored_property_and_preserves_paid_receipts() {
    let mut w = World::new_commander(17301).unwrap();
    let r = w.review_xyzzy(crate::XyzzyAction::Enter, 0).unwrap();
    w.prepare_xyzzy(r, 0).unwrap().commit();
    for _ in 0..4 {
        let r = w
            .review_module(
                ModuleAction::Purchase {
                    kind: ModuleKind::BeamLaser,
                },
                0,
            )
            .unwrap();
        let id = w.prepare_module(r, 0).unwrap().commit();
        let r = w
            .review_module(ModuleAction::Store { item: id }, 0)
            .unwrap();
        w.prepare_module(r, 0).unwrap().commit();
    }
    assert_eq!(w.credits_tenths(), Some(9_812_000));
    assert_eq!(
        w.modular_equipment_snapshot()
            .unwrap()
            .items
            .iter()
            .filter(|i| i.kind == ModuleKind::BeamLaser
                && i.slot.is_none()
                && matches!(
                    i.provenance,
                    ModuleProvenance::Paid {
                        paid_tenths: 47000,
                        ..
                    }
                ))
            .count(),
        4
    );
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::BeamLaser
            },
            0
        ),
        Err(ModuleError::Capacity)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
}
