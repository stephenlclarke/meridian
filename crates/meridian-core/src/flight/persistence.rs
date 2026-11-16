use super::*;
use crate::freight::g02::SavedDomainSummary;
use crate::save::{MAX_SAVE_BYTES, Reader, SAVE_VERSION, SaveError, Writer, checksum};
use crate::station_services::{SavedServiceSummary, StationMarkets};

const MAGIC: &[u8; 8] = b"MERSAVE1";
// Rules and immutable content form the version-four compatibility boundary, not an authenticity key.
const CONTENT: &[u8] = concat!(
    "meridian-world/rules/4;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

fn compiled_content_fingerprint() -> u64 {
    let mut bytes = CONTENT.to_vec();
    for part in [
        include_bytes!("../../../../assets/legacy/sh028.physics").as_slice(),
        include_bytes!("../../../../assets/legacy/sh025.physics").as_slice(),
        include_bytes!("../../../../assets/legacy/sh028.port").as_slice(),
        include_bytes!("../../../../assets/legacy/g01.navigation").as_slice(),
        include_bytes!("../../../../assets/legacy/g01-food.markets").as_slice(),
        include_bytes!("../../../../assets/legacy/commodities.catalogue").as_slice(),
        include_bytes!("../../../../assets/legacy/g01-commodities.markets").as_slice(),
        include_bytes!("../../../../assets/legacy/sh025-cargo-bay.equipment").as_slice(),
    ] {
        bytes.extend((part.len() as u64).to_le_bytes());
        bytes.extend(part);
    }
    checksum(&bytes)
}

impl World {
    /// Return a complete checked memory save. The host must atomically publish these bytes itself.
    pub fn save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        self.validate_saved_world()?;
        let mut w = Writer::new();
        w.bytes.extend(MAGIC);
        w.u16(SAVE_VERSION);
        w.bytes.extend(CONTENT);
        w.u64(compiled_content_fingerprint());
        write_flight(&mut w, &self.state);
        self.navigation
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        self.commander_account
            .as_ref()
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        self.freight_session
            .as_ref()
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        self.station_markets
            .as_ref()
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        self.pulse.encode_save(&mut w);
        let digest = checksum(&w.bytes);
        w.u64(digest);
        if w.bytes.len() > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(w.bytes)
    }

    /// Decode and validate a whole candidate without touching another World or repairing data.
    pub fn from_save_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        let mut candidate = Self::decode_saved_world(bytes)?;
        candidate
            .freight_session
            .as_mut()
            .ok_or(SaveError::NoCommander)?
            .invalidate_after_continue((0, 0))?;
        Ok(candidate)
    }

    /// Replace this World only after complete candidate validation and UI sequence invalidation.
    pub fn restore_bytes(&mut self, bytes: &[u8]) -> Result<(), SaveError> {
        let mut candidate = Self::decode_saved_world(bytes)?;
        let floor = self
            .freight_session
            .as_ref()
            .map(mission::FreightSession::continue_floor)
            .unwrap_or((0, 0));
        candidate
            .freight_session
            .as_mut()
            .ok_or(SaveError::NoCommander)?
            .invalidate_after_continue(floor)?;
        *self = candidate;
        Ok(())
    }

    fn decode_saved_world(bytes: &[u8]) -> Result<Self, SaveError> {
        if bytes.len() > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        let end = bytes.len().checked_sub(8).ok_or(SaveError::Truncated)?;
        let (body, tail) = bytes.split_at(end);
        let stored = Reader::new(tail).u64()?;
        if checksum(body) != stored {
            return Err(SaveError::ChecksumMismatch);
        }
        let mut r = Reader::new(body);
        if &r.fixed::<8>()? != MAGIC {
            return Err(SaveError::InvalidEncoding);
        }
        if r.u16()? != SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion);
        }
        if r.fixed::<{ CONTENT.len() }>()?.as_slice() != CONTENT {
            return Err(SaveError::ContentMismatch);
        }
        if r.u64()? != compiled_content_fingerprint() {
            return Err(SaveError::ContentMismatch);
        }
        let station_data =
            OriginalStationData::bundled().map_err(|_| SaveError::ContentMismatch)?;
        let mut state = read_flight(&mut r)?;
        state.station = Some(station_data.station_pose());
        let candidate = Self {
            state,
            station_data: Some(station_data),
            navigation_catalogue: Some(
                OriginalNavigation::bundled().map_err(|_| SaveError::ContentMismatch)?,
            ),
            navigation: Some(NavigationState::decode_save(&mut r)?),
            commander_account: Some(CommanderAccount::decode_save(&mut r)?),
            freight_session: Some(mission::FreightSession::decode_save(&mut r)?),
            station_markets: Some(StationMarkets::decode_save(&mut r)?),
            pulse: weapons::PulseResources::decode_save(&mut r)?,
        };
        if !r.finished() {
            return Err(SaveError::InvalidEncoding);
        }
        candidate.validate_saved_world()?;
        Ok(candidate)
    }

    fn validate_saved_world(&self) -> Result<(), SaveError> {
        let navigation = self.navigation.ok_or(SaveError::NoCommander)?;
        let account = self
            .commander_account
            .as_ref()
            .ok_or(SaveError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(SaveError::NoCommander)?;
        let markets = self
            .station_markets
            .as_ref()
            .ok_or(SaveError::NoCommander)?;
        let catalogue = self
            .navigation_catalogue
            .as_ref()
            .ok_or(SaveError::NoCommander)?;
        let station = self.station_data.as_ref().ok_or(SaveError::NoCommander)?;
        validate_flight(&self.state)?;
        if self.state.station != Some(station.station_pose()) || self.state.navigation.is_some() {
            return Err(SaveError::InvalidState(
                "immutable station or derived navigation",
            ));
        }
        let nav = navigation.validate_save(
            catalogue,
            self.state.mode,
            self.state.tick,
            account.fuel_tenths(),
        )?;
        self.pulse
            .validate_save(self.state.tick, self.state.mode, nav.launches)?;
        session.validate_save(navigation.dock())?;
        let domain = account.validate_saved_domain(&session.state, nav, session.epoch())?;
        let services = markets.validate_save(
            nav.owner,
            account.hold_capacity(),
            account.equipment_state().historical_capacity(),
        )?;
        validate_accounting(domain, services, nav.jumps, nav.fuel_spent)?;
        if navigation
            .dock()
            .is_some_and(|dock| dock.origin() == crate::navigation::DockOrigin::InitialStart)
            && (self.state.position != [0.0; 3] || self.state.orientation != [0.0, 0.0, 0.0, 1.0])
        {
            return Err(SaveError::InvalidState("initial dock pose"));
        }
        if self.state.mode == FlightMode::Docked && self.state.speed != 0.0 {
            return Err(SaveError::InvalidState("docked speed"));
        }
        Ok(())
    }
}
fn validate_accounting(
    d: SavedDomainSummary,
    s: SavedServiceSummary,
    jumps: u64,
    fuel_spent: u64,
) -> Result<(), SaveError> {
    let income = 7200_u128
        + u128::from(d.completed) * u128::from(crate::freight::REWARD_TENTHS)
        + u128::from(s.credited);
    let fuel = 130_u128 + u128::from(s.refilled_units);
    let available_gross = 7200_u128
        + u128::from(d.completed) * u128::from(crate::freight::REWARD_TENTHS)
        + s.gross_credit;
    if s.gross_credit != u128::from(s.credited) && available_gross <= u128::from(u32::MAX) {
        return Err(SaveError::InvalidState(
            "unjustified market credit saturation",
        ));
    }
    let paid = u128::from(s.paid) + u128::from(d.equipment_paid);
    if income < paid
        || income - paid != u128::from(d.credits)
        || fuel < u128::from(fuel_spent)
        || fuel - u128::from(fuel_spent) != u128::from(d.fuel)
        || u64::from(d.food) != s.food
        || d.other_commodities != s.other_commodities
        || d.legal & !s.possible_legal_bits != 0
        || (d.legal != 0 && d.launches == 0)
        || u128::from(d.revision)
            != u128::from(d.freight_revision)
                + u128::from(s.revision)
                + u128::from(jumps)
                + u128::from(d.equipment_revision)
    {
        return Err(SaveError::InvalidState(
            "wallet, cargo, fuel or payment chronology",
        ));
    }
    Ok(())
}
fn write_flight(w: &mut Writer, s: &Snapshot) {
    for p in s.position {
        w.f32(p);
    }
    for q in s.orientation {
        w.f32(q);
    }
    w.f32(s.speed);
    w.u64(s.tick);
    w.u8(match s.mode {
        FlightMode::Docked => 0,
        FlightMode::Launching => 1,
        FlightMode::Flight => 2,
    });
    w.u8(match s.motion_refusal {
        None => 0,
        Some(MotionRefusal::StationContact) => 1,
        Some(MotionRefusal::EntryConditions) => 2,
    });
}
fn read_flight(r: &mut Reader<'_>) -> Result<Snapshot, SaveError> {
    let position = r.array(|r| r.f32())?;
    let orientation = r.array(|r| r.f32())?;
    let speed = r.f32()?;
    let tick = r.u64()?;
    let mode = match r.u8()? {
        0 => FlightMode::Docked,
        1 => FlightMode::Launching,
        2 => FlightMode::Flight,
        _ => return Err(SaveError::InvalidEncoding),
    };
    let motion_refusal = match r.u8()? {
        0 => None,
        1 => Some(MotionRefusal::StationContact),
        2 => Some(MotionRefusal::EntryConditions),
        _ => return Err(SaveError::InvalidEncoding),
    };
    Ok(Snapshot {
        position,
        orientation,
        speed,
        tick,
        mode,
        station: None,
        navigation: None,
        motion_refusal,
    })
}
fn validate_flight(s: &Snapshot) -> Result<(), SaveError> {
    let norm = s.orientation.iter().map(|v| v * v).sum::<f32>();
    let travel_bound = 16000.0
        + s.tick as f64
            * f64::from(MAX_SPEED * DISTANCE_PER_SPEED_UNIT * FIXED_DELTA_SECONDS)
            * 1.1;
    if !s.speed.is_finite()
        || !(0.0..=MAX_SPEED).contains(&s.speed)
        || s.position
            .iter()
            .any(|v| !v.is_finite() || f64::from(v.abs()) > travel_bound)
        || !norm.is_finite()
        || (norm - 1.0).abs() > 1e-5
    {
        return Err(SaveError::InvalidState("flight pose/speed"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
