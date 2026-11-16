//! First supported native path: original T01 accepted callback joined to actual World preparation.
use crate::{Action, Error, Event, Instance, Pack, Value};
use meridian_core::{FlightReceptionAction, FlightReceptionReview, PreparedFlightReception, World};
use sha2::{Digest, Sha256};

const T01_DIGEST: &str = "bc01834e04f87816c8021b76fabe3d9b8f3af394ff8fcd420c4b7c3acf1350d4";
const MAGIC: &[u8; 8] = b"MMIS0001";

/// Holds exclusive borrows through publication. It contains no Lua VM or callable script handle.
pub struct PreparedFlightAcceptance<'a> {
    native: PreparedFlightReception<'a>,
    instance: &'a mut Instance,
    candidate: Instance,
    bytes: Vec<u8>,
}
impl PreparedFlightAcceptance<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn candidate_script(&self) -> &Instance {
        &self.candidate
    }
    /// The host atomically publishes this complete envelope to its owned profile location.
    /// A failure or dropped preparation changes neither World nor explicit script state.
    pub fn publish(self, publish: impl FnOnce(&[u8]) -> Result<(), Error>) -> Result<(), Error> {
        publish(&self.bytes)?;
        self.native.commit();
        *self.instance = self.candidate;
        Ok(())
    }
}

pub fn prepare_flight_acceptance<'a>(
    world: &'a mut World,
    instance: &'a mut Instance,
    pack: &Pack,
    review: FlightReceptionReview,
    expected_tick: u64,
) -> Result<PreparedFlightAcceptance<'a>, Error> {
    require_t01(pack)?;
    if instance.identity() != pack.identity()
        || instance.native_binding.is_some()
        || instance.revision() != 0
        || instance.next_sequence != 0
        || !instance.pending().is_empty()
        || !instance.journal().is_empty()
        || instance.data() != &Value::empty_map()
        || review.action() != FlightReceptionAction::Accept
    {
        return Err(Error::Identity);
    }
    // This is the existing opaque native review/provider. No Lua fields create the acceptance.
    let native = world
        .prepare_flight_reception(review, expected_tick)
        .map_err(|e| Error::World(e.to_string()))?;
    // Decode the exact immutable native candidate only for its public snapshot. No simulation is
    // advanced, no second World is retained, and no decoded proof is submitted to the live World.
    let candidate_world =
        World::from_save_bytes(native.bytes()).map_err(|e| Error::World(e.to_string()))?;
    let dock = candidate_world
        .current_dock_context()
        .ok_or(Error::Pack("candidate T01 berth missing"))?;
    let active = candidate_world
        .flight_reception_snapshot()
        .and_then(|s| s.active)
        .ok_or(Error::Pack("candidate T01 acceptance missing"))?;
    let sequence = instance
        .next_sequence
        .checked_add(1)
        .ok_or(Error::Limit("event sequence"))?;
    let event_value = Value::map([
        (
            "instance_id",
            Value::String(format!("t01:{}:{}", active.owner(), active.ordinal())),
        ),
        ("context", Value::Null),
        ("kind", Value::String("mission.accepted".into())),
        (
            "sequence",
            Value::Integer(i64::try_from(sequence).map_err(|_| Error::Limit("event sequence"))?),
        ),
    ]);
    let scoped = Value::map([
        ("galaxy", Value::Integer(i64::from(dock.system().galaxy))),
        ("system", Value::Integer(i64::from(dock.system().system))),
        ("docked", Value::Bool(true)),
        ("main_station", Value::Bool(true)),
    ]);
    let queued = instance.with_event(Event {
        sequence,
        kind: "mission.accepted".into(),
        value: event_value,
    })?;
    let transition = queued.prepare(pack, sequence, &scoped)?;
    if transition
        .actions()
        .iter()
        .any(|a| !matches!(a, Action::JournalAppend(_)))
    {
        return Err(Error::UnsupportedOperation(
            "T01 accepts journal proposals only".into(),
        ));
    }
    let mut candidate = transition.candidate;
    candidate.native_binding = Some((active.owner(), active.ordinal()));
    let script = candidate.to_bytes()?;
    let bytes = envelope(native.bytes(), &script)?;
    Ok(PreparedFlightAcceptance {
        native,
        instance,
        candidate,
        bytes,
    })
}

pub fn restore_flight_publication(
    bytes: &[u8],
    pack: Option<&Pack>,
) -> Result<(World, Instance), Error> {
    if bytes.len() < 48 || &bytes[..8] != MAGIC {
        return Err(Error::Codec("unsupported mission publication"));
    }
    let (body, checksum) = bytes.split_at(bytes.len() - 32);
    if &Sha256::digest(body)[..] != checksum {
        return Err(Error::Codec("mission publication checksum"));
    }
    let world_length = u32::from_le_bytes(
        body[8..12]
            .try_into()
            .map_err(|_| Error::Codec("publication length"))?,
    ) as usize;
    let script_length = u32::from_le_bytes(
        body[12..16]
            .try_into()
            .map_err(|_| Error::Codec("publication length"))?,
    ) as usize;
    if world_length > body.len() - 16 || script_length != body.len() - 16 - world_length {
        return Err(Error::Codec("mission publication lengths"));
    }
    let world = World::from_save_bytes(&body[16..16 + world_length])
        .map_err(|e| Error::World(e.to_string()))?;
    let instance = Instance::from_bytes(&body[16 + world_length..])?;
    if instance.identity.definition != "l337.training/flight-reception"
        || instance.identity.version != "1.0.0"
        || instance.identity.digest != T01_DIGEST
    {
        return Err(Error::Identity);
    }
    let binding = instance.native_binding.ok_or(Error::Identity)?;
    if !world.flight_reception_snapshot().is_some_and(|s| {
        s.attempts
            .iter()
            .any(|a| (a.id.owner(), a.id.ordinal()) == binding)
    }) {
        return Err(Error::Identity);
    }
    if let Some(pack) = pack {
        require_t01(pack)?;
    }
    let instance = instance.reconcile(pack)?;
    Ok((world, instance))
}
fn require_t01(pack: &Pack) -> Result<(), Error> {
    let identity = pack.identity();
    if identity.definition != "l337.training/flight-reception"
        || identity.version != "1.0.0"
        || identity.digest != T01_DIGEST
    {
        return Err(Error::Identity);
    }
    Ok(())
}
fn envelope(world: &[u8], script: &[u8]) -> Result<Vec<u8>, Error> {
    let mut bytes = MAGIC.to_vec();
    for length in [world.len(), script.len()] {
        bytes.extend(
            u32::try_from(length)
                .map_err(|_| Error::Limit("publication length"))?
                .to_le_bytes(),
        );
    }
    bytes.extend(world);
    bytes.extend(script);
    let checksum = Sha256::digest(&bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}
