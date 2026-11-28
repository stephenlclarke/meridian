//! Exact original SH025 cargo utility and three original main-station offers; no ownership authority.
use super::{EXPANDED_HOLD_TONNES, Error, STANDARD_HOLD_TONNES};
use crate::save::Reader;
use crate::station_services::MarketSite;
pub(crate) const CONTENT: &[u8] =
    include_bytes!("../../../../assets/legacy/sh025-cargo-bay.equipment");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Definition {
    pub id: &'static str,
    pub name: &'static str,
    pub family: &'static str,
    pub slot: &'static str,
    pub hull: &'static str,
    pub price_tenths: u32,
    pub required_technology_level: u8,
    pub base_hold_tonnes: u16,
    pub extra_hold_tonnes: u16,
    pub utility_mounts_capacity: u16,
    pub equipment_units_capacity: u16,
    pub item_utility_mount_cost: u8,
    pub item_equipment_unit_cost: u8,
    pub stations: [StationOffer; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationOffer {
    pub site: MarketSite,
    pub technology: u8,
    pub required_technology_level: u8,
    pub price_tenths: u32,
    pub purchase_available: bool,
}
impl Definition {
    pub(crate) fn for_player(mut self, medusa: bool) -> Self {
        if medusa {
            self.hull = "SH171";
            self.base_hold_tonnes = 22;
            self.extra_hold_tonnes = 10;
            self.utility_mounts_capacity = 4;
            self.equipment_units_capacity = 13;
        }
        self
    }

    pub fn offer(self, site: MarketSite) -> StationOffer {
        self.stations[match site {
            MarketSite::Avenor => 0,
            MarketSite::Renas => 1,
            MarketSite::Lareria => 2,
        }]
    }
}
pub fn definition() -> Result<Definition, Error> {
    decode(CONTENT).map_err(|_| Error::InvalidContent)
}
fn text(r: &mut Reader<'_>, expected: &str) -> Result<(), crate::save::SaveError> {
    let invalid = crate::save::SaveError::ContentMismatch;
    if usize::from(r.u8()?) != expected.len() {
        return Err(invalid);
    }
    for byte in expected.bytes() {
        if r.u8()? != byte {
            return Err(invalid);
        }
    }
    Ok(())
}
fn decode(bytes: &[u8]) -> Result<Definition, crate::save::SaveError> {
    let invalid = crate::save::SaveError::ContentMismatch;
    let mut r = Reader::new(bytes);
    if r.fixed::<8>()? != *b"MEQP0001" {
        return Err(invalid);
    }
    let price_tenths = r.u32()?;
    let required_technology_level = r.u8()?;
    let base_hold_tonnes = r.u16()?;
    let extra_hold_tonnes = r.u16()?;
    let utility_mounts_capacity = u16::from(r.u8()?);
    let equipment_units_capacity = r.u16()?;
    let item_utility_mount_cost = r.u8()?;
    let item_equipment_unit_cost = r.u8()?;
    // This version is the verified one-item/one-hull subset, not a user-authored admission catalogue.
    if (
        price_tenths,
        required_technology_level,
        base_hold_tonnes,
        extra_hold_tonnes,
        utility_mounts_capacity,
        equipment_units_capacity,
        item_utility_mount_cost,
        item_equipment_unit_cost,
    ) != (
        28000,
        4,
        STANDARD_HOLD_TONNES,
        EXPANDED_HOLD_TONNES - STANDARD_HOLD_TONNES,
        3,
        24,
        1,
        1,
    ) {
        return Err(invalid);
    }
    for expected in [
        "cargo-bay",
        "Large Cargo Bay",
        "utility-cargoExpansion",
        "utility.cargoExpansion",
        "SH025",
    ] {
        text(&mut r, expected)?;
    }
    if r.u8()? != 3 {
        return Err(invalid);
    }
    let mut stations = Vec::with_capacity(3);
    for (site, technology) in [
        (MarketSite::Avenor, 12),
        (MarketSite::Renas, 13),
        (MarketSite::Lareria, 14),
    ] {
        if r.u16()? != site.system() || r.u8()? != technology {
            return Err(invalid);
        }
        stations.push(StationOffer {
            site,
            technology,
            required_technology_level,
            price_tenths,
            purchase_available: technology >= required_technology_level,
        });
    }
    if !r.finished() {
        return Err(invalid);
    }
    Ok(Definition {
        id: "cargo-bay",
        name: "Large Cargo Bay",
        family: "utility-cargoExpansion",
        slot: "utility.cargoExpansion",
        hull: "SH025",
        price_tenths,
        required_technology_level,
        base_hold_tonnes,
        extra_hold_tonnes,
        utility_mounts_capacity,
        equipment_units_capacity,
        item_utility_mount_cost,
        item_equipment_unit_cost,
        stations: stations.try_into().map_err(|_| invalid)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_original_cargo_utility_and_station_technology_contract_rejects_changed_content() {
        let actual = definition().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            (
                actual.price_tenths,
                actual.required_technology_level,
                actual.base_hold_tonnes,
                actual.extra_hold_tonnes
            ),
            (28000, 4, 92, 35)
        );
        assert_eq!(
            (
                actual.utility_mounts_capacity,
                actual.equipment_units_capacity
            ),
            (3, 24)
        );
        assert_eq!(actual.offer(MarketSite::Avenor).technology, 12);
        assert_eq!(actual.offer(MarketSite::Renas).technology, 13);
        for length in [0, 7, 21, CONTENT.len() - 1] {
            assert!(decode(&CONTENT[..length]).is_err());
        }
        for offset in [0, 8, 12, 13, 15, 17, 18, 20, 21, 23, CONTENT.len() - 1] {
            let mut changed = CONTENT.to_vec();
            changed[offset] ^= 1;
            assert!(decode(&changed).is_err());
        }
        let mut extra = CONTENT.to_vec();
        extra.push(0);
        assert!(decode(&extra).is_err());
    }
}
