//! Native approach reservation components. No source test was executed by the author.
use super::super::{
    CAPITAL, CommodityId, MarketSite, OriginalStationData, Population, RigidPose, State,
    StationMarkets, SystemId, TickContext, Vec3, add, length, pose, sub,
};
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing actual actor"))
}
fn clear(_: Vec3, _: Vec3, _: f64) -> bool {
    true
}
fn context(
    data: &OriginalStationData,
    tick: u64,
    before: RigidPose,
    after: RigidPose,
) -> TickContext<'_> {
    TickContext {
        system: SystemId::AVENOR,
        data,
        player_before: before,
        player_after: after,
        world_tick: tick,
        protected: false,
        celestial_clear: &clear,
        beam_clear: &clear,
        player_target: None,
    }
}
#[test]
fn actual_approach_defers_only_launch_payment_then_native_birth_resumes_after_player_clears() {
    let data = ok(OriginalStationData::bundled());
    let mut traffic = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    let opening = market.commodity_quote(MarketSite::Avenor, CommodityId::Food);
    let approach = pose(Vec3::new(0.0, 0.0, 16000.0), core::f64::consts::PI);
    for tick in 1..=900 {
        (traffic, market) =
            ok(traffic.prepare_step(context(&data, tick, approach, approach), &market));
    }
    assert!(traffic.enabled());
    assert_eq!(traffic.clocks[0], 900);
    assert!(traffic.actors[0].is_none());
    assert_eq!(
        market.commodity_quote(MarketSite::Avenor, CommodityId::Food),
        opening
    );
    let outside = pose(Vec3::new(-600.0, 0.0, 16000.0), core::f64::consts::PI);
    (traffic, market) = ok(traffic.prepare_step(context(&data, 901, approach, outside), &market));
    assert!(traffic.actors[0].is_none());
    (traffic, market) = ok(traffic.prepare_step(context(&data, 902, outside, outside), &market));
    let trader = some(traffic.actors[0]);
    assert_eq!(trader.born_world_tick, 902);
    assert_eq!(
        (trader.manifest, trader.units, trader.wallet),
        (CommodityId::Food, 9, CAPITAL - 9 * 226)
    );
    assert_eq!(
        market
            .commodity_quote(MarketSite::Avenor, CommodityId::Food)
            .stock,
        opening.stock - 9
    );
    ok(traffic.validate(12005, 902, 0, &data, &market));
}
#[test]
fn approach_reservation_does_not_disable_real_off_axis_pirate_admission_or_local_clock() {
    let data = ok(OriginalStationData::bundled());
    let mut traffic = State::fresh(12001);
    let mut market = ok(StationMarkets::new_commander(12001));
    let approach = pose(Vec3::new(0.0, 0.0, 16000.0), core::f64::consts::PI);
    for tick in 1..=900 {
        (traffic, market) =
            ok(traffic.prepare_step(context(&data, tick, approach, approach), &market));
    }
    assert!(traffic.actors[0].is_none());
    let pirate = some(traffic.actors[1]);
    assert_eq!(pirate.snapshot().hull_id, "SH025");
    assert_eq!(pirate.snapshot().population, Population::Pirate);
    assert!(pirate.airborne());
    assert!(pirate.position.x.abs() > 1200.0);
    assert_eq!(traffic.clocks[0], 900);
    ok(traffic.validate(12001, 900, 0, &data, &market));
}
#[test]
fn preexisting_trader_physically_yields_before_unmodified_continuous_player_sweep() {
    let data = ok(OriginalStationData::bundled());
    let mut traffic = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    let outside = pose(Vec3::new(-600.0, 0.0, 1200.0), 0.0);
    for tick in 1..=480 {
        (traffic, market) =
            ok(traffic.prepare_step(context(&data, tick, outside, outside), &market));
    }
    let original = some(traffic.actors[0]);
    let radius = data.player_clearance_radius();
    let mut player = pose(Vec3::new(0.0, 0.0, 16000.0), core::f64::consts::PI);
    let mut last = 480;
    for tick in 481..=1744 {
        let next = pose(
            add(player.position, Vec3::new(0.0, 0.0, -34.3 * 18.0 / 60.0)),
            core::f64::consts::PI,
        );
        assert!(traffic.player_motion_clear(
            SystemId::AVENOR,
            player.position,
            next.position,
            radius
        ));
        let before = some(traffic.actors[0]);
        (traffic, market) = ok(traffic.prepare_step(context(&data, tick, player, next), &market));
        let after = some(traffic.actors[0]);
        assert_eq!(after.id, original.id);
        assert!(length(sub(after.position, before.position)) <= 34.3 * 18.0 / 60.0 + 1e-8);
        assert!(
            (after.yaw - before.yaw)
                .sin()
                .atan2((after.yaw - before.yaw).cos())
                .abs()
                <= 0.65 / 60.0 + 1e-8
        );
        assert!(!after.sold);
        player = next;
        last = tick;
    }
    let yielded = some(traffic.actors[0]);
    assert!(yielded.position.x.abs() > radius * 2.0 + 64.0);
    // Yielding never weakens the original enclosing-hull collision predicate.
    assert!(!traffic.player_motion_clear(
        SystemId::AVENOR,
        sub(yielded.position, Vec3::new(0.0, 0.0, 1000.0)),
        add(yielded.position, Vec3::new(0.0, 0.0, 1000.0)),
        radius
    ));
    ok(traffic.validate(12005, last, 0, &data, &market));
}
