//! Bounded, versioned memory representation. The host owns atomic disk publication.
use core::fmt;

pub const SAVE_VERSION: u16 = 6;
pub const MAX_SAVE_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    NoCommander,
    TooLarge,
    Truncated,
    UnsupportedVersion,
    ContentMismatch,
    ChecksumMismatch,
    InvalidEncoding,
    InvalidState(&'static str),
    SequenceExhausted,
}
impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState(reason) => write!(f, "invalid saved World: {reason}"),
            other => write!(f, "World save refused: {other:?}"),
        }
    }
}
impl std::error::Error for SaveError {}

pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
}
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

macro_rules! number {
    ($write:ident, $read:ident, $ty:ty, $size:expr) => {
        impl Writer {
            pub(crate) fn $write(&mut self, value: $ty) {
                self.bytes.extend(value.to_le_bytes());
            }
        }
        impl Reader<'_> {
            pub(crate) fn $read(&mut self) -> Result<$ty, SaveError> {
                Ok(<$ty>::from_le_bytes(self.fixed::<$size>()?))
            }
        }
    };
}
number!(u16, u16, u16, 2);
number!(u32, u32, u32, 4);
number!(u64, u64, u64, 8);
number!(f32, f32, f32, 4);

impl Writer {
    pub(crate) fn new() -> Self {
        Self { bytes: Vec::new() }
    }
    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    pub(crate) fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }
    pub(crate) fn option<T>(&mut self, value: Option<&T>, write: impl FnOnce(&mut Self, &T)) {
        self.bool(value.is_some());
        if let Some(value) = value {
            write(self, value);
        }
    }
}
impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    pub(crate) fn fixed<const N: usize>(&mut self) -> Result<[u8; N], SaveError> {
        let end = self.offset.checked_add(N).ok_or(SaveError::TooLarge)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(SaveError::Truncated)?;
        self.offset = end;
        value.try_into().map_err(|_| SaveError::InvalidEncoding)
    }
    pub(crate) fn u8(&mut self) -> Result<u8, SaveError> {
        Ok(self.fixed::<1>()?[0])
    }
    pub(crate) fn bool(&mut self) -> Result<bool, SaveError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
    pub(crate) fn option<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, SaveError>,
    ) -> Result<Option<T>, SaveError> {
        if self.bool()? {
            Ok(Some(read(self)?))
        } else {
            Ok(None)
        }
    }
    pub(crate) fn array<T, const N: usize>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, SaveError>,
    ) -> Result<[T; N], SaveError> {
        let mut values = Vec::with_capacity(N);
        for _ in 0..N {
            values.push(read(self)?);
        }
        values.try_into().map_err(|_| SaveError::InvalidEncoding)
    }
    pub(crate) fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

/// Corruption detection only. This is not authentication against a party rewriting a whole save.
pub(crate) fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}
