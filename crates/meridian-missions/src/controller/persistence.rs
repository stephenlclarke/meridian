//! Meridian's versioned explicit script frame, not an implicit legacy JSON migration.
use super::{Event, Instance, Status};
use crate::{Error, Identity, Value, decode, encode};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;

const MAGIC: &[u8; 8] = b"MLST0001";
const STORAGE_BYTES: usize = 33_554_432; // Original typed script storage aggregate byte budget.

impl Instance {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.validate_storage()?;
        let mut bytes = MAGIC.to_vec();
        for text in [
            &self.identity.definition,
            &self.identity.version,
            &self.identity.digest,
        ] {
            frame(&mut bytes, text.as_bytes())?;
        }
        match self.native_binding {
            Some((owner, ordinal)) => {
                bytes.push(1);
                bytes.extend(owner.to_le_bytes());
                bytes.push(ordinal);
            }
            None => bytes.push(0),
        }
        for number in [self.revision, self.next_sequence, self.acknowledged_through] {
            bytes.extend(number.to_le_bytes());
        }
        match &self.status {
            Status::Active => bytes.push(0),
            Status::Suspended(reason) => {
                bytes.push(1);
                frame(&mut bytes, reason.as_bytes())?;
            }
        }
        frame(
            &mut bytes,
            &encode(std::slice::from_ref(&self.data), 262_144)?,
        )?;
        bytes.extend((self.journal.len() as u32).to_le_bytes());
        for text in &self.journal {
            frame(&mut bytes, text.as_bytes())?;
        }
        bytes.extend((self.pending.len() as u32).to_le_bytes());
        for event in &self.pending {
            bytes.extend(event.sequence.to_le_bytes());
            frame(&mut bytes, event.kind.as_bytes())?;
            frame(
                &mut bytes,
                &encode(std::slice::from_ref(&event.value), 262_144)?,
            )?;
        }
        if bytes.len() > STORAGE_BYTES {
            return Err(Error::Limit("script storage bytes"));
        }
        let checksum = Sha256::digest(&bytes);
        bytes.extend_from_slice(&checksum);
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 40 || bytes.len() > STORAGE_BYTES + 32 {
            return Err(Error::Codec("script storage size"));
        }
        let (body, checksum) = bytes.split_at(bytes.len() - 32);
        if &Sha256::digest(body)[..] != checksum {
            return Err(Error::Codec("script storage checksum"));
        }
        let mut reader = Reader {
            bytes: body,
            cursor: 0,
        };
        if reader.take(8)? != MAGIC {
            return Err(Error::Codec("unsupported script storage version"));
        }
        let identity = Identity {
            definition: reader.text(320)?,
            version: reader.text(80)?,
            digest: reader.text(64)?,
        };
        let native_binding = match reader.byte()? {
            0 => None,
            1 => Some((reader.number()?, reader.byte()?)),
            _ => return Err(Error::Codec("native binding tag")),
        };
        let revision = reader.number()?;
        let next_sequence = reader.number()?;
        let acknowledged_through = reader.number()?;
        let status = match reader.byte()? {
            0 => Status::Active,
            1 => Status::Suspended(reader.text(12_000)?),
            _ => return Err(Error::Codec("script status tag")),
        };
        let data = decode(reader.frame(262_144)?)?;
        let count = reader.count(512)?;
        let mut journal = Vec::new();
        for _ in 0..count {
            journal.push(reader.text(12_000)?);
        }
        let count = reader.count(4096)?;
        let mut pending = VecDeque::new();
        for _ in 0..count {
            pending.push_back(Event {
                sequence: reader.number()?,
                kind: reader.text(160)?,
                value: decode(reader.frame(262_144)?)?,
            });
        }
        if reader.cursor != body.len() {
            return Err(Error::Codec("trailing script storage"));
        }
        let instance = Self {
            identity,
            native_binding,
            data,
            revision,
            next_sequence,
            acknowledged_through,
            status,
            journal,
            pending,
        };
        instance.validate_storage()?;
        Ok(instance)
    }
    fn validate_storage(&self) -> Result<(), Error> {
        if self.identity.definition.is_empty()
            || self.identity.definition.len() > 320
            || self.identity.version.is_empty()
            || self.identity.version.len() > 80
            || self.identity.digest.len() != 64
            || !self
                .identity
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || [self.revision, self.next_sequence]
                .iter()
                .any(|v| *v >= i64::MAX as u64)
            || self.acknowledged_through > self.next_sequence
            || !matches!(self.data, Value::Map(_))
            || self.pending.len() > 4096
            || self.journal.len() > 512
            || self.journal.iter().any(|s| s.len() > 12_000)
        {
            return Err(Error::Codec("invalid explicit script state"));
        }
        let mut previous = 0;
        for event in &self.pending {
            if event.sequence <= previous
                || event.sequence > self.next_sequence
                || event.kind.is_empty()
                || event.kind.len() > 160
            {
                return Err(Error::Codec("invalid pending script event"));
            }
            previous = event.sequence;
        }
        if self
            .native_binding
            .is_some_and(|(_, ordinal)| ordinal >= 128)
        {
            return Err(Error::Codec("invalid native script binding"));
        }
        if let Status::Suspended(reason) = &self.status {
            if reason.is_empty() || reason.len() > 12_000 {
                return Err(Error::Codec("invalid suspension"));
            }
        }
        Ok(())
    }
}
fn frame(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), Error> {
    if value.len() > STORAGE_BYTES.saturating_sub(bytes.len() + 4) {
        return Err(Error::Limit("script storage bytes"));
    }
    bytes.extend((value.len() as u32).to_le_bytes());
    bytes.extend(value);
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        if count > self.bytes.len().saturating_sub(self.cursor) {
            return Err(Error::Codec("truncated script storage"));
        }
        let start = self.cursor;
        self.cursor += count;
        Ok(&self.bytes[start..self.cursor])
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn number(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| Error::Codec("script number"))?,
        ))
    }
    fn count(&mut self, maximum: usize) -> Result<usize, Error> {
        let count = u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| Error::Codec("script length"))?,
        ) as usize;
        if count > maximum {
            return Err(Error::Limit("script count/frame"));
        }
        Ok(count)
    }
    fn frame(&mut self, maximum: usize) -> Result<&'a [u8], Error> {
        let count = self.count(maximum)?;
        self.take(count)
    }
    fn text(&mut self, maximum: usize) -> Result<String, Error> {
        String::from_utf8(self.frame(maximum)?.to_vec())
            .map_err(|_| Error::Codec("script storage UTF-8"))
    }
}
