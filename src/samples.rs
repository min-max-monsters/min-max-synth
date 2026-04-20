//! Embedded low-bit-rate percussion samples.
//!
//! Each sample is stored as signed 8-bit PCM at 11025 Hz to keep the binary
//! small while sounding appropriately crunchy. They are generated
//! procedurally on first use — no external assets required.

use std::sync::OnceLock;

/// Sample rate of every embedded sample.
pub const SAMPLE_RATE: f32 = 11_025.0;

/// A single percussion sample (8-bit signed PCM, mono).
#[derive(Debug)]
pub struct DrumSample {
    pub name: &'static str,
    /// Original MIDI note this sample is "tuned" to. Pitch shifting is done
    /// by playback rate.
    pub root_note: u8,
    pub data: Vec<i8>,
}

impl DrumSample {
    /// Length in samples.
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// All embedded drum samples, indexed by `DrumKind`.
pub fn samples() -> &'static [DrumSample] {
    static CELL: OnceLock<Vec<DrumSample>> = OnceLock::new();
    CELL.get_or_init(|| {
        vec![
            kick(),
            snare(),
            hat_closed(),
            hat_open(),
            tom(),
            clap(),
            cowbell(),
            zap(),
        ]
    })
}

/// Stable index per drum (matches order in `samples()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DrumKind {
    Kick = 0,
    Snare = 1,
    HatClosed = 2,
    HatOpen = 3,
    Tom = 4,
    Clap = 5,
    Cowbell = 6,
    Zap = 7,
}

impl DrumKind {
    pub const ALL: [DrumKind; 8] = [
        DrumKind::Kick,
        DrumKind::Snare,
        DrumKind::HatClosed,
        DrumKind::HatOpen,
        DrumKind::Tom,
        DrumKind::Clap,
        DrumKind::Cowbell,
        DrumKind::Zap,
    ];
    pub fn label(self) -> &'static str {
        samples()[self as usize].name
    }
}

// ---------------------------------------------------------------------------
// Procedural sample generators (1-shot, < 0.5s each).
// ---------------------------------------------------------------------------

fn pcm(len: usize, mut f: impl FnMut(usize) -> f32) -> Vec<i8> {
    (0..len)
        .map(|i| {
            let v = f(i).clamp(-1.0, 1.0);
            (v * 127.0) as i8
        })
        .collect()
}

fn kick() -> DrumSample {
    let len = (SAMPLE_RATE * 0.30) as usize;
    let mut phase = 0.0_f32;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        // Pitch sweep 120Hz -> 45Hz exponentially.
        let f = 45.0 + (120.0 - 45.0) * (-t * 25.0).exp();
        phase += f / SAMPLE_RATE;
        let env = (-t * 8.0).exp();
        (phase * std::f32::consts::TAU).sin() * env
    });
    DrumSample { name: "Kick", root_note: 60, data }
}

fn snare() -> DrumSample {
    let len = (SAMPLE_RATE * 0.20) as usize;
    let mut lfsr: u16 = 1;
    let mut tone_phase = 0.0_f32;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        // Step LFSR.
        let bit = ((lfsr ^ (lfsr >> 1)) & 1) as u16;
        lfsr = (lfsr >> 1) | (bit << 14);
        let noise = if (lfsr & 1) == 0 { 1.0 } else { -1.0 };
        tone_phase += 200.0 / SAMPLE_RATE;
        let tone = (tone_phase * std::f32::consts::TAU).sin();
        let env = (-t * 18.0).exp();
        (noise * 0.7 + tone * 0.5) * env
    });
    DrumSample { name: "Snare", root_note: 60, data }
}

fn hat(decay: f32, name: &'static str) -> DrumSample {
    let len = (SAMPLE_RATE * (0.05 + decay * 0.02)) as usize;
    let mut lfsr: u16 = 0xACE1;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        let bit = ((lfsr ^ (lfsr >> 6)) & 1) as u16;
        lfsr = (lfsr >> 1) | (bit << 14);
        let noise = if (lfsr & 1) == 0 { 1.0 } else { -1.0 };
        let env = (-t * decay).exp();
        noise * env * 0.7
    });
    DrumSample { name, root_note: 60, data }
}

fn hat_closed() -> DrumSample {
    hat(60.0, "Hat (Closed)")
}

