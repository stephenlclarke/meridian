use super::{
    CommanderAccount, DISTANCE_PER_SPEED_UNIT, FIXED_DELTA_SECONDS, FlightMode, MAX_SPEED,
    MotionRefusal, NavigationState, OriginalNavigation, OriginalStationData, Snapshot, World,
    combat, flight_reception, missiles, mission, navigation_trade, practice, weapons,
};
#[cfg(test)]
use super::{
    FlightInput, MissionError, MissionSnapshot, ServiceSnapshot, StationNpc, StepError, SystemId,
};
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
const V6_CONTENT: &[u8] = concat!(
    "meridian-world/rules/6;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const V7_CONTENT: &[u8] = concat!(
    "meridian-world/rules/7;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const V8_CONTENT: &[u8] = concat!(
    "meridian-world/rules/8;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const V9_CONTENT: &[u8] = concat!(
    "meridian-world/rules/9;practice-attempts/a11fe169;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const V10_CONTENT: &[u8] = concat!(
    "meridian-world/rules/10;t01/native-main-berth-flight-v1/a11fe169;practice-attempts/a11fe169;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const V11_CONTENT: &[u8] = concat!(
    "meridian-world/rules/11;t04/8c23ce72936e8bdfa47978f7d7d4240171babe05c2da5ac49727d2616e38d802/a11fe169/fuel-chain-v1;t01/native-main-berth-flight-v1/a11fe169;practice-attempts/a11fe169;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const V13_CONTENT: &[u8] = concat!(
    "meridian-world/rules/13;modular-mining/front-paid-v1/a11fe169;paid-standard-missile/pylon-nine-retained-v1/a11fe169;t04/8c23ce72936e8bdfa47978f7d7d4240171babe05c2da5ac49727d2616e38d802/a11fe169/fuel-chain-v1;t01/native-main-berth-flight-v1/a11fe169;practice-attempts/a11fe169;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "fb9dea9e2c03f4cdcd74ed1e0409706bf2cdd0ed9fb37ca2c56934005deb72fa"
)
.as_bytes();

const CONTENT: &[u8] = concat!(
    "meridian-world/rules/15;npc-missiles/native-surface-guidance-v2;traffic/local-paid-sh025-v2;npc-player/exact-pools-journal-v1;modular-mining/front-paid-v1/a11fe169;paid-standard-missile/pylon-nine-retained-v1/a11fe169;t04/8c23ce72936e8bdfa47978f7d7d4240171babe05c2da5ac49727d2616e38d802/a11fe169/fuel-chain-v1;t01/native-main-berth-flight-v1/a11fe169;practice-attempts/a11fe169;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
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
    if content == CONTENT {
        bytes.extend(crate::traffic::CONTENT_RULES);
    }
    checksum(&bytes)
}

