//! Original T01 acceptance and preservation of its explicit state across native publications.
use crate::controller::MAX_SCRIPT_STATE_BYTES;
use crate::{Action, Error, Event, Instance, Pack, Value};
use meridian_core::save::MAX_SAVE_BYTES;
use meridian_core::{FlightReceptionAction, FlightReceptionReview, PreparedFlightReception, World};
use sha2::{Digest, Sha256};

const T01_DIGEST: &str = "bc01834e04f87816c8021b76fabe3d9b8f3af394ff8fcd420c4b7c3acf1350d4";
const MAGIC: &[u8; 8] = b"MMIS0001";
const ENVELOPE_OVERHEAD: usize = 48;

/// Maximum complete MMIS0001 publication: native save cap + complete script-frame cap +
/// envelope framing/checksum. Hosts must apply this bound to file reads and atomic writes.
/// Each contained native save and script frame still has its own independent size limit.
pub const MAX_FLIGHT_PUBLICATION_BYTES: usize =
    MAX_SAVE_BYTES + MAX_SCRIPT_STATE_BYTES + ENVELOPE_OVERHEAD;

/// Holds exclusive borrows through publication. It contains no Lua VM or callable script handle.
pub struct PreparedFlightTransition<'a> {
    native: PreparedFlightReception<'a>,
    instance: &'a mut Instance,
    candidate: Instance,
    bytes: Vec<u8>,
}
/// The original accepted-callback preparation uses the same atomic native/script publication.
pub type PreparedFlightAcceptance<'a> = PreparedFlightTransition<'a>;

impl PreparedFlightTransition<'_> {
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
    validate_binding(&candidate_world, &candidate)?;
    let bytes = envelope(native.bytes(), &candidate.to_bytes()?)?;
    Ok(PreparedFlightTransition {
        native,
        instance,
        candidate,
        bytes,
    })
}

/// Prepares a later real T01 Accept, Cancel or Receive without executing a Lua lifecycle callback.
/// The retained binding identifies the original accepted attempt in native history. Later accepts
/// never rebind it or replay its journal. Suspension and pending explicit state remain unchanged.
pub fn prepare_flight_transition<'a>(
    world: &'a mut World,
    instance: &'a mut Instance,
    review: FlightReceptionReview,
    expected_tick: u64,
) -> Result<PreparedFlightTransition<'a>, Error> {
    validate_binding(world, instance)?;
    let native = world
        .prepare_flight_reception(review, expected_tick)
        .map_err(|e| Error::World(e.to_string()))?;
    let bytes = serialize_flight_publication(native.bytes(), instance)?;
    let candidate = instance.clone();
    Ok(PreparedFlightTransition {
        native,
        instance,
        candidate,
        bytes,
    })
}

/// Wraps actual current/prospective native save bytes with retained, already-bound script state.
/// This validates both payloads and their real T01 binding without executing Lua or committing
/// native proposals. The host owns publication ordering and any subsequent native commit.
/// An unbound instance before the first real accepted callback is not a mission publication.
pub fn serialize_flight_publication(native: &[u8], instance: &Instance) -> Result<Vec<u8>, Error> {
    if native.len() > MAX_SAVE_BYTES {
        return Err(Error::Limit("native publication bytes"));
    }
    let world = World::from_save_bytes(native).map_err(|e| Error::World(e.to_string()))?;
    validate_binding(&world, instance)?;
    envelope(native, &instance.to_bytes()?)
}

pub fn restore_flight_publication(
    bytes: &[u8],
    pack: Option<&Pack>,
) -> Result<(World, Instance), Error> {
    if bytes.len() > MAX_FLIGHT_PUBLICATION_BYTES {
        return Err(Error::Limit("mission publication bytes"));
    }
    if bytes.len() < ENVELOPE_OVERHEAD || &bytes[..8] != MAGIC {
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
    if world_length > MAX_SAVE_BYTES || script_length > MAX_SCRIPT_STATE_BYTES {
        return Err(Error::Limit("mission publication payload bytes"));
    }
    let world = World::from_save_bytes(&body[16..16 + world_length])
        .map_err(|e| Error::World(e.to_string()))?;
    let instance = Instance::from_bytes(&body[16 + world_length..])?;
    validate_binding(&world, &instance)?;
    if let Some(pack) = pack {
        require_t01(pack)?;
    }
    let instance = instance.reconcile(pack)?;
    Ok((world, instance))
}
fn validate_binding(world: &World, instance: &Instance) -> Result<(), Error> {
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
    Ok(())
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
    if world.len() > MAX_SAVE_BYTES || script.len() > MAX_SCRIPT_STATE_BYTES {
        return Err(Error::Limit("mission publication payload bytes"));
    }
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
