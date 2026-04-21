//! Preset bank with metadata tags, factory presets, and user preset I/O.
//!
//! Factory presets are loaded from the embedded `factory_presets.json`.
//! User presets are stored as JSON files under a `presets/` directory next
//! to the plugin data path.

use crate::params::{LegatoMode, SynthParams, WaveChoice};
use nih_plug::prelude::{Param, ParamSetter};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Tag enums
// ---------------------------------------------------------------------------

/// Hardware system / era.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum System {
    NES,
    Gameboy,
    SNES,
    Genesis,
    SID,
    Amiga,
    PCSpk,
    Tandy,
    Atari2600,
    MSX,
    Spectrum,
    Arcade,
    Generic,
}

impl System {
    pub const ALL: &[Self] = &[
        Self::NES,
        Self::Gameboy,
        Self::SNES,
        Self::Genesis,
        Self::SID,
        Self::Amiga,
        Self::PCSpk,
        Self::Tandy,
        Self::Atari2600,
        Self::MSX,
        Self::Spectrum,
        Self::Arcade,
        Self::Generic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::NES => "NES",
            Self::Gameboy => "GB",
            Self::SNES => "SNES",
            Self::Genesis => "GEN",
            Self::SID => "SID",
            Self::Amiga => "AMIGA",
            Self::PCSpk => "PCSPK",
            Self::Tandy => "TANDY",
            Self::Atari2600 => "2600",
            Self::MSX => "MSX",
            Self::Spectrum => "ZX",
            Self::Arcade => "ARC",
            Self::Generic => "ANY",
        }
    }
}

/// Sound category / role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Lead,
    Bass,
    Pad,
    Keys,
    Pluck,
    Arp,
    SFX,
    DrumKit,
    Brass,
    Strings,
    Organ,
    Bell,
    Noise,
}

impl Category {
    pub const ALL: &[Self] = &[
        Self::Lead,
        Self::Bass,
        Self::Pad,
        Self::Keys,
        Self::Pluck,
        Self::Arp,
        Self::SFX,
        Self::DrumKit,
        Self::Brass,
        Self::Strings,
        Self::Organ,
        Self::Bell,
        Self::Noise,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lead => "LEAD",
            Self::Bass => "BASS",
            Self::Pad => "PAD",
            Self::Keys => "KEYS",
            Self::Pluck => "PLUCK",
            Self::Arp => "ARP",
            Self::SFX => "SFX",
            Self::DrumKit => "DRUMS",
            Self::Brass => "BRASS",
            Self::Strings => "STRINGS",
            Self::Organ => "ORGAN",
            Self::Bell => "BELL",
            Self::Noise => "NOISE",
        }
    }
}

/// Voicing / play style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Voicing {
    Poly,
    Mono,
    Arp,
}

impl Voicing {
    pub const ALL: &[Self] = &[Self::Poly, Self::Mono, Self::Arp];

    pub fn label(self) -> &'static str {
        match self {
            Self::Poly => "POLY",
            Self::Mono => "MONO",
            Self::Arp => "ARP",
        }
    }
}

// ---------------------------------------------------------------------------
// Preset metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetMeta {
    pub system: System,
    pub category: Category,
    pub voicing: Voicing,
}

// ---------------------------------------------------------------------------
// Param snapshot — all automatable values captured as plain floats.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSnapshot {
    pub gain: f32,
    pub waveform: i32,
    pub pulse_duty: f32,
    pub noise_short: bool,
    pub fm_ratio: f32,
    pub fm_index: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub duty_lfo_rate: f32,
    pub duty_lfo_depth: f32,
    pub vibrato_rate: f32,
    pub vibrato_depth: f32,
    pub vibrato_delay: f32,
    pub sweep_semi: f32,
    pub sweep_time: f32,
    pub mono: bool,
    pub arp_rate: f32,
    /// Mono note-transition behaviour. Stored as the enum's `#[id]` string
    /// so the JSON stays human-readable. Defaults to "retrig" (the
    /// pre-legato behaviour) for backward compatibility with existing
    /// presets.
    #[serde(default = "default_legato_mode")]
    pub legato_mode: i32,
    /// Glide time in milliseconds (matches the parameter's display unit).
    #[serde(default = "default_glide_time")]
    pub glide_time: f32,
    pub bit_depth: f32,
    pub bit_rate: f32,
    pub lp_cutoff: f32,
    pub hp_cutoff: f32,
    pub fine_tune: f32,
    pub octave: i32,
    pub drum_mode: bool,
    pub drum_pitch: bool,
    #[serde(default)]
    pub speech_mode: bool,
    #[serde(default)]
    pub phoneme: i32,
    #[serde(default)]
    pub speech_buzz: f32,
    #[serde(default)]
    pub speech_seq_len: i32,
    #[serde(default = "default_speech_step_ms")]
    pub speech_step_ms: f32,
    #[serde(default)]
    pub speech_seq_loop: bool,
    #[serde(default)]
    pub speech_seq: Vec<i32>,
    // Per-drum slots (8 × 9 params = 72 values)
    pub drums: Vec<DrumSlotSnapshot>,
}