/// Immutable candidate references for a joint ordinary commerce/lesson save.
/// World remains the sole live owner; these parts have no public constructor.
pub(super) struct Commerce<'a> {
    pub(super) account: Option<&'a CommanderAccount>,
    pub(super) markets: Option<&'a StationMarkets>,
    pub(super) lesson: &'a navigation_trade::State,
    pub(super) ammunition: &'a super::ammunition::State,
    pub(super) modular: &'a super::modular::State,
}
impl<'a> Commerce<'a> {
    fn current(world: &'a World) -> Self {
        Self {
            account: world.commander_account.as_ref(),
            markets: world.station_markets.as_ref(),
            lesson: &world.navigation_trade,
            ammunition: &world.ammunition,
            modular: &world.modular,
        }
    }
}
impl World {
    pub(super) fn encoded_current_size(&self) -> Result<usize, SaveError> {
        self.encode_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &Commerce::current(self),
        )
        .map(|b| b.len())
    }
    /// Return a complete checked memory save. The host must atomically publish these bytes itself.
    pub fn save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        self.save_bytes_with_practice(self.practice)
    }

    pub(super) fn save_bytes_with_practice(
        &self,
        practice: Option<practice::Practice>,
    ) -> Result<Vec<u8>, SaveError> {
        self.save_bytes_with_practice_history(practice, &self.practice_history, self.practice_trace)
    }
    pub(super) fn save_bytes_with_practice_history(
        &self,
        practice: Option<practice::Practice>,
        history: &[practice::Practice],
        trace: Option<practice::PracticeAttemptId>,
    ) -> Result<Vec<u8>, SaveError> {
        self.validate_saved_world(
            practice,
            history,
            trace,
            &self.flight_reception,
            &Commerce::current(self),
        )?;
        self.encode_saved_world(
            practice,
            history,
            trace,
            &self.flight_reception,
            &Commerce::current(self),
        )
    }
    /// Bind all checked saved state except the current simulation tick. A docked redraw
    /// advances only that clock; historical permission/reading/dock/terminal clocks stay bound.
    /// The normalized bytes are fingerprint input only, never a save or a mutated World.
    pub(super) fn flight_reception_material_fingerprint(&self) -> Result<u64, SaveError> {
        let mut bytes = self.save_bytes()?;
        let flight_start = MAGIC.len() + size_of::<u16>() + CONTENT.len() + size_of::<u64>();
        let mut material = self.state;
        material.tick = 0;
        let mut encoded = Writer::new();
        write_flight(&mut encoded, &material);
        bytes[flight_start..flight_start + encoded.bytes.len()].copy_from_slice(&encoded.bytes);
        // The stored checksum also derives from the ordinary tick; hash its checked body.
        Ok(checksum(&bytes[..bytes.len() - size_of::<u64>()]))
    }
    pub(super) fn save_bytes_with_flight_reception(
        &self,
        lesson: &flight_reception::State,
    ) -> Result<Vec<u8>, SaveError> {
        self.validate_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            lesson,
            &Commerce::current(self),
        )?;
        self.encode_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            lesson,
            &Commerce::current(self),
        )
    }
    pub(super) fn save_bytes_with_navigation_trade(
        &self,
        lesson: &navigation_trade::State,
        account: &CommanderAccount,
        markets: &StationMarkets,
    ) -> Result<Vec<u8>, SaveError> {
        let commerce = Commerce {
            account: Some(account),
            markets: Some(markets),
            lesson,
            ammunition: &self.ammunition,
            modular: &self.modular,
        };
        self.validate_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &commerce,
        )?;
        self.encode_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &commerce,
        )
    }
    pub(super) fn save_bytes_with_ammunition(
        &self,
        account: &CommanderAccount,
        ammunition: &super::ammunition::State,
    ) -> Result<Vec<u8>, SaveError> {
        let commerce = Commerce {
            account: Some(account),
            markets: self.station_markets.as_ref(),
            lesson: &self.navigation_trade,
            ammunition,
            modular: &self.modular,
        };
        self.validate_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &commerce,
        )?;
        self.encode_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &commerce,
        )
    }
    pub(super) fn save_bytes_with_modules(
        &self,
        account: &CommanderAccount,
        modular: &super::modular::State,
    ) -> Result<Vec<u8>, SaveError> {
        let commerce = Commerce {
            account: Some(account),
            markets: self.station_markets.as_ref(),
            lesson: &self.navigation_trade,
            ammunition: &self.ammunition,
            modular,
        };
        self.validate_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &commerce,
        )?;
        self.encode_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &commerce,
        )
    }
    // Encoding is separate so negative tests can exercise the loader with coherent checksums.
    fn encode_saved_world(
        &self,
        practice: Option<practice::Practice>,
        history: &[practice::Practice],
        trace: Option<practice::PracticeAttemptId>,
        lesson: &flight_reception::State,
        commerce: &Commerce<'_>,
    ) -> Result<Vec<u8>, SaveError> {
        let mut w = Writer::new();
        w.bytes.extend(MAGIC);
        w.u16(SAVE_VERSION);
        w.bytes.extend(CONTENT);
        w.u64(compiled_content_fingerprint(CONTENT));
        write_flight(&mut w, &self.state);
        self.navigation
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        commerce
            .account
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        self.freight_session
            .as_ref()
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        commerce
            .markets
            .ok_or(SaveError::NoCommander)?
            .encode_save(&mut w);
        commerce.modular.encode(&mut w);
        commerce.ammunition.encode(&mut w);
        commerce.lesson.encode(&mut w);
        lesson.encode(&mut w);
        self.missiles.encode(&mut w);
        practice::lifecycle::encode_history(&mut w, history, trace);
        w.option(practice.as_ref(), |w, practice| {
            practice.encode_reception(w)
        });
        w.option(practice.as_ref(), |w, practice| practice.encode(w));
        self.combat.encode(&mut w);
        w.option(practice.and_then(|p| p.ordinary_hit).as_ref(), |w, hit| {
            hit.encode(w)
        });
        self.pulse.encode_save(&mut w);
        self.traffic.encode(&mut w);
        self.damage.encode(&mut w);
        let digest = checksum(&w.bytes);
        w.u64(digest);
        if w.bytes.len() > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(w.bytes)
    }

    #[cfg(test)]
    pub(super) fn unchecked_test_save(&self) -> Result<Vec<u8>, SaveError> {
        self.encode_saved_world(
            self.practice,
            &self.practice_history,
            self.practice_trace,
            &self.flight_reception,
            &Commerce::current(self),
        )
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

    pub(super) fn decode_saved_world(bytes: &[u8]) -> Result<Self, SaveError> {
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
            6 if r.fixed::<{ V6_CONTENT.len() }>()?.as_slice() == V6_CONTENT => V6_CONTENT,
            7 if r.fixed::<{ V7_CONTENT.len() }>()?.as_slice() == V7_CONTENT => V7_CONTENT,
            8 if r.fixed::<{ V8_CONTENT.len() }>()?.as_slice() == V8_CONTENT => V8_CONTENT,
            9 if r.fixed::<{ V9_CONTENT.len() }>()?.as_slice() == V9_CONTENT => V9_CONTENT,
            10 if r.fixed::<{ V10_CONTENT.len() }>()?.as_slice() == V10_CONTENT => V10_CONTENT,
            11 if r.fixed::<{ V11_CONTENT.len() }>()?.as_slice() == V11_CONTENT => V11_CONTENT,
            13 if r.fixed::<{ V13_CONTENT.len() }>()?.as_slice() == V13_CONTENT => V13_CONTENT,
            SAVE_VERSION if r.fixed::<{ CONTENT.len() }>()?.as_slice() == CONTENT => CONTENT,
            4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 13 | SAVE_VERSION => {
                return Err(SaveError::ContentMismatch);
            }
            _ => return Err(SaveError::UnsupportedVersion),
        };
        if r.u64()? != compiled_content_fingerprint(content) {
            return Err(SaveError::ContentMismatch);
        }
        let station_data =
            OriginalStationData::bundled().map_err(|_| SaveError::ContentMismatch)?;
        let mut state = read_flight(&mut r)?;
        state.station = Some(station_data.station_pose());
        let navigation = NavigationState::decode_save(&mut r)?;
        let commander_account = CommanderAccount::decode_save(&mut r, version)?;
        let freight_session = mission::FreightSession::decode_save(&mut r)?;
        let station_markets = StationMarkets::decode_save(&mut r)?;
        let modular = if version >= 13 {
            super::modular::State::decode(&mut r)?
        } else {
            super::modular::State::default()
        };
        let ammunition = if version >= 12 {
            super::ammunition::State::decode(&mut r)?
        } else {
            super::ammunition::State::default()
        };
        let navigation_trade = if version >= 11 {
            navigation_trade::State::decode(&mut r)?
        } else {
            navigation_trade::State::default()
        };
        let flight_reception = if version >= 10 {
            flight_reception::State::decode(&mut r, version)?
        } else {
            flight_reception::State::default()
        };
        let missiles = if version >= 8 {
            missiles::State::decode(&mut r, version)?
        } else {
            missiles::State::default()
        };
        let (practice_history, practice_trace) = if version >= 9 {
            practice::lifecycle::decode_history(&mut r)?
        } else {
            (Vec::new(), None)
        };
        let mut candidate = Self {
            state,
            traffic: crate::traffic::State::default(),
            damage: super::damage::State::default(),
            station_data: Some(station_data),
            navigation_catalogue: Some(
                OriginalNavigation::bundled().map_err(|_| SaveError::ContentMismatch)?,
            ),
            navigation: Some(navigation),
            commander_account: Some(commander_account),
            freight_session: Some(freight_session),
            station_markets: Some(station_markets),
            missiles,
            ammunition,
            modular,
            flight_reception,
            navigation_trade,
            practice_history,
            practice_trace,
            practice: read_practice(&mut r, version)?,
            pulse: weapons::PulseResources::default(),
            combat: combat::State::default(),
        };
        if version >= 7 {
            candidate.combat = combat::State::decode(&mut r, version)?;
            let ordinary_hit = r.option(|r| combat::OrdinaryImpact::decode_version(r, version))?;
            match (candidate.practice.as_mut(), ordinary_hit) {
                (Some(practice), hit) => practice.ordinary_hit = hit,
                (None, None) => {}
                _ => return Err(SaveError::InvalidEncoding),
            }
        }
        candidate.pulse = weapons::PulseResources::decode_save(&mut r)?;
        if version >= 15 {
            candidate.traffic = crate::traffic::State::decode(&mut r)?;
            candidate.damage = super::damage::State::decode(&mut r)?;
        }
        if version < 15 && candidate.state.motion_refusal == Some(MotionRefusal::NpcContact) {
            return Err(SaveError::InvalidEncoding);
        }
        if !r.finished() {
            return Err(SaveError::InvalidEncoding);
        }
        if version <= 8 {
            // Earlier supported completion already carried the trusted trace proof.
            candidate.practice_trace = candidate
                .practice
                .filter(|p| p.completed_tick().is_some())
                .map(|p| p.attempt_id());
        }
        candidate.validate_saved_world(
            candidate.practice,
            &candidate.practice_history,
            candidate.practice_trace,
            &candidate.flight_reception,
            &Commerce::current(&candidate),
        )?;
        Ok(candidate)
    }

    fn validate_saved_world(
        &self,
        practice: Option<practice::Practice>,
        history: &[practice::Practice],
        trace: Option<practice::PracticeAttemptId>,
        lesson: &flight_reception::State,
        commerce: &Commerce<'_>,
    ) -> Result<(), SaveError> {
        let navigation = self.navigation.ok_or(SaveError::NoCommander)?;
        let account = commerce.account.ok_or(SaveError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(SaveError::NoCommander)?;
        let markets = commerce.markets.ok_or(SaveError::NoCommander)?;
        let catalogue = self
            .navigation_catalogue
            .as_ref()
            .ok_or(SaveError::NoCommander)?;
        let station = self.station_data.as_ref().ok_or(SaveError::NoCommander)?;
        validate_flight(&self.state)?;
        if self.celestial_snapshot().is_some_and(|scene| {
            [scene.planet, scene.sun].into_iter().any(|body| {
                body.blocks_motion(
                    self.state.position,
                    self.state.position,
                    station.player_radius(),
                )
            })
        }) {
            return Err(SaveError::InvalidState(
                "player intersects physical celestial surface",
            ));
        }
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
        if !self.damage.enabled() {
            self.damage.validate_resources(self, nav.dock_sequence)?;
        }
        // Refuse unbound reducer damage with its established boundary before the
        // missile graph checks the numeric shield result against retained origins.
        self.combat.validate_damage_boundary(self)?;
        self.validate_practice_ledger(practice, history, trace, nav)?;
        lesson.validate(self, nav, practice, history)?;
        commerce.lesson.validate(
            self,
            commerce,
            nav,
            lesson.occupancy(),
            (
                history.len() + usize::from(practice.is_some()),
                usize::from(practice.is_some_and(|p| !p.reception_is_terminal())),
            ),
        )?;
        let modular = commerce.modular.ledger(self, account)?;
        let ammunition = commerce.ammunition.ledger_with_module_count(
            self,
            account,
            commerce.modular.paid_count(),
        )?;
        self.missiles
            .validate(self, practice, history, nav.dock_sequence)?;
        self.combat.validate(self)?;
        for attempt in practice::lifecycle::lessons(practice, history) {
            if let Some(last) = self.combat_snapshot().and_then(|s| s.last_ordinary_impact)
                && attempt.accepts_incident(last)
                && attempt.ordinary_hit.is_none()
            {
                return Err(SaveError::InvalidState("ordinary hit was removed from T02"));
            }
            if let Some(hit) = attempt.ordinary_hit {
                hit.validate(self)?;
                if self
                    .combat_snapshot()
                    .and_then(|s| s.last_ordinary_impact)
                    .is_none_or(|last| {
                        hit.incident > last.incident
                            || hit.physical_tick > last.physical_tick
                            || (hit.incident == last.incident && hit != last)
                    })
                {
                    return Err(SaveError::InvalidState(
                        "T02 ordinary hit has no accepted incident",
                    ));
                }
            }
        }
        if self.damage.enabled() {
            // The shared replay already required exact shields, energy, heat and cadence.
        } else if self
            .missiles
            .has_undocked_player_hit(practice, history, nav.dock_sequence)
        {
            self.pulse
                .validate_damaged_save(self.state.tick, self.state.mode, nav.launches)?;
        } else {
            self.pulse
                .validate_save(self.state.tick, self.state.mode, nav.launches)?;
        }
        for attempt in practice::lifecycle::lessons(practice, history) {
            attempt.validate(nav, navigation, self.state, station, self.pulse)?;
        }
        session.validate_save(navigation.dock())?;
        let domain = account.validate_saved_domain(&session.state, nav, session.epoch())?;
        if !self.damage.enabled() && !self.traffic.player_hits().is_empty() {
            return Err(SaveError::InvalidState(
                "NPC player hit has no resource birth provenance",
            ));
        }
        self.traffic.validate(
            nav.owner,
            self.state.tick,
            self.combat.emissions,
            station,
            markets,
        )?;
        self.validate_traffic_player_scenes()?;
        self.validate_npc_missile_receipts()?;
        if self.state.mode != FlightMode::Docked {
            let pose = super::flight_pose(self.state.position, self.state.orientation)
                .ok_or(SaveError::InvalidState("traffic player pose"))?;
            if !self.traffic_motion_clear(pose, pose) {
                return Err(SaveError::InvalidState(
                    "saved player hull intersects traffic",
                ));
            }
        }
        let services = if self.traffic.enabled() {
            markets.validate_save_with_traffic(
                nav.owner,
                account.hold_capacity(),
                account.equipment_state().historical_capacity(),
                self.traffic.flows(),
            )?
        } else {
            markets.validate_save(
                nav.owner,
                account.hold_capacity(),
                account.equipment_state().historical_capacity(),
            )?
        };
        validate_accounting(
            domain,
            services,
            nav.jumps,
            nav.fuel_spent,
            self.combat.penalties(),
            ammunition
                .paid
                .checked_add(modular.paid)
                .ok_or(SaveError::InvalidState("paid equipment overflow"))?,
            ammunition
                .extra_revisions
                .checked_add(modular.revision)
                .ok_or(SaveError::SequenceExhausted)?,
        )?;
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
    let reception = if version >= 6 {
        r.option(practice::reception::Reception::decode)?
    } else {
        None
    };
    let mut practice = r.option(|r| practice::Practice::decode(r, version))?;
    if version >= 6 {
        match (practice.as_mut(), reception) {
            (Some(practice), Some(reception)) => practice.reception = reception,
            (None, None) => {}
            _ => return Err(SaveError::InvalidEncoding),
        }
    }
    if version <= 8
        && let Some(p) = &mut practice
    {
        p.upgrade_legacy_terminal();
    }
    Ok(practice)
}

fn validate_accounting(
    d: SavedDomainSummary,
    s: SavedServiceSummary,
    jumps: u64,
    fuel_spent: u64,
    penalties: u8,
    ammunition_paid: u32,
    ammunition_revisions: u64,
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
    let paid = u128::from(s.paid) + u128::from(d.equipment_paid) + u128::from(ammunition_paid);
    if income < paid
        || income - paid != u128::from(d.credits)
        || fuel < u128::from(fuel_spent)
        || fuel - u128::from(fuel_spent) != u128::from(d.fuel)
        || u64::from(d.food) != s.food
        || d.other_commodities != s.other_commodities
        || !combat::legal_reachable(d.legal, s.possible_legal_bits, penalties)
        || (d.legal != 0 && d.launches == 0)
        || u128::from(d.revision)
            != u128::from(d.freight_revision)
                + u128::from(s.revision)
                + u128::from(jumps)
                + u128::from(d.equipment_revision)
                + u128::from(d.consumed_missiles)
                + u128::from(
                    ammunition_revisions
                        .checked_sub(u64::from(d.consumed_missiles))
                        .ok_or(SaveError::InvalidState("missile fit revision"))?,
                )
    {
        return Err(SaveError::InvalidState(
            "wallet, cargo, fuel or payment chronology",
        ));
    }
    Ok(())
}
pub(super) fn write_flight(w: &mut Writer, s: &Snapshot) {
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
        Some(MotionRefusal::CelestialContact) => 3,
        Some(MotionRefusal::NpcContact) => 4,
    });
}
pub(super) fn read_flight(r: &mut Reader<'_>) -> Result<Snapshot, SaveError> {
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
        3 => Some(MotionRefusal::CelestialContact),
        4 => Some(MotionRefusal::NpcContact),
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
pub(super) fn validate_flight(s: &Snapshot) -> Result<(), SaveError> {
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
fn legacy_test_fixture(world: &World, version: u16) -> Result<Vec<u8>, SaveError> {
    if !world.traffic.legacy_empty() {
        return Err(SaveError::InvalidState(
            "physical traffic requires current format",
        ));
    }
    if version < 8 && !world.missiles.save_supported() {
        return Err(SaveError::UnsupportedMissiles);
    }
    world.validate_saved_world(
        world.practice,
        &world.practice_history,
        world.practice_trace,
        &world.flight_reception,
        &Commerce::current(world),
    )?;
    if world.modular != super::modular::State::default() {
        return Err(SaveError::UnsupportedVersion);
    }
    if world.ammunition != super::ammunition::State::default() || world.missiles.used() > 3 {
        return Err(SaveError::UnsupportedMissiles);
    }
    if version < 11 && world.navigation_trade != navigation_trade::State::default() {
        return Err(SaveError::InvalidState("T04 requires v11"));
    }
    if version < 10 && world.flight_reception != flight_reception::State::default() {
        return Err(SaveError::InvalidState("T01 requires v10"));
    }
    if version < 9
        && (!world.practice_history.is_empty()
            || world
                .practice
                .is_some_and(|p| p.is_cancelled() || p.attempt_id().ordinal() != 0))
    {
        return Err(SaveError::InvalidState("attempts require v9"));
    }
    let content = match version {
        4 => V4_CONTENT,
        5 => V5_CONTENT,
        6 => V6_CONTENT,
        7 => V7_CONTENT,
        8 => V8_CONTENT,
        9 => V9_CONTENT,
        10 => V10_CONTENT,
        11 => V11_CONTENT,
        _ => return Err(SaveError::UnsupportedVersion),
    };
    let mut w = Writer::new();
    w.bytes.extend(MAGIC);
    w.u16(version);
    w.bytes.extend(content);
    w.u64(compiled_content_fingerprint(content));
    write_flight(&mut w, &world.state);
    world
        .navigation
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut w);
    world
        .commander_account
        .as_ref()
        .ok_or(SaveError::NoCommander)?
        .encode_legacy_version(&mut w, version);
    world
        .freight_session
        .as_ref()
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut w);
    world
        .station_markets
        .as_ref()
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut w);
    if version >= 11 {
        world.navigation_trade.encode(&mut w);
    }
    if version >= 10 {
        world.flight_reception.encode_version(&mut w, version);
    }
    if version >= 8 {
        world.missiles.encode_version(&mut w, version);
    }
    if version >= 9 {
        practice::lifecycle::encode_history(&mut w, &world.practice_history, world.practice_trace);
    }
    if version >= 6 {
        w.option(world.practice.as_ref(), |w, p| p.encode_reception(w));
    }
    if version >= 5 {
        w.option(world.practice.as_ref(), |w, p| {
            if version >= 9 {
                p.encode(w)
            } else if version >= 8 {
                p.encode_v8(w)
            } else {
                p.encode_v7(w)
            }
        });
    }
    if version >= 7 {
        world.combat.encode_legacy_version(&mut w, version);
        w.option(
            world.practice.and_then(|p| p.ordinary_hit).as_ref(),
            |w, h| h.encode_version(w, version),
        );
    }
    world.pulse.encode_save(&mut w);
    let digest = checksum(&w.bytes);
    w.u64(digest);
    Ok(w.bytes)
}
#[cfg(test)]
pub(super) fn v4_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    assert!(world.practice.is_none());
    legacy_test_fixture(world, 4)
}
#[cfg(test)]
pub(super) fn v5_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    legacy_test_fixture(world, 5)
}
#[cfg(test)]
pub(super) fn v6_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    assert!(
        world
            .combat_snapshot()
            .and_then(|s| s.last_ordinary_impact)
            .is_none()
    );
    legacy_test_fixture(world, 6)
}
#[cfg(test)]
pub(super) fn v7_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    legacy_test_fixture(world, 7)
}

