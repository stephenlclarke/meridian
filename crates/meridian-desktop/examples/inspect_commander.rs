//! Read-only qualification observer. Uses the real public World decoder, never a second codec.
use meridian_core::{World, save::MAX_SAVE_FILE_BYTES};
use std::{error::Error, fs::File, io::Read, path::Path};

fn bounded_bytes(reader: impl Read) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_SAVE_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SAVE_FILE_BYTES {
        return Err("Commander file exceeds the current save bound".into());
    }
    Ok(bytes)
}

fn inspect(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = bounded_bytes(File::open(path)?)?;
    let world = World::from_save_bytes(&bytes)?;
    // Header access follows the complete semantic decoder, which has already checked its size.
    let version = u16::from_le_bytes([bytes[8], bytes[9]]);
    let commodities = meridian_core::station_services::CommodityId::ALL.map(|id| {
        (
            id,
            id.unit(),
            world.commodity_quantity(id),
            world.reserved_commodity_quantity(id),
            world.commodity_quote(id),
        )
    });
    Ok(format!(
        "saved_format={version}\nflight={:?}\nview={:?}\ncredits_tenths={:?}\ncommodities={commodities:?}\nservices={:?}\nmodular={:?}\nammunition={:?}\ncombat={:?}\nflight_resources={:?}\npulse={:?}\nmissiles={:?}\ntraffic={:?}\nmission={:?}\npractice={:?}\nflight_lesson={:?}\ntrade_lesson={:?}\ncelestial={:?}\n",
        world.snapshot(),
        world.flight_view(),
        world.credits_tenths(),
        world.service_snapshot(),
        world.modular_equipment_snapshot(),
        world.ammunition_snapshot(),
        world.combat_snapshot(),
        world.flight_resources_snapshot(),
        world.pulse_laser_snapshot(),
        world.missile_snapshot(),
        world.traffic_snapshot(),
        world.mission_snapshot(),
        world.practice_snapshot(),
        world.flight_reception_snapshot(),
        world.navigation_trade_snapshot(),
        world.celestial_snapshot(),
    ))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .filter(|p| !p.is_empty())
        .ok_or("Usage: inspect_commander COMMANDER_FILE")?;
    if args.next().is_some() {
        return Err("Usage: inspect_commander COMMANDER_FILE".into());
    }
    print!("{}", inspect(Path::new(&path))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::hash_map::RandomState,
        fs,
        hash::{BuildHasher, Hasher},
    };

    struct Slot(std::path::PathBuf);
    impl Slot {
        fn new() -> Self {
            let nonce = RandomState::new().build_hasher().finish();
            let path = std::env::temp_dir().join(format!(
                "meridian-inspector-{}-{nonce:016x}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap_or_else(|e| panic!("{e}"));
            Self(path)
        }
    }
    impl Drop for Slot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|error| panic!("{error:?}"))
    }

    #[test]
    fn actual_public_decoder_observation_does_not_modify_the_commander_or_directory() {
        let slot = Slot::new();
        let world = ok(World::new_commander(12005));
        let bytes = ok(world.save_bytes());
        let path = slot.0.join("commander.meridian");
        ok(fs::write(&path, &bytes));
        let output = ok(inspect(&path));
        assert!(output.starts_with(&format!(
            "saved_format={}\n",
            meridian_core::save::SAVE_VERSION
        )));
        assert!(output.contains("credits_tenths=Some(7200)\n"));
        assert!(output.contains(&format!("traffic={:?}\n", world.traffic_snapshot())));
        assert!(output.contains(&format!(
            "flight_resources={:?}\n",
            world.flight_resources_snapshot()
        )));
        assert_eq!(ok(fs::read(&path)), bytes);
        assert_eq!(ok(fs::read_dir(&slot.0)).count(), 1);
    }

    #[test]
    fn corrupt_commander_is_refused_without_rewriting_or_replacing_it() {
        let slot = Slot::new();
        let mut bytes = ok(ok(World::new_commander(12005)).save_bytes());
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        let path = slot.0.join("commander.meridian");
        ok(fs::write(&path, &bytes));
        assert!(inspect(&path).is_err());
        assert_eq!(ok(fs::read(&path)), bytes);
        assert_eq!(ok(fs::read_dir(&slot.0)).count(), 1);
    }

    #[test]
    fn oversize_and_truncated_inputs_never_bypass_the_existing_world_decoder() {
        assert!(bounded_bytes(std::io::Cursor::new(vec![0; MAX_SAVE_FILE_BYTES + 1])).is_err());
        let slot = Slot::new();
        let path = slot.0.join("commander.meridian");
        for bytes in [
            vec![],
            b"MERSAVE1".to_vec(),
            vec![0; MAX_SAVE_FILE_BYTES + 1],
        ] {
            ok(fs::write(&path, &bytes));
            assert!(inspect(&path).is_err());
            assert_eq!(ok(fs::read(&path)), bytes);
        }
    }
}
