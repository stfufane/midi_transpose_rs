use std::{ops::AddAssign, sync::Arc};

use nih_plug::{nih_dbg, nih_trace, plugin::ProcessStatus};

use crate::{note_info::NoteInfo, params::MidiTransposerParams, MidiProcessor, NotesState};

pub(crate) struct ChordProcessor {
    chord_playing: bool, // Whether the chord is currently playing.
}

impl MidiProcessor for ChordProcessor {
    fn process(&mut self, notes_state: &NotesState, _nb_samples: usize) -> ProcessStatus {
        if notes_state.notes_held.is_empty() {
            return ProcessStatus::Normal;
        }
        nih_trace!("ChordProcessor::process: {:?}", notes_state.current_chord);
        nih_dbg!(notes_state.current_note_held);
        ProcessStatus::Normal
    }

    fn arp_reset(&mut self, _on_off: bool) {
        // todo!("Implement the arp reset")
    }
}

impl Default for ChordProcessor {
    fn default() -> Self {
        Self {
            chord_playing: false,
        }
    }
}

impl ChordProcessor {
    pub(crate) fn build_chord(params: Arc<MidiTransposerParams>, note_info: &NoteInfo) -> u128 {
        let mut chord: u128 = 0;
        let base_note = note_info.note % 12;

        // Exit if the transposition is deactivated for this note.
        if !params.notes[base_note as usize].active.value() {
            // Just play the base note.
            chord.add_assign(1 << note_info.note);
            return chord;
        }

        // Create a copy of the note info to map with the transposition.
        let octave_transpose = params.octave_transpose.value() as u8;
        let note_transpose = params.notes[base_note as usize].transpose.value() as i8;

        let mapped_note_info = note_info.with_transposition(note_transpose);

        // Include the base note at its original octave if there's an octave transpose.
        if octave_transpose != 0 {
            chord.add_assign(1 << mapped_note_info.note);
        }
        // Also include the base note at the transposed octave.
        chord.add_assign(1 << (mapped_note_info.note + 12 * octave_transpose));

        params.notes[base_note as usize]
            .intervals
            .iter()
            .map(|interval_param| {
                (mapped_note_info.note as i32
                    + octave_transpose as i32 * 12
                    + interval_param.interval.value()) as u8
            })
            .filter(|note| *note < 128)
            .for_each(|note| {
                chord.add_assign(1 << note);
            });

        chord
    }
}
