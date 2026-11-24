//! A song's levels: 4096 peaks and RMS values on a linear 0..1 scale, kept as bytes (0..255).

/// Parts a song is cut into, whatever its length.
pub const BINS: usize = 4096;
/// Samples (all channels) summed into one block at first.
const BLOCK: u32 = 512;
/// Blocks kept at most (16 bytes each: 1 MB). When full, neighbours merge and blocks grow twice as long, so a
/// song of any length, rate or channel count decodes in bounded memory, always with >= 8 blocks per part.
const MAX_BLOCKS: usize = 1 << 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Levels {
    pub peaks: Vec<u8>,
    pub rms: Vec<u8>, // each <= its peak
}

/// Takes the decoded samples as they come (any chunking) and cuts them into `BINS` parts at the end, so the length
/// need not be known up front.
pub struct Accumulator {
    blocks: Vec<(f32, f64, u32)>, // peak, sum of squares, samples
    cur: (f32, f64, u32),
    block: u32, // samples per block now
    cap: usize,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Accumulator {
    pub fn new() -> Self {
        Self::with_cap(MAX_BLOCKS)
    }

    fn with_cap(cap: usize) -> Self {
        Accumulator { blocks: Vec::with_capacity(cap), cur: (0.0, 0.0, 0), block: BLOCK, cap }
    }

    /// Halves the blocks: each pair becomes one twice as long.
    fn merge(&mut self) {
        let half = self.blocks.len().div_ceil(2);
        for i in 0..half {
            let a = self.blocks[2 * i];
            let b = self.blocks.get(2 * i + 1).copied().unwrap_or_default();
            self.blocks[i] = (a.0.max(b.0), a.1 + b.1, a.2 + b.2);
        }
        self.blocks.truncate(half); // in place: no allocation
        self.block *= 2;
    }

    /// Interleaved samples; a frame may be split across calls.
    pub fn push(&mut self, samples: &[f32]) {
        for &x in samples {
            let x = if x.is_finite() { x.abs() } else { 0.0 };
            self.cur.0 = self.cur.0.max(x);
            self.cur.1 += f64::from(x) * f64::from(x);
            self.cur.2 += 1;
            if self.cur.2 >= self.block {
                self.blocks.push(std::mem::take(&mut self.cur));
                if self.blocks.len() == self.cap {
                    self.merge();
                }
            }
        }
    }

    /// None when no sample came.
    pub fn finish(mut self) -> Option<Levels> {
        if self.cur.2 > 0 {
            self.blocks.push(self.cur);
        }
        let n = self.blocks.len();
        if n == 0 {
            return None;
        }
        let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        let (mut peaks, mut rms) = (Vec::with_capacity(BINS), Vec::with_capacity(BINS));
        for i in 0..BINS {
            let lo = i * n / BINS;
            let hi = ((i + 1) * n / BINS).max(lo + 1); // a song shorter than BINS blocks repeats blocks
            let (mut p, mut sq, mut c) = (0f32, 0f64, 0u64);
            for b in &self.blocks[lo..hi] {
                p = p.max(b.0);
                sq += b.1;
                c += u64::from(b.2);
            }
            peaks.push(byte(f64::from(p)));
            rms.push(byte((sq / c as f64).sqrt()));
        }
        Some(Levels { peaks, rms })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(frames: &[f32]) -> Levels {
        let mut a = Accumulator::new();
        for c in frames.chunks(4096) {
            a.push(c);
        }
        a.finish().unwrap()
    }

    #[test]
    fn full_scale_square_is_255_peak_and_rms() {
        let f: Vec<f32> = (0..BINS * 256 * 2).map(|i| if (i / 2) % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let l = run(&f);
        assert_eq!(l.peaks.len(), BINS);
        assert!(l.peaks.iter().all(|&p| p == 255));
        assert!(l.rms.iter().all(|&r| r == 255));
    }

    #[test]
    fn half_sine_peak_and_rms() {
        let f: Vec<f32> = (0..48_000 * 10)
            .flat_map(|i| {
                let x = 0.5 * (i as f32 * 2.0 * std::f32::consts::PI * 1000.0 / 48_000.0).sin();
                [x, x]
            })
            .collect();
        let l = run(&f);
        assert!(l.peaks.iter().all(|&p| (126..=128).contains(&p)), "{:?}", &l.peaks[..8]);
        assert!(l.rms.iter().all(|&r| (89..=91).contains(&r)), "{:?}", &l.rms[..8]);
    }

    #[test]
    fn rms_never_exceeds_peak() {
        let mut s: u32 = 1;
        let f: Vec<f32> = (0..300_000)
            .map(|_| {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (s >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
            })
            .collect();
        let l = run(&f);
        assert!(l.peaks.iter().zip(&l.rms).all(|(p, r)| r <= p));
    }

    #[test]
    fn short_audio_fills_every_bin() {
        let l = run(&[0.25; 2000]); // 1000 stereo frames: four blocks, the last one short
        assert!(l.peaks.iter().all(|&p| p == 64), "{:?}", &l.peaks[..8]);
        assert!(l.rms.iter().all(|&r| r == 64));
    }

    #[test]
    fn memory_stays_bounded() {
        let mut a = Accumulator::with_cap(8);
        for i in 0..100 {
            a.push(&[if i < 50 { 1.0 } else { 0.5 }; 512]);
            assert!(a.blocks.len() <= 8, "{} blocks", a.blocks.len());
        }
        let l = a.finish().unwrap();
        assert_eq!((l.peaks[0], l.rms[0]), (255, 255));
        assert_eq!((l.peaks[BINS - 1], l.rms[BINS - 1]), (128, 128));
        assert!(l.peaks.iter().zip(&l.rms).all(|(p, r)| r <= p));
    }

    #[test]
    fn no_frames_is_none() {
        assert!(Accumulator::new().finish().is_none());
    }
}
