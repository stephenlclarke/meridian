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
// A route is optional in v17. The exact legacy encoding is retained when no
// route exists; a distinct tag separates the additive trailer from corruption.
const ROUTE_EXTENSION: &[u8; 8] = b"MRTE0001";
const CONTENT: &[u8] = concat!(
    "meridian-world/rules/17;explicit-xyzzy/account-prefix-replay-v2/a11fe169;distinct-emp/paid-defensive-local-v1/a11fe169;navigation-computer/exact-route-journal/a11fe169;lareria-main-services/station-combat-three-v1/a11fe169;rapid-lasers-energy/shared-nine-byte-resources-v1/a11fe169;mining/native-work-pending-v2;fuel-scoops/paid-utility-v1;npc-missiles/canonical-native-ray-v3;traffic/local-paid-sh025-v2;npc-player/exact-pools-journal-v1;modular-mining/front-paid-v1/a11fe169;paid-standard-missile/pylon-nine-retained-v1/a11fe169;t04/8c23ce72936e8bdfa47978f7d7d4240171babe05c2da5ac49727d2616e38d802/a11fe169/fuel-chain-v1;t01/native-main-berth-flight-v1/a11fe169;practice-attempts/a11fe169;durable-missiles/a11fe169;ordinary-impact/a11fe169;t02/4e55aeba14211b7b064088f1248602161a53c728f5447f1bd29a3f7cec6f81d7;",
    "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572;",
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9;",
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883;",
    "e1e97b33550c426db1a7e90a4dcea6fe05dbd09268a964189793ba2bab96d823;",
    "31b8273c6e8fb8afd4f43b4ec029857445fffb722c3870703707fe6d4fc98dff"
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
        bytes.extend(crate::mining::content_bytes());
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
    pub(super) emp: &'a super::emp::State,
}
impl<'a> Commerce<'a> {
    fn current(world: &'a World) -> Self {
        Self {
            account: world.commander_account.as_ref(),
            markets: world.station_markets.as_ref(),
            lesson: &world.navigation_trade,
            ammunition: &world.ammunition,
            modular: &world.modular,
            emp: &world.emp,
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
            emp: &self.emp,
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
            emp: &self.emp,
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
    pub(super) fn save_bytes_with_emp(
        &self,
        account: &CommanderAccount,
        emp: &super::emp::State,
    ) -> Result<Vec<u8>, SaveError> {
        let commerce = Commerce {
            account: Some(account),
            markets: self.station_markets.as_ref(),
            lesson: &self.navigation_trade,
            ammunition: &self.ammunition,
            modular: &self.modular,
            emp,
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
            emp: &self.emp,
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
        self.xyzzy.encode(&mut w);
        commerce.emp.encode(&mut w);
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
        self.mining.encode(&mut w);
        self.jump_journal.encode(&mut w);
        if let Some(route) = &self.navigation_route {
            w.bytes.extend(ROUTE_EXTENSION);
            route.encode_save(&mut w);
        }
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
        if version != SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion);
        }
        if r.fixed::<{ CONTENT.len() }>()?.as_slice() != CONTENT
            || r.u64()? != compiled_content_fingerprint(CONTENT)
        {
            return Err(SaveError::ContentMismatch);
        }
        let station_data =
            OriginalStationData::bundled().map_err(|_| SaveError::ContentMismatch)?;
        let mut state = read_flight(&mut r, version)?;
        state.station = Some(station_data.station_pose());
        let navigation = NavigationState::decode_save(&mut r)?;
        let commander_account = CommanderAccount::decode_save(&mut r, version)?;
        let freight_session = mission::FreightSession::decode_save(&mut r)?;
        let station_markets = StationMarkets::decode_save(&mut r)?;
        let xyzzy = super::xyzzy::State::decode(&mut r)?;
        let emp = super::emp::State::decode(&mut r)?;
        let modular = super::modular::State::decode(&mut r, version)?;
        let ammunition = super::ammunition::State::decode(&mut r)?;
        let navigation_trade = navigation_trade::State::decode(&mut r)?;
        let flight_reception = flight_reception::State::decode(&mut r, version)?;
        let missiles = missiles::State::decode(&mut r, version)?;
        let (practice_history, practice_trace) =
            practice::lifecycle::decode_history(&mut r, version)?;
        let mut candidate = Self {
            state,
            traffic: crate::traffic::State::default(),
            targeting: super::targeting::State::default(),
            auto_docking: None,
            goshawk_validation: None,
            mule_validation: None,
            mule_renas: None,
            mining: crate::mining::State::legacy_absent(),
            mining_scope: std::sync::Arc::new(()),
            damage: super::damage::State::default(),
            station_data: Some(station_data),
            navigation_catalogue: Some(
                OriginalNavigation::bundled().map_err(|_| SaveError::ContentMismatch)?,
            ),
            navigation: Some(navigation),
            navigation_route: None,
            jump_journal: crate::navigation::JumpJournal::default(),
            commander_account: Some(commander_account),
            freight_session: Some(freight_session),
            station_markets: Some(station_markets),
            missiles,
            ammunition,
            modular,
            xyzzy,
            emp,
            flight_reception,
            navigation_trade,
            practice_history,
            practice_trace,
            practice: read_practice(&mut r, version)?,
            pulse: weapons::PulseResources::default(),
            combat: combat::State::default(),
        };
        candidate.combat = combat::State::decode(&mut r, version)?;
        let ordinary_hit = r.option(|r| combat::OrdinaryImpact::decode_version(r, version))?;
        match (candidate.practice.as_mut(), ordinary_hit) {
            (Some(practice), hit) => practice.ordinary_hit = hit,
            (None, None) => {}
            _ => return Err(SaveError::InvalidEncoding),
        }
        candidate.pulse = weapons::PulseResources::decode_save(&mut r)?;
        candidate.traffic = crate::traffic::State::decode_version(&mut r, version)?;
        candidate.damage = super::damage::State::decode(&mut r, version)?;
        candidate.mining = crate::mining::State::decode(&mut r)?;
        candidate.jump_journal = crate::navigation::JumpJournal::decode(&mut r)?;
        if !r.finished() {
            if r.fixed::<8>().map_err(|_| SaveError::InvalidEncoding)? != *ROUTE_EXTENSION {
                return Err(SaveError::InvalidEncoding);
            }
            candidate.navigation_route = Some(crate::navigation::RoutePlan::decode_save(&mut r)?);
        }
        if !r.finished() {
            return Err(SaveError::InvalidEncoding);
        }
        candidate.validate_saved_world(
            candidate.practice,
            &candidate.practice_history,
            candidate.practice_trace,
            &candidate.flight_reception,
            &Commerce::current(&candidate),
        )?;
        candidate
            .restore_renas_mule()
            .map_err(|_| SaveError::ContentMismatch)?;
        Ok(candidate)
    }

    pub(super) fn validate_practice_incidents(
        &self,
        practice: Option<practice::Practice>,
        history: &[practice::Practice],
    ) -> Result<(), SaveError> {
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
        Ok(())
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
        self.validate_saved_navigation_route()?;
        if !self.damage.enabled() {
            self.damage.validate_resources(self, nav.dock_sequence)?;
        }
        // Refuse unbound reducer damage with its established boundary before the
        // missile graph checks the numeric shield result against retained origins.
        self.combat.validate_damage_boundary(self)?;
        self.validate_practice_ledger(practice, history, trace, nav)?;
        // Preserve the established lesson-specific refusal before the newly exact
        // shared player-pool replay checks the same altered weapon resources.
        for attempt in practice::lifecycle::lessons(practice, history) {
            attempt.validate(nav, navigation, self.state, station, self)?;
        }
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
        if modular.shield_grade() != Some(self.combat.shields().grade()) {
            return Err(SaveError::InvalidState(
                "shield banks do not match the fitted module",
            ));
        }
        let fitted_drive = modular
            .drive()
            .ok_or(SaveError::InvalidState("missing fitted drive"))?;
        navigation.validate_pending_reach(catalogue, modular.jump_reach_ly(), fitted_drive)?;
        self.jump_journal.validate(
            navigation,
            catalogue,
            self.state.tick,
            self.combat.physical_tick(),
            &modular.navigation_changes,
            fitted_drive.1,
        )?;
        let ammunition = commerce.ammunition.ledger_with_module_count(
            self,
            account,
            commerce.modular.paid_count(),
        )?;
        self.missiles
            .validate(self, practice, history, nav.dock_sequence)?;
        self.combat.validate(self)?;
        if !self.damage.enabled() {
            self.validate_practice_incidents(practice, history)?;
            if self
                .missiles
                .has_undocked_player_hit(practice, history, nav.dock_sequence)
            {
                self.pulse
                    .validate_damaged_save(self.state.tick, self.state.mode, nav.launches)?;
            } else {
                self.pulse
                    .validate_save(self.state.tick, self.state.mode, nav.launches)?;
            }
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
        self.validate_mining_work(commerce.modular)?;
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
        let emp = commerce.emp.validate(self, account, commerce.modular)?;
        let secret = self.xyzzy.validate(self, account, commerce)?;
        commerce.modular.validate_test_custody_history(
            self,
            account,
            commerce.emp,
            commerce.lesson,
        )?;
        validate_accounting(
            domain,
            services,
            nav.jumps,
            nav.fuel_spent,
            self.combat.penalties(),
            (
                ammunition
                    .paid
                    .checked_add(modular.paid)
                    .and_then(|paid| paid.checked_add(emp.0))
                    .ok_or(SaveError::InvalidState("paid equipment overflow"))?,
                ammunition
                    .extra_revisions
                    .checked_add(modular.revision)
                    .and_then(|revisions| revisions.checked_add(emp.1))
                    .ok_or(SaveError::SequenceExhausted)?,
            ),
            secret,
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
    let reception = r.option(practice::reception::Reception::decode)?;
    let mut practice = r.option(|r| practice::Practice::decode(r, version))?;
    match (practice.as_mut(), reception) {
        (Some(practice), Some(reception)) => practice.reception = reception,
        (None, None) => {}
        _ => return Err(SaveError::InvalidEncoding),
    }
    Ok(practice)
}

fn validate_accounting(
    d: SavedDomainSummary,
    s: SavedServiceSummary,
    jumps: u64,
    fuel_spent: u64,
    penalties: u8,
    ammunition: (u32, u64),
    secret: super::xyzzy::Terms,
) -> Result<(), SaveError> {
    let (ammunition_paid, ammunition_revisions) = ammunition;
    let income = 7200_u128
        + u128::from(d.completed) * u128::from(crate::freight::REWARD_TENTHS)
        + u128::from(s.credited);
    let income = adjust_secret_cash(income, secret.cash_adjustment)?;
    let fuel = 130_u128 + u128::from(s.refilled_units) + secret.fuel_supplied;
    let available_gross = 7200_u128
        + u128::from(d.completed) * u128::from(crate::freight::REWARD_TENTHS)
        + s.gross_credit;
    let available_gross = adjust_secret_cash(available_gross, secret.cash_adjustment)?;
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
                + u128::from(secret.revisions)
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
fn adjust_secret_cash(value: u128, adjustment: i128) -> Result<u128, SaveError> {
    if adjustment >= 0 {
        value.checked_add(adjustment as u128)
    } else {
        value.checked_sub(adjustment.unsigned_abs())
    }
    .ok_or(SaveError::InvalidState("XYZZY cash reset chronology"))
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
        Some(MotionRefusal::AsteroidContact) => 5,
    });
}
pub(super) fn read_flight(r: &mut Reader<'_>, version: u16) -> Result<Snapshot, SaveError> {
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
        4 if version >= 15 => Some(MotionRefusal::NpcContact),
        5 if version >= 16 => Some(MotionRefusal::AsteroidContact),
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

/// Build an unsupported-version refusal input with the current payload and checksum.
#[cfg(test)]
pub(in crate::flight) fn unsupported_version_test_input(
    world: &World,
    version: u16,
) -> Result<Vec<u8>, SaveError> {
    assert_ne!(version, SAVE_VERSION);
    let mut bytes = world.save_bytes()?;
    bytes[8..10].copy_from_slice(&version.to_le_bytes());
    let end = bytes.len() - 8;
    let digest = checksum(&bytes[..end]);
    bytes[end..].copy_from_slice(&digest.to_le_bytes());
    Ok(bytes)
}

/// Locate the current pulse field for semantic corruption tests without assuming
/// that the private NPC/resource extensions are absent. Verify both actual bytes.
#[cfg(test)]
pub(super) fn pulse_save_start(world: &World, bytes: &[u8]) -> usize {
    let mut extension = Writer::new();
    world.traffic.encode(&mut extension);
    world.damage.encode(&mut extension);
    world.mining.encode(&mut extension);
    world.jump_journal.encode(&mut extension);
    let extension_end = bytes.len() - 8;
    let extension_start = extension_end - extension.bytes.len();
    assert_eq!(
        &bytes[extension_start..extension_end],
        extension.bytes.as_slice()
    );
    let mut pulse = Writer::new();
    world.pulse.encode_save(&mut pulse);
    let start = extension_start - pulse.bytes.len();
    assert_eq!(&bytes[start..extension_start], pulse.bytes.as_slice());
    start
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
pub(super) use tests::assert_resealed_enlirin_fuel_refusal;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod paid_missile_format_tests {
    use super::*;
    #[test]
    fn unsupported_v11_label_refuses_without_replacing_current_state() {
        let world = World::new_commander(8401).unwrap_or_else(|e| panic!("{e:?}"));
        let old = unsupported_version_test_input(&world, 11).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(u16::from_le_bytes([old[8], old[9]]), 11);
        assert!(matches!(
            World::from_save_bytes(&old),
            Err(SaveError::UnsupportedVersion)
        ));
        let before = world.save_bytes().unwrap_or_else(|e| panic!("{e:?}"));
        let mut world = world;
        assert_eq!(
            world.restore_bytes(&old),
            Err(SaveError::UnsupportedVersion)
        );
        assert_eq!(
            world.save_bytes().unwrap_or_else(|e| panic!("{e:?}")),
            before
        );
    }
}