fn hat_open() -> DrumSample {
    let mut s = hat(8.0, "Hat (Open)");
    // Make it a bit longer.
    let extra = (SAMPLE_RATE * 0.25) as usize;
    let mut lfsr: u16 = 0xBEEF;
    for i in 0..extra {
        let t = (s.data.len() + i) as f32 / SAMPLE_RATE;
        let bit = ((lfsr ^ (lfsr >> 6)) & 1) as u16;
        lfsr = (lfsr >> 1) | (bit << 14);
        let noise = if (lfsr & 1) == 0 { 1.0 } else { -1.0 };
        let env = (-t * 6.0).exp();
        let v = (noise * env * 0.6).clamp(-1.0, 1.0);
        s.data.push((v * 127.0) as i8);
    }
    s
}

fn tom() -> DrumSample {
    let len = (SAMPLE_RATE * 0.30) as usize;
    let mut phase = 0.0_f32;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        let f = 90.0 + (180.0 - 90.0) * (-t * 12.0).exp();
        phase += f / SAMPLE_RATE;
        let env = (-t * 6.0).exp();
        (phase * std::f32::consts::TAU).sin() * env
    });
    DrumSample { name: "Tom", root_note: 60, data }
}

fn clap() -> DrumSample {
    let len = (SAMPLE_RATE * 0.30) as usize;
    let mut lfsr: u16 = 0x1234;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        let bit = ((lfsr ^ (lfsr >> 2)) & 1) as u16;
        lfsr = (lfsr >> 1) | (bit << 14);
        let noise = if (lfsr & 1) == 0 { 1.0 } else { -1.0 };
        // Three quick bursts then a tail.
        let env = if t < 0.01 {
            1.0
        } else if t < 0.02 {
            0.4
        } else if t < 0.03 {
            0.9
        } else if t < 0.04 {
            0.4
        } else if t < 0.05 {
            0.8
        } else {
            (-(t - 0.05) * 12.0).exp()
        };
        noise * env * 0.8
    });
    DrumSample { name: "Clap", root_note: 60, data }
}

fn cowbell() -> DrumSample {
    let len = (SAMPLE_RATE * 0.18) as usize;
    let mut p1 = 0.0_f32;
    let mut p2 = 0.0_f32;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        p1 += 540.0 / SAMPLE_RATE;
        p2 += 800.0 / SAMPLE_RATE;
        if p1 >= 1.0 { p1 -= 1.0 }
        if p2 >= 1.0 { p2 -= 1.0 }
        let s1 = if p1 < 0.5 { 1.0 } else { -1.0 };
        let s2 = if p2 < 0.5 { 1.0 } else { -1.0 };
        let env = (-t * 12.0).exp();
        (s1 * 0.5 + s2 * 0.5) * env * 0.7
    });
    DrumSample { name: "Cowbell", root_note: 60, data }
}

fn zap() -> DrumSample {
    let len = (SAMPLE_RATE * 0.25) as usize;
    let mut phase = 0.0_f32;
    let data = pcm(len, |i| {
        let t = i as f32 / SAMPLE_RATE;
        // Fast downward sweep.
        let f = 1500.0 * (-t * 10.0).exp() + 80.0;
        phase += f / SAMPLE_RATE;
        let s = if phase.fract() < 0.5 { 1.0 } else { -1.0 };
        let env = (-t * 6.0).exp();
        s * env
    });
    DrumSample { name: "Zap", root_note: 60, data }
}

// ---------------------------------------------------------------------------
// Sample player
// ---------------------------------------------------------------------------

/// Plays back a `DrumSample` with linear pitch-rate adjustment.
#[derive(Debug, Default, Clone)]
pub struct SamplePlayer {
    kind: Option<DrumKind>,
    pos: f32,
    rate: f32,
}

impl SamplePlayer {
    pub fn trigger(&mut self, kind: DrumKind, playback_rate: f32) {
        self.kind = Some(kind);
        self.pos = 0.0;
        self.rate = playback_rate.max(0.001);
    }

    pub fn is_active(&self) -> bool {
        self.kind.is_some()
    }

    #[inline]
    pub fn tick(&mut self, host_sample_rate: f32) -> f32 {
        let Some(kind) = self.kind else { return 0.0 };
        let s = &samples()[kind as usize];
        let i = self.pos as usize;
        if i >= s.data.len() {
            self.kind = None;
            return 0.0;
        }
        let v = s.data[i] as f32 / 127.0;
        // Step position: source is at SAMPLE_RATE, host at host_sample_rate.
        self.pos += (SAMPLE_RATE / host_sample_rate) * self.rate;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_samples_nonempty() {
        for s in samples() {
            assert!(!s.is_empty(), "{} is empty", s.name);
        }
    }

    #[test]
    fn player_finishes() {
        let mut p = SamplePlayer::default();
        p.trigger(DrumKind::Kick, 1.0);
        for _ in 0..100_000 {
            p.tick(48_000.0);
            if !p.is_active() {
                return;
            }
        }
        panic!("sample player never finished");
    }
}
