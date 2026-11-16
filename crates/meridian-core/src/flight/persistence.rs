use super::*;
use crate::freight::g02::SavedDomainSummary;
use crate::save::{MAX_SAVE_BYTES, Reader, SAVE_VERSION, SaveError, Writer, checksum};
use crate::station_services::{SavedServiceSummary, StationMarkets};

const MAGIC: &[u8; 8] = b"MERSAVE1";
// Rules and immutable content form the version-four compatibility boundary, not an authenticity key.
const V4_CONTENT: &[u8] = concat!(
    "meridian-world/rules/4;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

// Rules and immutable content form the version-five compatibility boundary, not an authenticity key.
const V5_CONTENT: &[u8] = concat!(
    "meridian-world/rules/5;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

// Rules and immutable content form the version-six compatibility boundary, not an authenticity key.
const CONTENT: &[u8] = concat!(
    "meridian-world/rules/6;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

fn compiled_content_fingerprint(content: &[u8]) -> u64 {
    let mut bytes = content.to_vec();
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
        self.save_bytes_with_practice(self.practice)
    }

    pub(super) fn save_bytes_with_practice(
        &self,
        practice: Option<practice::Practice>,
    ) -> Result<Vec<u8>, SaveError> {
        self.validate_saved_world(practice)?;
        let mut w = Writer::new();
        w.bytes.extend(MAGIC);
        w.u16(SAVE_VERSION);
        w.bytes.extend(CONTENT);
        w.u64(compiled_content_fingerprint(CONTENT));
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
        w.option(practice.as_ref(), |w, practice| {
            practice.encode_reception(w)
        });
        w.option(practice.as_ref(), |w, practice| practice.encode(w));
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
        let version = r.u16()?;
        let content = match version {
            4 if r.fixed::<{ V4_CONTENT.len() }>()?.as_slice() == V4_CONTENT => V4_CONTENT,
            5 if r.fixed::<{ V5_CONTENT.len() }>()?.as_slice() == V5_CONTENT => V5_CONTENT,
            SAVE_VERSION if r.fixed::<{ CONTENT.len() }>()?.as_slice() == CONTENT => CONTENT,
            4 | 5 | SAVE_VERSION => return Err(SaveError::ContentMismatch),
            _ => return Err(SaveError::UnsupportedVersion),
        };
        if r.u64()? != compiled_content_fingerprint(content) {
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
            practice: read_practice(&mut r, version)?,
            pulse: weapons::PulseResources::decode_save(&mut r)?,
        };
        if !r.finished() {
            return Err(SaveError::InvalidEncoding);
        }
        candidate.validate_saved_world(candidate.practice)?;
        Ok(candidate)
    }

    fn validate_saved_world(&self, practice: Option<practice::Practice>) -> Result<(), SaveError> {
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
        if let Some(practice) = practice {
            practice.validate(nav, navigation, self.state, station, self.pulse)?;
        }
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
fn read_practice(
    r: &mut Reader<'_>,
    version: u16,
) -> Result<Option<practice::Practice>, SaveError> {
    if version == 4 {
        return Ok(None);
    }
    let reception = if version == SAVE_VERSION {
        r.option(practice::reception::Reception::decode)?
    } else {
        None
    };
    let mut practice = r.option(practice::Practice::decode)?;
    if version == SAVE_VERSION {
        match (practice.as_mut(), reception) {
            (Some(practice), Some(reception)) => practice.reception = reception,
            (None, None) => {}
            _ => return Err(SaveError::InvalidEncoding),
        }
    }
    Ok(practice)
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
pub(super) fn v4_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    let current = world.save_bytes()?;
    assert!(world.practice.is_none());
    let prefix = 18 + CONTENT.len();
    // Snapshot through market bytes are unchanged. Remove both absent v6
    // practice tags immediately before the five old weapon bytes.
    let tail = current.len() - 13;
    assert_eq!(current[tail - 1], 0);
    assert_eq!(current[tail - 2], 0);
    let mut w = Writer::new();
    w.bytes.extend(MAGIC);
    w.u16(4);
    w.bytes.extend(V4_CONTENT);
    // Independently calculated from the exact predecessor rules header and
    // original eight compiled resources.
    w.u64(0xaa91_4776_a63c_5f97);
    w.bytes.extend(&current[prefix..tail - 2]);
    w.bytes.extend(&current[tail..tail + 5]);
    let digest = checksum(&w.bytes);
    w.u64(digest);
    Ok(w.bytes)
}

#[cfg(test)]
pub(super) fn v5_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    let current = world.save_bytes()?;
    let mut legacy = Writer::new();
    legacy.option(world.practice.as_ref(), |w, practice| practice.encode(w));
    let mut reception = Writer::new();
    reception.option(world.practice.as_ref(), |w, practice| {
        practice.encode_reception(w)
    });
    let start = 18 + CONTENT.len();
    let reception_end = current.len() - 13 - legacy.bytes.len();
    let reception_start = reception_end - reception.bytes.len();
    assert_eq!(
        &current[reception_start..reception_end],
        reception.bytes.as_slice()
    );
    let mut w = Writer::new();
    w.bytes.extend(MAGIC);
    w.u16(5);
    w.bytes.extend(V5_CONTENT);
    // Independently calculated from the exact v5 base header and assets.
    w.u64(0x46a9_5792_16cc_8694);
    w.bytes.extend(&current[start..reception_start]);
    w.bytes.extend(&current[reception_end..current.len() - 8]);
    let digest = checksum(&w.bytes);
    w.u64(digest);
    Ok(w.bytes)
}

#[cfg(test)]
mod tests;
