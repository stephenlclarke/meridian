use super::*;
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn id(system: u16) -> SystemId {
    SystemId { galaxy: 0, system }
}
fn all() -> Vec<SystemId> {
    (0..256).map(id).collect()
}
fn ids(route: &RouteAlternative) -> Vec<u16> {
    route.systems.iter().map(|id| id.system).collect()
}

#[test]
fn original_direct_route_merges_both_labels_and_summarizes_actual_arrival_exposure() {
    let nav = ok(OriginalNavigation::bundled());
    let alternatives = ok(nav.route_alternatives(id(7), id(185), &[id(185), id(7)], 9.0, 9.0));
    assert_eq!(alternatives.len(), 1);
    let route = &alternatives[0];
    assert_eq!(route.labels, [RouteKind::FewestJumps, RouteKind::Safer]);
    assert_eq!(ids(route), [7, 185]);
    assert!((route.distance_ly - 3.364975176728651).abs() < 1e-9);
    assert_eq!(route.first_leg_distance_ly, route.distance_ly);
    assert_eq!(route.arrival_hostile_fraction_sum, 0.2);
    assert!(!route.assumes_refuelling_for_later_legs);
    assert!(route.refuelling_assumption().contains("not guaranteed"));
}
#[test]
fn original_safer_path_can_trade_an_extra_hop_for_lower_actual_exposure_without_refuelling_grants()
{
    let nav = ok(OriginalNavigation::bundled());
    let alternatives = ok(nav.route_alternatives(id(7), id(3), &all(), 9.0, 9.0));
    assert_eq!(alternatives.len(), 2);
    assert_eq!(alternatives[0].labels, [RouteKind::FewestJumps]);
    assert_eq!(ids(&alternatives[0]), [7, 164, 107, 56, 138, 209, 3]);
    assert_eq!(alternatives[1].labels, [RouteKind::Safer]);
    assert_eq!(ids(&alternatives[1]), [7, 164, 146, 253, 35, 102, 85, 3]);
    assert!((alternatives[0].arrival_hostile_fraction_sum - 2.4).abs() < 1e-12);
    assert!((alternatives[1].arrival_hostile_fraction_sum - 0.9).abs() < 1e-12);
    assert!((alternatives[0].distance_ly - 46.19213094965041).abs() < 1e-10);
    assert!((alternatives[1].distance_ly - 51.44493420766694).abs() < 1e-10);
    for route in &alternatives {
        assert!(route.assumes_refuelling_for_later_legs);
        assert!(
            route
                .systems
                .windows(2)
                .all(|leg| ok(nav.distance_ly(leg[0], leg[1])) <= 9.0)
        );
    }
    // Same endpoints with no visible intermediary are genuinely disconnected.
    assert!(ok(nav.route_alternatives(id(7), id(3), &[id(7), id(3)], 9.0, 9.0)).is_empty());
}
#[test]
fn first_leg_fuel_and_later_full_tank_reach_are_distinct_and_legacy_fewest_results_remain() {
    let nav = ok(OriginalNavigation::bundled());
    assert!(ok(nav.route_alternatives(id(7), id(185), &[id(7), id(185)], 3.0, 9.0)).is_empty());
    let direct = ok(nav.route_alternatives(id(7), id(185), &[id(7), id(185)], 4.0, 0.00001));
    assert_eq!(ids(&direct[0]), [7, 185]);
    for target in [0, 3, 91, 107, 162, 185] {
        let expected = ok(nav.fewest_jumps(id(7), id(target), 4.0, 9.0));
        let actual = ok(nav.route_alternatives(id(7), id(target), &all(), 4.0, 9.0));
        assert_eq!(actual[0].systems, expected);
        assert!(actual[0].first_leg_distance_ly <= 4.0);
    }
}
#[test]
fn duplicate_visibility_is_set_like_but_hidden_unknown_cross_galaxy_and_excessive_scope_refuse() {
    let nav = ok(OriginalNavigation::bundled());
    let mut duplicate = vec![id(7); 300];
    duplicate.push(id(185));
    let expected = ok(nav.route_alternatives(id(7), id(185), &[id(7), id(185)], 9.0, 9.0));
    assert_eq!(
        ok(nav.route_alternatives(id(7), id(185), &duplicate, 9.0, 9.0)),
        expected
    );
    assert_eq!(
        nav.route_alternatives(id(7), id(185), &[id(7)], 9.0, 9.0),
        Err(RouteError::HiddenSystem(id(185)))
    );
    assert_eq!(
        nav.route_alternatives(id(7), id(256), &all(), 9.0, 9.0),
        Err(RouteError::Navigation(NavigationError::UnknownSystem))
    );
    let foreign = SystemId {
        galaxy: 1,
        system: 7,
    };
    assert_eq!(
        nav.route_alternatives(id(7), foreign, &[id(7), foreign], 9.0, 9.0),
        Err(RouteError::CrossGalaxy)
    );
    assert_eq!(
        nav.route_alternatives(id(7), id(185), &[id(7), id(185), foreign], 9.0, 9.0),
        Err(RouteError::CrossGalaxy)
    );
    let mut excessive = all();
    excessive.push(foreign);
    assert_eq!(
        nav.route_alternatives(id(7), id(185), &excessive, 9.0, 9.0),
        Err(RouteError::ExcessiveScope)
    );
}
#[test]
fn invalid_reach_refuses_and_same_origin_has_zero_distance_exposure_and_no_refuel_assumption() {
    let nav = ok(OriginalNavigation::bundled());
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for (first, full) in [(bad, 9.0), (9.0, bad)] {
            assert_eq!(
                nav.route_alternatives(id(7), id(185), &all(), first, full),
                Err(RouteError::Navigation(NavigationError::InvalidReach))
            );
        }
    }
    let same = ok(nav.route_alternatives(id(7), id(7), &[id(7)], 9.0, 9.0));
    assert_eq!(ids(&same[0]), [7]);
    assert_eq!(
        (
            same[0].distance_ly,
            same[0].first_leg_distance_ly,
            same[0].arrival_hostile_fraction_sum
        ),
        (0.0, 0.0, 0.0)
    );
    assert!(!same[0].assumes_refuelling_for_later_legs);
    assert_eq!(same[0].labels, [RouteKind::FewestJumps, RouteKind::Safer]);
}
#[test]
fn stable_search_ties_use_lower_original_system_and_strict_improvements_only() {
    let graph = vec![
        vec![None, Some(1.0), Some(1.0), None],
        vec![None, None, None, Some(1.0)],
        vec![None, None, None, Some(1.0)],
        vec![None, None, None, None],
    ];
    assert_eq!(ok(shortest_path(&graph, 0, 3)), Some(vec![0, 1, 3]));
    assert_eq!(ok(shortest_path(&graph, 3, 0)), None);
    assert_eq!(ok(shortest_path(&graph, 0, 0)), Some(vec![0]));
    // A clear independent weighted counterexample must prefer two cheap hops.
    let weighted = vec![
        vec![None, Some(0.5), Some(2.0)],
        vec![None, None, Some(0.5)],
        vec![None, None, None],
    ];
    assert_eq!(ok(shortest_path(&weighted, 0, 2)), Some(vec![0, 1, 2]));
}
#[test]
fn every_weight_is_validated_before_destination_exit_and_cost_overflow_refuses() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let graph = vec![
            vec![None, Some(1.0), None],
            vec![None, None, None],
            vec![None, Some(bad), None],
        ];
        assert_eq!(shortest_path(&graph, 0, 1), Err(RouteError::InvalidScore));
        assert_eq!(shortest_path(&graph, 0, 0), Err(RouteError::InvalidScore));
    }
    assert_eq!(
        shortest_path(&[vec![None, None]], 0, 0),
        Err(RouteError::InvalidScore)
    );
    let graph = vec![
        vec![None, Some(f64::MAX), None],
        vec![None, None, Some(f64::MAX)],
        vec![None, None, None],
    ];
    assert_eq!(shortest_path(&graph, 0, 2), Err(RouteError::InvalidScore));
}
#[test]
fn exact_original_exposure_encoding_rejects_truncation_id_relabel_and_invalid_scalars() {
    assert_eq!(ok(decode_exposure(EXPOSURE))[185], 0.2);
    assert_eq!(
        decode_exposure(&EXPOSURE[..EXPOSURE.len() - 1]),
        Err(RouteError::InvalidExposure)
    );
    let mut wrong = EXPOSURE.to_vec();
    wrong[0] ^= 1;
    assert_eq!(decode_exposure(&wrong), Err(RouteError::InvalidExposure));
    let mut wrong = EXPOSURE.to_vec();
    wrong[10..12].copy_from_slice(&1u16.to_le_bytes());
    assert_eq!(decode_exposure(&wrong), Err(RouteError::InvalidExposure));
    for bad in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        let mut wrong = EXPOSURE.to_vec();
        wrong[12..20].copy_from_slice(&bad.to_le_bytes());
        assert_eq!(decode_exposure(&wrong), Err(RouteError::InvalidExposure));
    }
}
