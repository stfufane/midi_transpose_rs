use std::sync::Arc;

use nih_plug::{context::process::ProcessContext, plugin::ProcessStatus};

use crate::{params::ArpParams, MidiProcessor, MidiTransposer, NotesState};

pub(crate) struct ArpProcessor {
    params: Arc<ArpParams>,
    notes: Vec<u8>,
    current_index: usize, // The position in the arpeggiated chord.
    synced: bool,
    pub(crate) sample_rate: f32,
    division: f64,
    next_beat_position: f64,
    time: u32,
}

impl ArpProcessor {
    pub fn new(params: Arc<ArpParams>) -> Self {
        Self {
            params,
            notes: Vec::with_capacity(8),
            current_index: 0,
            synced: false,
            sample_rate: 44100.0,
            division: 1.0,
            next_beat_position: 0.0,
            time: 0,
        }
    }

    pub fn reset(&mut self) {
        self.notes.clear();
        self.current_index = 0;
        self.next_beat_position = 0.0;
        self.time = 0;
    }

    pub fn process_free(
        context: &mut impl ProcessContext<MidiTransposer>,
        notes_state: &NotesState,
        nb_samples: usize,
    ) {
    }
}

impl MidiProcessor for ArpProcessor {
    fn process(
        &mut self,
        _context: &mut impl ProcessContext<MidiTransposer>,
        _notes_state: &NotesState,
        _nb_samples: usize,
    ) -> ProcessStatus {
        ProcessStatus::Normal
    }

    fn arp_toggled(
        &mut self,
        _context: &mut impl ProcessContext<MidiTransposer>,
        on_off: bool,
        notes_state: &NotesState,
    ) {
        if on_off {
            // Just reconstruct the chord, the notes will be handled in the next call to process.
            if let Some(current_chord) = &notes_state.current_chord {
                for i in 0..128 {
                    if current_chord.notes & (1 << i) != 0 {
                        self.notes.push(i as u8);
                    }
                }
            }
        } else {
            // Turn off the current note.
            if !self.notes.is_empty() {
                if let Some(current_note) = notes_state.current_note_held {
                    let last_note = self.notes[self.current_index];
                    // TODO when arp processing is implemented.
                    // context.send_event(NoteEvent::NoteOff {
                    //     note: last_note,
                    //     channel: current_note.channel,
                    //     velocity: 0.0,
                    //     voice_id: None,
                    //     timing: 0,
                    // });
                }
            }
            // Reinitialize all the internal values.
            self.reset();
        }
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
