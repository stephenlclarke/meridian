//! Port of the independently generated Swift SoundEffect, reference a11fe169.
pub const SAMPLE_RATE: u32 = 22_050;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEffect {
    Laser,
    Launch,
    Dock,
    Explosion,
    Ping,
    Ecm,
}

impl SoundEffect {
    pub fn sample_count(self) -> usize {
        match self {
            Self::Laser => 2646,
            Self::Launch => 17640,
            Self::Dock => 13230,
            Self::Explosion => 7717,
            Self::Ping => 2205,
            Self::Ecm => 6615,
        }
    }

    fn frequency(self, progress: f64) -> f64 {
        match self {
            Self::Laser => 1100.0 - progress * 900.0,
            Self::Launch => 180.0 + progress * 800.0,
            Self::Dock => 900.0 - progress * 700.0,
            Self::Explosion => 60.0 + progress * 30.0,
            Self::Ping => 880.0,
            Self::Ecm => 600.0 + (progress * std::f64::consts::PI * 12.0).sin() * 300.0,
        }
    }

    pub fn pcm(self) -> Vec<i16> {
        let count = self.sample_count();
        let mut phase = 0.0;
        let mut noise = 0x454c_4954_u32;
        (0..count)
            .map(|index| {
                let progress = index as f64 / count as f64;
                phase += self.frequency(progress) / f64::from(SAMPLE_RATE);
                noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let positive = if self == Self::Explosion {
                    noise & 1 != 0
                } else {
                    phase % 1.0 >= 0.5
                };
                let sign = if positive { 1.0 } else { -1.0 };
                (sign * 5000.0 * (1.0 - progress)) as i16
            })
            .collect()
    }

    /// PCM16 mono RIFF bytes, including the original truncating envelope.
    pub fn wav(self) -> Vec<u8> {
        let samples = self.pcm();
        let byte_count = (samples.len() * 2) as u32;
        let mut bytes = Vec::with_capacity(44 + byte_count as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + byte_count).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&byte_count.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }
}
