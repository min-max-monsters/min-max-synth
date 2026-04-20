//! Standalone runner: launches the synth in its own window with a built-in
//! audio backend so you can play it with the QWERTY keyboard without a host.

use min_max_synth::MinMaxSynth;
use nih_plug::prelude::*;

fn main() {
    nih_export_standalone::<MinMaxSynth>();
}