#[cfg(test)]
pub(super) fn v8_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    legacy_test_fixture(world, 8)
}

#[cfg(test)]
pub(super) fn combat_save_len(world: &World) -> usize {
    let mut w = Writer::new();
    world.combat.encode(&mut w);
    w.option(
        world.practice.and_then(|p| p.ordinary_hit).as_ref(),
        |w, hit| hit.encode(w),
    );
    w.bytes.len()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) fn v9_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    legacy_test_fixture(world, 9)
}

#[cfg(test)]
pub(super) fn v10_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    legacy_test_fixture(world, 10)
}

#[cfg(test)]
mod paid_missile_format_tests {
    use super::*;
    #[test]
    fn explicit_qualified_v11_is_read_without_relabelling_it_as_v12() {
        let world = World::new_commander(8401).unwrap_or_else(|e| panic!("{e:?}"));
        let old = legacy_test_fixture(&world, 11).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(u16::from_le_bytes([old[8], old[9]]), 11);
        let loaded = World::from_save_bytes(&old).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(loaded.ammunition_snapshot(), world.ammunition_snapshot());
        assert_eq!(loaded.credits_tenths(), world.credits_tenths());
        let current = loaded.save_bytes().unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(u16::from_le_bytes([current[8], current[9]]), SAVE_VERSION);
        assert_ne!(old, current);
    }
}

