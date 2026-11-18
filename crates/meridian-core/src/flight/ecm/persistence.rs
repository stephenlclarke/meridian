use super::{
    Activation, Arc, CombatIdentity, EcmOrigin, FieldBoundary, INVALID, MAX_ACTIVATIONS, Reader,
    SaveError, State, Writer,
};
use crate::navigation::SystemId;

impl State {
    pub(in crate::flight) fn encode(&self, writer: &mut Writer) {
        writer.u16(self.activations.len() as u16);
        for activation in self.activations.iter() {
            activation.origin.encode(writer);
            writer.u64(activation.item);
            writer.u64(activation.world_tick);
            writer.u64(activation.physical_tick);
            writer.u64(activation.after_emission);
            writer.option(activation.closed.as_ref(), |writer, boundary| {
                writer.u64(boundary.world_tick);
                writer.u64(boundary.physical_tick);
                writer.u64(boundary.after_emission);
            });
        }
    }
    pub(in crate::flight) fn decode(reader: &mut Reader<'_>) -> Result<Self, SaveError> {
        let count = usize::from(reader.u16()?);
        if count > MAX_ACTIVATIONS {
            return Err(INVALID);
        }
        let mut activations = Vec::with_capacity(count);
        for _ in 0..count {
            activations.push(Activation {
                origin: EcmOrigin::decode(reader)?,
                item: reader.u64()?,
                world_tick: reader.u64()?,
                physical_tick: reader.u64()?,
                after_emission: reader.u64()?,
                closed: reader.option(|reader| {
                    Ok(FieldBoundary {
                        world_tick: reader.u64()?,
                        physical_tick: reader.u64()?,
                        after_emission: reader.u64()?,
                    })
                })?,
            });
        }
        Ok(Self {
            activations: Arc::new(activations),
        })
    }
}

impl EcmOrigin {
    pub(in crate::flight) fn encode(self, writer: &mut Writer) {
        let owner = self.owner;
        writer.u64(owner.owner);
        writer.u64(owner.hull);
        writer.u8(owner.system.galaxy);
        writer.u16(owner.system.system);
        writer.u64(owner.visit);
        writer.u16(self.activation);
    }
    pub(in crate::flight) fn decode(reader: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: CombatIdentity {
                owner: reader.u64()?,
                hull: reader.u64()?,
                system: SystemId {
                    galaxy: reader.u8()?,
                    system: reader.u16()?,
                },
                visit: reader.u64()?,
            },
            activation: reader.u16()?,
        })
    }
}
impl super::EcmTermination {
    pub(in crate::flight) fn encode(self, writer: &mut Writer) {
        self.origin.encode(writer);
        writer.u64(self.after_emission);
    }
    pub(in crate::flight) fn decode(reader: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            origin: EcmOrigin::decode(reader)?,
            after_emission: reader.u64()?,
        })
    }
}
