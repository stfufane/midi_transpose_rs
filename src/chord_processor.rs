use std::{
    collections::HashSet,
    sync::{Arc, Mutex, RwLock},
};

use nih_plug::{nih_dbg, nih_trace, plugin::ProcessStatus};

use crate::{note_info::NoteInfo, params::MidiTransposerParams, MidiProcessor, NotesState};

pub(crate) struct ChordProcessor {
    chord_playing: bool, // Whether the chord is currently playing.
}

impl MidiProcessor for ChordProcessor {
    fn process(&mut self, notes_state: NotesState, _nb_samples: usize) -> ProcessStatus {
        if notes_state.notes.is_empty() {
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
    pub(crate) fn build_chord(
        params: Arc<MidiTransposerParams>,
        note_info: &NoteInfo,
    ) -> Vec<NoteInfo> {
        let mut chord: Vec<NoteInfo> = Vec::with_capacity(12);
        let base_note = note_info.note % 12;
        // Exit if the transposition is deactivated for this note.
        if !params.notes[base_note as usize].active.value() {
            // Just play the base note.
            chord.push(*note_info);
            return chord;
        }

        // Create a copy of the note info to map with the transposition.
        let mut mapped_note_info = *note_info;
        let octave_transpose = params.octave_transpose.value() as u8;
        let note_transpose = params.notes[base_note as usize].transpose.value() as i8;

        // Include the base note at its original octave if there's an octave transpose.
        if octave_transpose != 0 {
            chord.push(*mapped_note_info.transposed(note_transpose));
        }

        // Map the intervals that are not 0 and remove identicals.
        let mut intervals: HashSet<i32> = params.notes[base_note as usize]
            .intervals
            .iter()
            .filter_map(|interval_param| {
                if interval_param.interval.value() > 0 {
                    Some(interval_param.interval.value())
                } else {
                    None
                }
            })
            .collect();
        // Add interval 0 for the base note that is played all the time.
        intervals.insert(0);

        // Map the intervals to the note info and build the chord.
        for interval in intervals {
            let mapped_note =
                (mapped_note_info.note as i32 + octave_transpose as i32 * 12 + interval) as u8;
            if mapped_note > 127 {
                continue;
            }
            chord.push(NoteInfo::new(
                mapped_note,
                note_info.channel,
                note_info.velocity,
                note_info.timing,
            ));
        }
        chord
    }
}