fn default_speech_step_ms() -> f32 {
    120.0
}

fn default_legato_mode() -> i32 {
    0 // LegatoMode::Retrigger
}

fn default_glide_time() -> f32 {
    60.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumSlotSnapshot {
    pub wave: i32,
    pub freq: f32,
    pub ratio: f32,
    pub noise: f32,
    pub pitch_env: f32,
    pub pitch_time: f32,
    pub decay: f32,
    pub burst: f32,
    pub level: f32,
}

impl ParamSnapshot {
    /// Capture current parameter values into a snapshot.
    /// Uses `unmodulated_plain_value()` to read the target value (not the
    /// smoothed/in-flight value), which is what `ParamSetter::set_parameter`
    /// writes to.
    pub fn capture(p: &SynthParams) -> Self {
        let mut drums = Vec::with_capacity(8);
        for i in 0..8 {
            drums.push(DrumSlotSnapshot {
                wave: p.d_wave(i).unmodulated_plain_value(),
                freq: p.d_freq(i).unmodulated_plain_value(),
                ratio: p.d_ratio(i).unmodulated_plain_value(),
                noise: p.d_noise(i).unmodulated_plain_value(),
                pitch_env: p.d_pitch_env(i).unmodulated_plain_value(),
                pitch_time: p.d_pitch_time(i).unmodulated_plain_value(),
                decay: p.d_decay(i).unmodulated_plain_value(),
                burst: p.d_burst(i).unmodulated_plain_value(),
                level: p.d_level(i).unmodulated_plain_value(),
            });
        }
        Self {
            gain: p.gain.unmodulated_plain_value(),
            waveform: p.waveform.unmodulated_plain_value() as i32,
            pulse_duty: p.pulse_duty.unmodulated_plain_value(),
            noise_short: p.noise_short.unmodulated_plain_value(),
            fm_ratio: p.fm_ratio.unmodulated_plain_value(),
            fm_index: p.fm_index.unmodulated_plain_value(),
            attack: p.attack.unmodulated_plain_value(),
            decay: p.decay.unmodulated_plain_value(),
            sustain: p.sustain.unmodulated_plain_value(),
            release: p.release.unmodulated_plain_value(),
            duty_lfo_rate: p.duty_lfo_rate.unmodulated_plain_value(),
            duty_lfo_depth: p.duty_lfo_depth.unmodulated_plain_value(),
            vibrato_rate: p.vibrato_rate.unmodulated_plain_value(),
            vibrato_depth: p.vibrato_depth.unmodulated_plain_value(),
            vibrato_delay: p.vibrato_delay.unmodulated_plain_value(),
            sweep_semi: p.sweep_semi.unmodulated_plain_value(),
            sweep_time: p.sweep_time.unmodulated_plain_value(),
            mono: p.mono.unmodulated_plain_value(),
            arp_rate: p.arp_rate.unmodulated_plain_value(),
            legato_mode: p.legato_mode.unmodulated_plain_value() as i32,
            glide_time: p.glide_time.unmodulated_plain_value(),
            bit_depth: p.bit_depth.unmodulated_plain_value(),
            bit_rate: p.bit_rate.unmodulated_plain_value(),
            lp_cutoff: p.lp_cutoff.unmodulated_plain_value(),
            hp_cutoff: p.hp_cutoff.unmodulated_plain_value(),
            fine_tune: p.fine_tune.unmodulated_plain_value(),
            octave: p.octave.unmodulated_plain_value(),
            drum_mode: p.drum_mode.unmodulated_plain_value(),
            drum_pitch: p.drum_pitch.unmodulated_plain_value(),
            speech_mode: p.speech_mode.unmodulated_plain_value(),
            phoneme: p.phoneme.unmodulated_plain_value(),
            speech_buzz: p.speech_buzz.unmodulated_plain_value(),
            speech_seq_len: p.speech_seq_len.unmodulated_plain_value(),
            speech_step_ms: p.speech_step_ms.unmodulated_plain_value(),
            speech_seq_loop: p.speech_seq_loop.unmodulated_plain_value(),
            speech_seq: (0..8).map(|i| p.sq(i).unmodulated_plain_value()).collect(),
            drums,
        }
    }

    /// Apply snapshot values to the plugin parameters.
    pub fn apply(&self, p: &SynthParams, s: &ParamSetter) {
        s.set_parameter(&p.gain, self.gain);
        // Waveform: map i32 back to enum variant via the enum index
        let wave_variants = [
            WaveChoice::Pulse,
            WaveChoice::Triangle,
            WaveChoice::Wave,
            WaveChoice::Noise,
            WaveChoice::Fm,
            WaveChoice::Saw,
        ];
        let wave = wave_variants.get(self.waveform as usize).copied().unwrap_or(WaveChoice::Pulse);
        s.set_parameter(&p.waveform, wave);
        s.set_parameter(&p.pulse_duty, self.pulse_duty);
        s.set_parameter(&p.noise_short, self.noise_short);
        s.set_parameter(&p.fm_ratio, self.fm_ratio);
        s.set_parameter(&p.fm_index, self.fm_index);
        s.set_parameter(&p.attack, self.attack);
        s.set_parameter(&p.decay, self.decay);
        s.set_parameter(&p.sustain, self.sustain);
        s.set_parameter(&p.release, self.release);
        s.set_parameter(&p.duty_lfo_rate, self.duty_lfo_rate);
        s.set_parameter(&p.duty_lfo_depth, self.duty_lfo_depth);
        s.set_parameter(&p.vibrato_rate, self.vibrato_rate);
        s.set_parameter(&p.vibrato_depth, self.vibrato_depth);
        s.set_parameter(&p.vibrato_delay, self.vibrato_delay);
        s.set_parameter(&p.sweep_semi, self.sweep_semi);
        s.set_parameter(&p.sweep_time, self.sweep_time);
        s.set_parameter(&p.mono, self.mono);
        s.set_parameter(&p.arp_rate, self.arp_rate);
        let legato_variants = [LegatoMode::Retrigger, LegatoMode::Legato, LegatoMode::Glide];
        let lm = legato_variants
            .get(self.legato_mode as usize)
            .copied()
            .unwrap_or(LegatoMode::Retrigger);
        s.set_parameter(&p.legato_mode, lm);
        s.set_parameter(&p.glide_time, self.glide_time);
        s.set_parameter(&p.bit_depth, self.bit_depth);
        s.set_parameter(&p.bit_rate, self.bit_rate);
        s.set_parameter(&p.lp_cutoff, self.lp_cutoff);
        s.set_parameter(&p.hp_cutoff, self.hp_cutoff);
        s.set_parameter(&p.fine_tune, self.fine_tune);
        s.set_parameter(&p.octave, self.octave);
        s.set_parameter(&p.drum_mode, self.drum_mode);
        s.set_parameter(&p.drum_pitch, self.drum_pitch);
        s.set_parameter(&p.speech_mode, self.speech_mode);
        s.set_parameter(&p.phoneme, self.phoneme);
        s.set_parameter(&p.speech_buzz, self.speech_buzz);
        s.set_parameter(&p.speech_seq_len, self.speech_seq_len);
        s.set_parameter(&p.speech_step_ms, self.speech_step_ms);
        s.set_parameter(&p.speech_seq_loop, self.speech_seq_loop);
        for (i, &ph) in self.speech_seq.iter().take(8).enumerate() {
            s.set_parameter(p.sq(i), ph);
        }
        for (i, d) in self.drums.iter().enumerate().take(8) {
            s.set_parameter(p.d_wave(i), d.wave);
            s.set_parameter(p.d_freq(i), d.freq);
            s.set_parameter(p.d_ratio(i), d.ratio);
            s.set_parameter(p.d_noise(i), d.noise);
            s.set_parameter(p.d_pitch_env(i), d.pitch_env);
            s.set_parameter(p.d_pitch_time(i), d.pitch_time);
            s.set_parameter(p.d_decay(i), d.decay);
            s.set_parameter(p.d_burst(i), d.burst);
            s.set_parameter(p.d_level(i), d.level);
        }
    }
}

// ---------------------------------------------------------------------------
// Preset entry — either factory (index-based) or user (snapshot-based).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PresetEntry {
    pub name: String,
    pub meta: PresetMeta,
    pub snapshot: ParamSnapshot,
    pub is_factory: bool,
}

