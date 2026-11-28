//! Source-bound SH171 empty stock engineering.
//! Prepared by tools/prepare_medusa_stock.py from the reviewed finite-equivalent model.

use crate::mule::MuleEngineering;

/// Empty dry reference for inspection and later hull admission.
pub const STOCK_EMPTY_DRY: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 40_712,
    center_of_mass_metres: [
        -0.005995982737240241,
        0.08640735998838894,
        -2.4239566464299034,
    ],
    inertia_about_center_kg_m2: [
        1385523.354783143,
        -25.12419784209598,
        517.6920558450192,
        -25.12419784209598,
        2614212.2324890774,
        -15878.051754398119,
        517.6920558450192,
        -15878.051754398119,
        2569400.213187349,
    ],
    scale_numerator: 5,
    scale_denominator: 41,
};