#[cfg(test)]
pub(super) fn v13_test_fixture(world: &World) -> Result<Vec<u8>, SaveError> {
    if !world.traffic.legacy_empty() {
        return Err(SaveError::InvalidState(
            "physical traffic requires current format",
        ));
    }
    world.save_bytes()?;
    let mut old = Writer::new();
    old.bytes.extend(MAGIC);
    old.u16(13);
    old.bytes.extend(V13_CONTENT);
    old.u64(compiled_content_fingerprint(V13_CONTENT));
    write_flight(&mut old, &world.state);
    world
        .navigation
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut old);
    world
        .commander_account
        .as_ref()
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut old);
    world
        .freight_session
        .as_ref()
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut old);
    world
        .station_markets
        .as_ref()
        .ok_or(SaveError::NoCommander)?
        .encode_save(&mut old);
    world.modular.encode(&mut old);
    world.ammunition.encode(&mut old);
    world.navigation_trade.encode(&mut old);
    world.flight_reception.encode_version(&mut old, 13);
    world.missiles.encode_version(&mut old, 13);
    practice::lifecycle::encode_history(&mut old, &world.practice_history, world.practice_trace);
    old.option(world.practice.as_ref(), |w, p| p.encode_reception(w));
    old.option(world.practice.as_ref(), |w, p| p.encode(w));
    world.combat.encode(&mut old);
    old.option(
        world.practice.and_then(|p| p.ordinary_hit).as_ref(),
        |w, h| h.encode(w),
    );
    world.pulse.encode_save(&mut old);
    let digest = checksum(&old.bytes);
    old.u64(digest);
    Ok(old.bytes)
}
