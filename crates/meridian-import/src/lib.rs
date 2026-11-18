//! Offline, source-pinned SH025/SH028 conversion. No World or renderer dependency.

mod encode;
mod error;
mod output;
mod palette;
mod source;

pub use error::{ImportError, Result};
pub use output::{Artifact, ImportBatch};
use sha2::{Digest, Sha256};
pub use source::{Model, REVISION, read_file, read_repository};

pub fn sha256(raw: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for &byte in Sha256::digest(raw).iter() {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 15)]));
    }
    text
}

pub fn import_ship(raw: &[u8]) -> Result<ImportBatch> {
    let ship = source::decode(Model::Sh025, raw)?;
    ImportBatch::new(
        vec![Artifact::verified(
            "sh025.mesh",
            encode::display(&ship.design)?,
        )?],
        vec![Model::Sh025],
        false,
    )
}

pub fn import_station(ship: &[u8], station: &[u8]) -> Result<ImportBatch> {
    import_pair(ship, station, false)
}

pub fn import_all(ship: &[u8], station: &[u8]) -> Result<ImportBatch> {
    import_pair(ship, station, true)
}

fn import_pair(ship: &[u8], station: &[u8], include_ship_display: bool) -> Result<ImportBatch> {
    let ship = source::decode(Model::Sh025, ship)?;
    let station = source::decode(Model::Sh028, station)?;
    let docking = station
        .docking
        .as_ref()
        .ok_or(ImportError::Geometry("Missing station port"))?;
    let mut artifacts = Vec::new();
    if include_ship_display {
        artifacts.push(Artifact::verified(
            "sh025.mesh",
            encode::display(&ship.design)?,
        )?);
    }
    artifacts.extend([
        Artifact::verified("sh028.mesh", encode::display(&station.design)?)?,
        Artifact::verified("sh028.physics", encode::physical(&station.native_design)?)?,
        Artifact::verified("sh025.physics", encode::physical(&ship.native_design)?)?,
        Artifact::verified("sh028.port", encode::port(docking)?)?,
    ]);
    ImportBatch::new(artifacts, vec![Model::Sh025, Model::Sh028], true)
}

#[cfg(test)]
mod tests;