// ---------------------------------------------------------------------------
// User preset file format
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresetFile {
    pub name: String,
    pub meta: PresetMeta,
    pub params: ParamSnapshot,
}

// ---------------------------------------------------------------------------
// Preset bank — holds all presets and provides filtering
// ---------------------------------------------------------------------------

pub struct PresetBank {
    pub entries: Vec<PresetEntry>,
    /// Indices into `entries` that pass the current filter.
    pub filtered: Vec<usize>,
    /// Currently selected index into `filtered`.
    pub selected: usize,
    // Filters (None = show all)
    pub filter_system: Option<System>,
    pub filter_category: Option<Category>,
    pub filter_voicing: Option<Voicing>,
    pub show_factory: bool,
    pub show_user: bool,
    pub search_text: String,
}

impl PresetBank {
    pub fn new() -> Self {
        let entries = build_factory_presets();
        let filtered: Vec<usize> = (0..entries.len()).collect();
        Self {
            entries,
            filtered,
            selected: 0,
            filter_system: None,
            filter_category: None,
            filter_voicing: None,
            show_factory: true,
            show_user: true,
            search_text: String::new(),
        }
    }

    /// Load user presets from disk and append them.
    pub fn load_user_presets(&mut self) {
        let dir = user_preset_dir();
        if !dir.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(data) = std::fs::read_to_string(&path) else { continue };
            let Ok(file) = serde_json::from_str::<UserPresetFile>(&data) else { continue };
            // Avoid duplicates by name
            if self.entries.iter().any(|e| !e.is_factory && e.name == file.name) {
                continue;
            }
            self.entries.push(PresetEntry {
                name: file.name,
                meta: file.meta,
                snapshot: file.params,
                is_factory: false,
            });
        }
        self.refilter();
    }

    /// Save a user preset to disk.
    pub fn save_user_preset(&mut self, name: &str, meta: PresetMeta, snap: ParamSnapshot) {
        let file = UserPresetFile {
            name: name.to_string(),
            meta: meta.clone(),
            params: snap.clone(),
        };
        let dir = user_preset_dir();
        let _ = std::fs::create_dir_all(&dir);
        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect();
        let path = dir.join(format!("{}.json", safe_name.trim()));
        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = std::fs::write(&path, json);
        }

        // Remove existing user entry with the same name
        self.entries.retain(|e| e.is_factory || e.name != name);
        self.entries.push(PresetEntry {
            name: name.to_string(),
            meta,
            snapshot: snap,
            is_factory: false,
        });
        self.refilter();
    }

    /// Delete a user preset (by entry index).
    pub fn delete_user_preset(&mut self, entry_idx: usize) {
        if entry_idx >= self.entries.len() || self.entries[entry_idx].is_factory {
            return;
        }
        let name = &self.entries[entry_idx].name;
        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect();
        let path = user_preset_dir().join(format!("{}.json", safe_name.trim()));
        let _ = std::fs::remove_file(&path);
        self.entries.remove(entry_idx);
        self.refilter();
    }

    /// Recompute `filtered` from current filter settings.
    pub fn refilter(&mut self) {
        let search_lower = self.search_text.to_ascii_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if e.is_factory && !self.show_factory {
                    return false;
                }
                if !e.is_factory && !self.show_user {
                    return false;
                }
                if let Some(sys) = self.filter_system {
                    if e.meta.system != sys {
                        return false;
                    }
                }
                if let Some(cat) = self.filter_category {
                    if e.meta.category != cat {
                        return false;
                    }
                }
                if let Some(voi) = self.filter_voicing {
                    if e.meta.voicing != voi {
                        return false;
                    }
                }
                if !search_lower.is_empty()
                    && !e.name.to_ascii_lowercase().contains(&search_lower)
                {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();
        // Keep selected in range
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    /// Get the currently selected entry (if any).
    pub fn current_entry(&self) -> Option<&PresetEntry> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Get the entry index in `entries` for the current selection.
    pub fn current_entry_idx(&self) -> Option<usize> {
        self.filtered.get(self.selected).copied()
    }

    /// Move to next preset in the filtered list.
    pub fn next(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    /// Move to previous preset in the filtered list.
    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

/// Directory for user presets.
fn user_preset_dir() -> PathBuf {
    dirs_next().join("presets")
}

/// Platform-appropriate config directory for the plugin.
fn dirs_next() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("min_max_synth")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\".into());
        PathBuf::from(appdata).join("min_max_synth")
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".config").join("min_max_synth")
    }
}

// ---------------------------------------------------------------------------
// Factory preset loading — all metadata + values live in factory_presets.json
// ---------------------------------------------------------------------------

/// Embedded factory presets JSON.
const FACTORY_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/factory_presets.json"));

fn build_factory_presets() -> Vec<PresetEntry> {
    let files: Vec<UserPresetFile> =
        serde_json::from_str(FACTORY_JSON).expect("parse embedded factory presets");
    files
        .into_iter()
        .map(|f| PresetEntry {
            name: f.name,
            meta: f.meta,
            snapshot: f.params,
            is_factory: true,
        })
        .collect()
}
