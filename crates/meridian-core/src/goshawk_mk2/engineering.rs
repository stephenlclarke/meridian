//! Adopted SH009 engineering in original hull-local metres and kilograms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmptyInterfaceMass {
    pub mass_kilograms: u32,
    pub anchor_metres: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoshawkMk2Engineering {
    pub dry_mass_kilograms: u32,
    pub center_of_mass_metres: [f64; 3],
    /// About the stock centre of mass, with both axes ordered X, Y, Z.
    pub inertia_about_center_kg_m2: [[f64; 3]; 3],
    /// Point masses already included in dry mass and inertia.
    pub empty_interface: [EmptyInterfaceMass; 2],
    pub proposal_sha256: &'static str,
}

// Both drive liners are accounted for within the intrinsic drives.
pub(super) static STOCK: GoshawkMk2Engineering = GoshawkMk2Engineering {
    dry_mass_kilograms: 33_072,
    center_of_mass_metres: [
        -0.000008861534966633264,
        0.19826192100360518,
        -2.153649052231662,
    ],
    inertia_about_center_kg_m2: [
        [2647102.4435325763, 3.060870338590036, -13.994218130815712],
        [3.060870338590036, 3698817.432572852, 105297.48583464578],
        [-13.994218130815712, 105297.48583464578, 1258367.897183716],
    ],
    empty_interface: [
        EmptyInterfaceMass {
            mass_kilograms: 70,
            anchor_metres: [-4.669922193065497, -4.047705186490957, 8.568521216407577],
        },
        EmptyInterfaceMass {
            mass_kilograms: 70,
            anchor_metres: [4.669922193065497, -4.047705186490957, 8.568521216407577],
        },
    ],
    proposal_sha256: "b35cbf08f9075f1bb99b88a374e078fed3f34ad06d8463554d585452981e7ede",
};
