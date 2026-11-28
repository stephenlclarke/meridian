use super::{
    Arc, AsteroidId, BELT_BODY_COUNT, INVALID, MiningContact, MiningStroke, Reader, RigidPose,
    SaveError, State, SystemId, Vec3, Writer,
};
fn point(w: &mut Writer, p: Vec3) {
    for x in [p.x, p.y, p.z] {
        w.u64(x.to_bits());
    }
}
fn read_point(r: &mut Reader<'_>) -> Result<Vec3, SaveError> {
    Ok(Vec3::new(
        f64::from_bits(r.u64()?),
        f64::from_bits(r.u64()?),
        f64::from_bits(r.u64()?),
    ))
}
pub(crate) fn rigid(w: &mut Writer, p: RigidPose) {
    point(w, p.position);
    for v in [p.orientation.x, p.orientation.y, p.orientation.z] {
        point(w, v);
    }
}
pub(crate) fn read_rigid(r: &mut Reader<'_>) -> Result<RigidPose, SaveError> {
    Ok(RigidPose {
        position: read_point(r)?,
        orientation: crate::docking::Basis3 {
            x: read_point(r)?,
            y: read_point(r)?,
            z: read_point(r)?,
        },
    })
}
impl State {
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.bool(self.enabled);
        w.u8(self.strokes.len() as u8);
        for hit in self.strokes.iter() {
            w.u64(hit.owner.owner);
            w.u64(hit.owner.hull);
            w.u8(hit.owner.system.galaxy);
            w.u16(hit.owner.system.system);
            w.u64(hit.owner.visit);
            for x in [
                hit.item,
                hit.account_revision,
                hit.world_tick,
                hit.physical_tick,
                hit.emission,
            ] {
                w.u64(x);
            }
            rigid(w, hit.pose);
            w.f32(hit.speed);
            w.u8(hit.contact.asteroid.ordinal);
            w.u64(hit.contact.distance.to_bits());
            point(w, hit.contact.point);
            hit.peers.encode(w);
            w.u16(hit.missiles.len() as u16);
            for (id, pose) in hit.missiles.iter() {
                id.encode(w);
                rigid(w, *pose);
            }
        }
        for expired in &self.expired {
            w.option(expired.as_ref(), |w, (tick, visit)| {
                w.u64(*tick);
                w.u64(*visit);
            });
        }
    }
    #[cfg(test)]
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let value = Self::decode_base(r)?;
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn decode_base(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let enabled = r.bool()?;
        let count = usize::from(r.u8()?);
        if count > BELT_BODY_COUNT * 8 {
            return Err(INVALID);
        }
        let mut strokes = Vec::with_capacity(count);
        for _ in 0..count {
            let owner = crate::CombatIdentity {
                owner: r.u64()?,
                hull: r.u64()?,
                system: SystemId {
                    galaxy: r.u8()?,
                    system: r.u16()?,
                },
                visit: r.u64()?,
            };
            let item = r.u64()?;
            let account_revision = r.u64()?;
            let world_tick = r.u64()?;
            let physical_tick = r.u64()?;
            let emission = r.u64()?;
            let pose = read_rigid(r)?;
            let speed = r.f32()?;
            let contact = MiningContact {
                asteroid: AsteroidId::from_ordinal(r.u8()?)?,
                distance: f64::from_bits(r.u64()?),
                point: read_point(r)?,
            };
            let peers = crate::traffic::MissileScene::decode(r)?;
            let n = usize::from(r.u16()?);
            if n > 256 {
                return Err(INVALID);
            }
            let mut missiles = Vec::with_capacity(n);
            for _ in 0..n {
                missiles.push((crate::ProjectileId::decode(r)?, read_rigid(r)?));
            }
            strokes.push(MiningStroke {
                owner,
                item,
                account_revision,
                world_tick,
                physical_tick,
                emission,
                pose,
                speed,
                contact,
                peers,
                missiles: Arc::new(missiles),
            });
        }
        let expired = r.array(|r| r.option(|r| Ok((r.u64()?, r.u64()?))))?;
        let value = Self {
            enabled,
            strokes: Arc::new(strokes),
            expired,
            collected: Arc::new(Vec::new()),
        };
        Ok(value)
    }
}
