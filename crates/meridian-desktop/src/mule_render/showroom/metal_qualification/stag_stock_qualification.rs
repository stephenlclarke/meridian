//! Mac-owner ordinary Stag stock presentation and exact independent stock-peer cache evidence.
use super::{
    kite_stock_qualification::qualify_stock_pair, sparrowhawk_stock_qualification::qualify_stock,
};

#[test]
#[ignore = "Mac owner only: stock Stag Liner ordinary Showroom RGB evidence"]
fn metal_stock_stag_normal_showroom() {
    qualify_stock("Stag Liner", "SH035", "MERIDIAN_SH035_STOCK_SHOWROOM_DIR");
}

#[test]
#[ignore = "Mac owner only: stock Stag/Sparrowhawk independent reference and cache evidence"]
fn metal_stock_stag_sparrowhawk_cache_comparison() {
    qualify_stock_pair(
        ("Stag Liner", "SH035", "stag"),
        ("Sparrowhawk", "SH030", "sparrowhawk"),
        "MERIDIAN_SH035_SH030_STOCK_CACHE_DIR",
    );
}

#[test]
#[ignore = "Mac owner only: stock Stag/Kite independent reference and cache evidence"]
fn metal_stock_stag_kite_cache_comparison() {
    qualify_stock_pair(
        ("Stag Liner", "SH035", "stag"),
        ("Kite Interceptor", "SH031", "kite"),
        "MERIDIAN_SH035_SH031_STOCK_CACHE_DIR",
    );
}
