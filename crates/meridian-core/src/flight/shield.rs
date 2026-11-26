//! Stock shield grades and the single player-owned pair of finite resource banks.
use crate::save::{Reader, SaveError, Writer};

pub const SHIELD_CAPACITY: u16 = 30_000;
const RECHARGE_HUNDREDTHS_PER_TICK: u64 = 3;
const EXTENDED_BANKS: u16 = u16::MAX;

/// Original stock module grades, not the historical legacy booster-strength aliases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ShieldGrade {
    #[default]
    Basic,
    BasicII,
    Intermediate,
    IntermediateII,
    Advanced,
    AdvancedII,
    Elite,
}

impl ShieldGrade {
    pub const ALL: [Self; 7] = [
        Self::Basic,
        Self::BasicII,
        Self::Intermediate,
        Self::IntermediateII,
        Self::Advanced,
        Self::AdvancedII,
        Self::Elite,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Basic => "Basic Shield",
            Self::BasicII => "Basic II Shield Module",
            Self::Intermediate => "Intermediate Shield Module",
            Self::IntermediateII => "Intermediate II Shield Module",
            Self::Advanced => "Advanced Shield Module",
            Self::AdvancedII => "Advanced II Shield Module",
            Self::Elite => "Elite Shield Module",
        }
    }

    /// The original saved/purchase identities are distinct from the current display names.
    pub const fn node(self) -> &'static str {
        match self {
            Self::Basic => "Basic Shield",
            Self::BasicII => "Pulse Shield Booster",
            Self::Intermediate => "Mining Shield Booster",
            Self::IntermediateII => "Beam Shield Booster",
            Self::Advanced => "Military Shield Booster",
            Self::AdvancedII => "Capital Ship Shield Booster",
            Self::Elite => "Elite Shield Module",
        }
    }

    /// Exact stock capacity for the current SH025 300-unit banks, in hundredths.
    pub const fn capacity(self) -> u32 {
        [30_000, 45_000, 54_000, 60_000, 69_000, 207_000, 276_000][self as usize]
    }

    /// Standard berth offer (credit tenths, displayed one-based technology).
    /// Advanced II is a reward or PLUGH-specific purchase; Elite is acquisition-only.
    pub const fn ordinary_offer(self) -> Option<(u32, u8)> {
        match self {
            Self::BasicII => Some((10_875, 3)),
            Self::Intermediate => Some((24_750, 6)),
            Self::IntermediateII => Some((35_250, 7)),
            Self::Advanced => Some((123_000, 14)),
            _ => None,
        }
    }

    pub const fn predecessor(self) -> Option<Self> {
        match self {
            Self::Basic => None,
            Self::BasicII => Some(Self::Basic),
            Self::Intermediate => Some(Self::BasicII),
            Self::IntermediateII => Some(Self::Intermediate),
            Self::Advanced => Some(Self::IntermediateII),
            Self::AdvancedII => Some(Self::Advanced),
            Self::Elite => Some(Self::AdvancedII),
        }
    }

    pub const fn equipment_units(self) -> u16 {
        match self {
            Self::Basic => 0,
            Self::Elite => 1,
            _ => 2,
        }
    }

    pub(super) fn from_tag(tag: u8) -> Result<Self, SaveError> {
        Self::ALL
            .get(usize::from(tag))
            .copied()
            .ok_or(SaveError::InvalidEncoding)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShieldPools {
    grade: ShieldGrade,
    banks: [u32; 2],
}

impl Default for ShieldPools {
    fn default() -> Self {
        Self {
            grade: ShieldGrade::Basic,
            banks: [u32::from(SHIELD_CAPACITY); 2],
        }
    }
}

impl ShieldPools {
    pub(super) fn grade(self) -> ShieldGrade {
        self.grade
    }
    pub(super) fn amounts(self) -> [u32; 2] {
        self.banks
    }
    pub(super) fn capacity(self) -> u32 {
        self.grade.capacity()
    }
    pub(super) fn valid(self) -> bool {
        self.banks.iter().all(|&bank| bank <= self.capacity())
    }
    pub(super) fn is_full(self) -> bool {
        self.banks == [self.capacity(); 2]
    }

    pub(super) fn absorb(&mut self, front: bool, damage: u16) -> u16 {
        let bank = &mut self.banks[usize::from(!front)];
        let absorbed = (*bank).min(u32::from(damage));
        *bank -= absorbed;
        // The absorbed value cannot exceed the u16 damage argument.
        absorbed as u16
    }

    pub(super) fn advance(&mut self, ticks: u64) {
        let capacity = u64::from(self.capacity());
        for bank in &mut self.banks {
            *bank = u64::from(*bank)
                .saturating_add(ticks.saturating_mul(RECHARGE_HUNDREDTHS_PER_TICK))
                .min(capacity) as u32;
        }
    }

    pub(super) fn fill(&mut self) {
        self.banks = [self.capacity(); 2];
    }

    /// Match the original preserve-resource-fractions rule at the integer game boundary.
    /// Round to the closest hundredth, with ties upward; never reset a depleted bank.
    pub(super) fn refit(&mut self, grade: ShieldGrade) {
        let old = u64::from(self.capacity());
        let next = u64::from(grade.capacity());
        for bank in &mut self.banks {
            *bank = ((u64::from(*bank) * next + old / 2) / old).min(next) as u32;
        }
        self.grade = grade;
    }

    // A Basic-only current or historical v17 record retains its exact four-byte field.
    // Real Basic banks never contain 65535, so non-Basic has a single unambiguous marker.
    pub(super) fn encode(self, w: &mut Writer) {
        if self.grade == ShieldGrade::Basic {
            // Invalid Basic values are kept invalid when tests deliberately seal bad state.
            w.u16(u16::try_from(self.banks[0]).unwrap_or(EXTENDED_BANKS - 1));
            w.u16(u16::try_from(self.banks[1]).unwrap_or(EXTENDED_BANKS - 1));
        } else {
            w.u16(EXTENDED_BANKS);
            w.u8(self.grade as u8);
            w.u32(self.banks[0]);
            w.u32(self.banks[1]);
        }
    }

    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let front = r.u16()?;
        if front != EXTENDED_BANKS {
            return Ok(Self {
                grade: ShieldGrade::Basic,
                banks: [u32::from(front), u32::from(r.u16()?)],
            });
        }
        let grade = ShieldGrade::from_tag(r.u8()?)?;
        if grade == ShieldGrade::Basic {
            return Err(SaveError::InvalidEncoding);
        }
        Ok(Self {
            grade,
            banks: [r.u32()?, r.u32()?],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_grades_keep_distinct_capacity_identity_and_offer_policy() {
        let expected = [30_000, 45_000, 54_000, 60_000, 69_000, 207_000, 276_000];
        for (index, grade) in ShieldGrade::ALL.into_iter().enumerate() {
            assert_eq!(grade.capacity(), expected[index]);
            assert_eq!(ShieldGrade::from_tag(index as u8), Ok(grade));
            assert_eq!(
                grade.predecessor(),
                index.checked_sub(1).map(|i| ShieldGrade::ALL[i])
            );
        }
        assert_eq!(ShieldGrade::BasicII.ordinary_offer(), Some((10_875, 3)));
        assert_eq!(
            ShieldGrade::Intermediate.ordinary_offer(),
            Some((24_750, 6))
        );
        assert_eq!(
            ShieldGrade::IntermediateII.ordinary_offer(),
            Some((35_250, 7))
        );
        assert_eq!(ShieldGrade::Advanced.ordinary_offer(), Some((123_000, 14)));
        for grade in [
            ShieldGrade::Basic,
            ShieldGrade::AdvancedII,
            ShieldGrade::Elite,
        ] {
            assert_eq!(grade.ordinary_offer(), None);
        }
        assert!(ShieldGrade::from_tag(7).is_err());
    }

    #[test]
    fn independent_wide_banks_preserve_fractions_and_receive_finite_damage() {
        let mut pools = ShieldPools::default();
        assert_eq!(pools.absorb(true, 15_000), 15_000);
        assert_eq!(pools.absorb(false, 30_000), 30_000);
        pools.refit(ShieldGrade::Elite);
        assert_eq!(pools.amounts(), [138_000, 0]);
        assert_eq!(pools.absorb(true, u16::MAX), u16::MAX);
        assert_eq!(pools.absorb(false, 1), 0);
        pools.advance(10);
        assert_eq!(pools.amounts(), [72_495, 30]);
        pools.refit(ShieldGrade::Basic);
        assert_eq!(pools.amounts(), [7_880, 3]);
        pools.advance(u64::MAX);
        assert!(pools.is_full());
        assert!(pools.valid());
        pools.refit(ShieldGrade::AdvancedII);
        assert_eq!(pools.amounts(), [207_000; 2]);
        assert_eq!(pools.grade(), ShieldGrade::AdvancedII);
        pools.fill();
        assert!(pools.is_full());
    }

    #[test]
    fn basic_codec_is_byte_identical_and_extended_grade_and_bounds_are_distinct() {
        let basic = ShieldPools::default();
        let mut writer = Writer::new();
        basic.encode(&mut writer);
        assert_eq!(writer.bytes, [0x30, 0x75, 0x30, 0x75]);
        assert_eq!(
            ShieldPools::decode(&mut Reader::new(&writer.bytes)),
            Ok(basic)
        );
        for grade in ShieldGrade::ALL.into_iter().skip(1) {
            let mut pools = basic;
            pools.refit(grade);
            pools.absorb(false, 1);
            let mut writer = Writer::new();
            pools.encode(&mut writer);
            assert_eq!(writer.bytes.len(), 11);
            assert_eq!(
                ShieldPools::decode(&mut Reader::new(&writer.bytes)),
                Ok(pools)
            );
            assert!(ShieldPools::decode(&mut Reader::new(&writer.bytes[..10])).is_err());
            writer.bytes[2] = 0;
            assert!(ShieldPools::decode(&mut Reader::new(&writer.bytes)).is_err());
            writer.bytes[2] = 7;
            assert!(ShieldPools::decode(&mut Reader::new(&writer.bytes)).is_err());
        }
        let invalid = ShieldPools {
            grade: ShieldGrade::Advanced,
            banks: [69_001, 0],
        };
        assert!(!invalid.valid());
    }
}
