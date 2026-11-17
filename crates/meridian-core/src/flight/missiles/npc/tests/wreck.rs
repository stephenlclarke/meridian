//! Real native actions create the wreck source. Direct edits appear only in a corruption test.
use super::*;
use crate::navigation::{BASIC_SPOOL_TICKS, SystemId};
use crate::station_services::MarketSite;
use crate::traffic::{WreckDamageSource, WreckManifest, WreckManifestError};

fn paid_wreck() -> (World, WreckManifest) {
    let mut w = ready(4);
    let source = trader(&w);
    let (commodity, quantity) = source.cargo.unwrap();
    assert_eq!(
        w.wreck_manifest(source.id),
        Err(WreckManifestError::SourceAlive)
    );
    let opening_cargo = w.commodity_quantity(commodity);
    let opening_cash = w.credits_tenths();
    let opening_market = w
        .station_markets
        .as_ref()
        .unwrap()
        .commodity_quote(MarketSite::Avenor, commodity);
    for _ in 0..7 {
        let origin = fire_npc(&mut w);
        let projectile = finish_npc(&mut w, origin.projectile);
        assert_eq!(projectile.contact, Some(MissileContact::Npc(source.id)));
        assert_native_endpoint(&w, projectile);
        if trader(&w).phase == NpcPhase::Destroyed {
            let wreck = w.wreck_manifest(source.id).unwrap();
            assert_eq!(
                (wreck.source, wreck.hull_id, wreck.commodity, wreck.quantity),
                (source.id, "SH025", commodity, quantity)
            );
            assert_eq!(wreck.cause, WreckDamageSource::Missile(origin.projectile));
            assert_eq!(wreck.impact_point, projectile.position);
            assert_eq!(wreck.pose, trader(&w).pose);
            assert_eq!(wreck.destroyed_at_tick, projectile.last_tick);
            assert!(wreck.paid_at_tick < wreck.destroyed_at_tick);
            assert!(wreck.destroyed_at_local_tick > 0);
            assert_eq!(wreck.planned_lot_quantities().collect::<Vec<_>>(), [5, 4]);
            assert_eq!(trader(&w).cargo, None);
            assert_eq!(w.commodity_quantity(commodity), opening_cargo);
            assert_eq!(w.credits_tenths(), opening_cash);
            assert_eq!(
                w.station_markets
                    .as_ref()
                    .unwrap()
                    .commodity_quote(MarketSite::Avenor, commodity),
                opening_market
            );
            return (w, wreck);
        }
        assert_eq!(
            w.wreck_manifest(source.id),
            Err(WreckManifestError::SourceAlive)
        );
        while w.missile_snapshot().unwrap().cooldown_ticks > 0 {
            step(&mut w, 1);
        }
        w = cold(&w);
    }
    panic!("real paid native hits did not destroy trader");
}

#[test]
fn actual_fatal_missile_settles_one_paid_manifest_and_cold_readout_never_issues_loot() {
    let (mut w, wreck) = paid_wreck();
    let before = w.save_bytes().unwrap();
    assert_eq!(w.wreck_manifest(wreck.source), Ok(wreck));
    assert_eq!(w.wreck_manifest(wreck.source), Ok(wreck));
    assert_eq!(w.save_bytes().unwrap(), before);
    let tick = w.snapshot().tick;
    assert_eq!(
        w.request_npc_missile_lock(Some(wreck.source), tick),
        Err(MissileError::NoLiveLock)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut loaded = cold(&w);
    assert_eq!(loaded.wreck_manifest(wreck.source), Ok(wreck));
    // A real jump removes the local source from the current World query. There was never a pod.
    let tick = loaded.snapshot().tick;
    loaded.request_jump(SystemId::RENAS, tick).unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        step(&mut loaded, 1);
    }
    assert_eq!(
        loaded.wreck_manifest(wreck.source),
        Err(WreckManifestError::OutsideSystem)
    );
    assert_eq!(
        loaded.commodity_quantity(wreck.commodity),
        w.commodity_quantity(wreck.commodity)
    );
    let loaded = cold(&loaded);
    assert_eq!(
        loaded.wreck_manifest(wreck.source),
        Err(WreckManifestError::OutsideSystem)
    );
}

#[test]
fn actual_live_sold_foreign_and_not_yet_admitted_traders_have_no_wreck_manifest() {
    let w = ready(0);
    let local = trader(&w).id;
    let foreign = trader(&ready_foreign()).id;
    let fresh = World::new_commander(12005).unwrap();
    assert_eq!(
        World::new().wreck_manifest(local),
        Err(WreckManifestError::NoCommander)
    );
    assert_eq!(
        fresh.wreck_manifest(local),
        Err(WreckManifestError::UnknownActor)
    );
    let mut w = w;
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.wreck_manifest(local),
        Err(WreckManifestError::SourceAlive)
    );
    assert_eq!(
        w.wreck_manifest(foreign),
        Err(WreckManifestError::ForeignActor)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    for _ in 0..24000 {
        if trader(&w).phase == NpcPhase::Complete {
            let before = w.save_bytes().unwrap();
            assert_eq!(
                w.wreck_manifest(local),
                Err(WreckManifestError::ManifestSold)
            );
            assert_eq!(w.save_bytes().unwrap(), before);
            assert_eq!(
                cold(&w).wreck_manifest(local),
                Err(WreckManifestError::ManifestSold)
            );
            return;
        }
        step(&mut w, 1);
    }
    panic!("actual trader did not complete native entry and sale");
}

#[test]
fn a_changed_terminal_native_contact_cannot_relocate_the_paid_manifest_or_replace_a_valid_world() {
    let (w, wreck) = paid_wreck();
    refuses(&w, |changed| {
        changed.traffic.corrupt_missile_hits(|hits| {
            let terminal = hits
                .iter_mut()
                .rfind(|hit| hit.victim == wreck.source)
                .unwrap();
            terminal.victim_pose.position.x += 1.0;
        });
        assert_eq!(
            changed.wreck_manifest(wreck.source),
            Err(WreckManifestError::MissingTerminalContact)
        );
    });
    assert_eq!(w.wreck_manifest(wreck.source), Ok(wreck));
}
