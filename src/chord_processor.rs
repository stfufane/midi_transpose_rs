use std::sync::Arc;

use nih_plug::{
    context::process::ProcessContext, midi::NoteEvent, nih_dbg, nih_trace, plugin::ProcessStatus,
};

use crate::{
    note_info::NoteInfo, params::MidiTransposerParams, MidiProcessor, MidiTransposer, NoteTrigger,
    NotesState,
};

#[derive(Default)]
pub(crate) struct ChordProcessor {}

impl MidiProcessor for ChordProcessor {
    fn process(
        &mut self,
        context: &mut impl ProcessContext<MidiTransposer>,
        notes_state: &NotesState,
        _nb_samples: usize,
    ) -> ProcessStatus {
        match &notes_state.trigger {
            Some(trigger) => match trigger {
                NoteTrigger::Play => {
                    if let Some(note_info) = notes_state.current_note_held {
                        if let Some(chord_to_stop) = notes_state.previous_chord {
                            for i in 0..128 {
                                if chord_to_stop & (1 << i) != 0 {
                                    context.send_event(NoteEvent::NoteOff {
                                        note: i as u8,
                                        channel: 0,
                                        velocity: 0.0,
                                        voice_id: None,
                                        timing: note_info.timing,
                                    });
                                }
                            }
                        }

                        if let Some(chord_to_play) = notes_state.current_chord {
                            for i in 0..128 {
                                if chord_to_play & (1 << i) != 0 {
                                    context.send_event(NoteEvent::NoteOn {
                                        note: i as u8,
                                        channel: 0,
                                        velocity: note_info.velocity,
                                        voice_id: None,
                                        timing: note_info.timing,
                                    });
                                }
                            }
                        }
                    }
                }
                NoteTrigger::Stop => {
                    if let Some(chord_to_stop) = notes_state.previous_chord {
                        for i in 0..128 {
                            if chord_to_stop & (1 << i) != 0 {
                                context.send_event(NoteEvent::NoteOff {
                                    note: i as u8,
                                    channel: 0,
                                    velocity: 0.0,
                                    voice_id: None,
                                    timing: 0,
                                });
                            }
                        }
                    }
                }
            },
            None => {
                // If there's no trigger, we don't have to do anything.
                return ProcessStatus::Normal;
            }
        }
        ProcessStatus::Normal
    }

    fn arp_reset(&mut self, _on_off: bool, _notes_state: &NotesState) {
        // TODO: Implement the arp reset. Here we want to play/stop the current chord.
    }
}

impl ChordProcessor {
    pub(crate) fn build_chord(params: Arc<MidiTransposerParams>, note_info: &NoteInfo) -> u128 {
        let mut chord: u128 = 0b0;
        let base_note = note_info.note % 12;

        // Exit if the transposition is deactivated for this note.
        if !params.notes[base_note as usize].active.value() {
            // Just play the base note.
            chord |= 1 << note_info.note;
            return chord;
        }

        // Create a copy of the note info to map with the transposition.
        let note_transpose = params.notes[base_note as usize].transpose.value() as i8;
        let mapped_note_info = note_info.with_transposition(note_transpose);

        // Include the base note at its original octave if there's an octave transpose.
        let octave_transpose = params.octave_transpose.value();
        if octave_transpose != 0 {
            chord |= 1 << mapped_note_info.note;
        }
        // Also include the base note at the transposed octave.
        chord |= 1 << (mapped_note_info.note + 12 * octave_transpose as u8);

        // For each interval defined in the params, add the corresponding note,
        // based on the base note and the transposition.
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
                chord |= 1 << note;
            });

        chord
    }
}
