use super::*;
/// Only the actor owner derives this from actual paid births and actual completed docking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Flows {
    pub(crate) bought: [[u64; 17]; 2],
    pub(crate) sold: [[u64; 17]; 2],
}
impl State {
    pub(crate) fn flows(&self) -> Flows {
        self.flows_at(u64::MAX)
    }
    pub(crate) fn flows_at(&self, tick: u64) -> Flows {
        let mut flows = Flows::default();
        for a in self
            .actors
            .iter()
            .flatten()
            .filter(|a| a.id.serial == 1 && a.born_world_tick <= tick)
        {
            if let Some(i) = system_index(a.id.system) {
                flows.bought[i][a.manifest.index()] += u64::from(a.units);
                if a.sold_world_tick.is_some_and(|sold| sold <= tick) {
                    flows.sold[i][a.manifest.index()] += u64::from(a.units);
                }
            }
        }
        flows
    }
    pub(super) fn birth(
        &mut self,
        context: &TickContext<'_>,
        markets: &mut StationMarkets,
    ) -> Result<(), TrafficError> {
        let system = context.system;
        let data = context.data;
        let player_before = context.player_before.position;
        let player_after = context.player_after.position;
        let world_tick = context.world_tick;
        let celestial_clear = context.celestial_clear;
        let Some(i) = system_index(system) else {
            return Ok(());
        };
        let clock = self.clocks[i];
        let radius = data.player_clearance_radius();
        if clock >= 480 && self.actors[i * 2].is_none() {
            let launch = data
                .launch_start()
                .map_err(|_| TrafficError::InvalidState)?;
            let start = crate::station::flight_pose(launch.position, launch.orientation)
                .ok_or(TrafficError::InvalidState)?;
            if self.spawn_clear(system, start.position, player_before, player_after, radius)
                && celestial_clear(start.position, start.position, radius)
            {
                let (commodity, units) = manifest(self.seed, system);
                let site = site(system).ok_or(TrafficError::InvalidState)?;
                let quote = markets.commodity_quote(site, commodity);
                let paid = quote
                    .ask_tenths
                    .checked_mul(u32::from(units))
                    .ok_or(TrafficError::InvalidState)?;
                if paid <= CAPITAL
                    && let Ok(next) =
                        markets.prepare_traffic_stock(site, commodity, u64::from(units), true)
                {
                    self.actors[i * 2] = Some(Actor {
                        id: NpcId {
                            profile: self.seed,
                            system,
                            serial: 1,
                        },
                        phase: NpcPhase::Departing,
                        position: start.position,
                        yaw: 0.0,
                        speed: f64::from(launch.speed),
                        birth_clock: clock,
                        born_world_tick: world_tick,
                        sold_world_tick: None,
                        manifest: commodity,
                        units,
                        wallet: CAPITAL - paid,
                        sold: false,
                        resources: Resources::fresh(),
                        resource_clock: clock - 1,
                        aggressor: None,
                        berth: None,
                    });
                    *markets = next;
                }
            }
        }
        if clock >= 900 && pirate_present(self.seed, system) && self.actors[i * 2 + 1].is_none() {
            let position = Vec3::new(2400.0, 0.0, 10500.0);
            let native = pose(position, core::f64::consts::PI);
            if self.spawn_clear(system, position, player_before, player_after, radius)
                && data.motion_clear(native, native, 0.0)
                && celestial_clear(position, position, radius)
            {
                self.actors[i * 2 + 1] = Some(Actor {
                    id: NpcId {
                        profile: self.seed,
                        system,
                        serial: 2,
                    },
                    phase: NpcPhase::Hunting,
                    position,
                    yaw: core::f64::consts::PI,
                    speed: 8.0,
                    birth_clock: clock,
                    born_world_tick: world_tick,
                    sold_world_tick: None,
                    manifest: CommodityId::Minerals,
                    units: 0,
                    wallet: 0,
                    sold: false,
                    resources: Resources::fresh(),
                    resource_clock: clock - 1,
                    aggressor: None,
                    berth: None,
                });
            }
        }
        Ok(())
    }
    fn spawn_clear(
        &self,
        system: SystemId,
        position: Vec3,
        player_before: Vec3,
        player_after: Vec3,
        radius: f64,
    ) -> bool {
        segment_distance(player_before, player_after, position) > radius * 2.0 + 64.0
            && self
                .actors
                .iter()
                .flatten()
                .filter(|a| a.id.system == system && a.solid())
                .all(|a| length(sub(a.position, position)) > radius * 2.0 + 64.0)
    }
    pub(super) fn finish_trade(
        a: &mut Actor,
        markets: &mut StationMarkets,
        world_tick: u64,
    ) -> Result<(), TrafficError> {
        if a.id.serial != 1 || a.sold || a.berth.is_none() || a.phase != NpcPhase::Inbound {
            return Err(TrafficError::InvalidState);
        }
        let site = site(a.id.system).ok_or(TrafficError::InvalidState)?;
        let bid = markets.commodity_quote(site, a.manifest).bid_tenths;
        let credit = bid
            .checked_mul(u32::from(a.units))
            .ok_or(TrafficError::SequenceExhausted)?;
        let wallet = a
            .wallet
            .checked_add(credit)
            .ok_or(TrafficError::SequenceExhausted)?;
        let next = markets
            .prepare_traffic_stock(site, a.manifest, u64::from(a.units), false)
            .map_err(|_| TrafficError::InvalidState)?;
        a.wallet = wallet;
        a.sold = true;
        a.sold_world_tick = Some(world_tick);
        a.phase = NpcPhase::Complete;
        a.speed = 0.0;
        *markets = next;
        Ok(())
    }
}
