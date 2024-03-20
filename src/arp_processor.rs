use std::sync::Arc;

use nih_plug::{nih_trace, plugin::ProcessStatus};

use crate::{note_info::NoteInfo, MidiProcessor, NotesState};

pub(crate) struct ArpProcessor {
    arp_playing: bool,        // Whether the arpeggio is currently playing.
    pub current_index: usize, // The position in the arpeggiated chord.
}

impl Default for ArpProcessor {
    fn default() -> Self {
        Self {
            arp_playing: false,
            current_index: 0,
        }
    }
}

impl MidiProcessor for ArpProcessor {
    fn process(&mut self, notes_state: NotesState, _nb_samples: usize) -> ProcessStatus {
        ProcessStatus::Normal
    }

    fn arp_reset(&mut self, _on_off: bool) {
        // todo!("Implement the arp reset")
    }
}

pub struct NoteDivision {
    label: &'static str,
    pub division: f64,
}

pub const NOTE_DIVISIONS: [NoteDivision; 9] = [
    NoteDivision {
        label: "1/1",
        division: 4.0,
    },
    NoteDivision {
        label: "1/2",
        division: 2.0,
    },
    NoteDivision {
        label: "1/4.d",
        division: 1.5,
    },
    NoteDivision {
        label: "1/4",
        division: 1.0,
    },
    NoteDivision {
        label: "1/8d",
        division: 0.75,
    },
    NoteDivision {
        label: "1/4.t",
        division: 2.0 / 3.0,
    },
    NoteDivision {
        label: "1/8",
        division: 0.5,
    },
    NoteDivision {
        label: "1/8.t",
        division: 1.0 / 3.0,
    },
    NoteDivision {
        label: "1/16",
        division: 0.25,
    },
];
