use nih_plug::{context::process::Transport, prelude::NoteEvent};
use std::collections::HashSet;
use std::sync::Arc;

use crate::note_state::NoteState;
use crate::params::MidiTransposerParams;

pub struct MidiProcessor {
    /**
     * A non owning reference to the parameters
     */
    params: Arc<MidiTransposerParams>,

    /**
     * The currently processed events
     */
    midi_events: Vec<NoteEvent<()>>,

    /**
     * The last note on that has been pressed
     */
    last_note_on: NoteState,

    /**
     * The notes that are currently on
     */
    current_input_notes_on: Vec<NoteState>,

    /**
     * The mapped notes that are generated from the last note held.
     */
    generated_chord: Vec<NoteState>,
}

impl MidiProcessor {
    pub fn new(params: Arc<MidiTransposerParams>) -> Self {
        Self {
            params,
            midi_events: Vec::new(),
            last_note_on: NoteState::default(),
            current_input_notes_on: Vec::new(),
            generated_chord: Vec::new(),
        }
    }

    /**
     * Process a midi event
     */
    pub fn process_event(
        &mut self,
        event: &NoteEvent<()>,
        _nb_samples: usize,
        _transport: &Transport,
    ) {
        // Exclude notes that are not from the filtered channel
        if self.params.in_channel.value() > 0
            && (event.channel().is_none()
                || event.channel() != Some(self.params.in_channel.value() as u8 - 1))
        {
            self.midi_events.push(*event);
            return;
        }

        let output_channel = self.params.out_channel.value() as u8;
        match event {
            NoteEvent::NoteOn {
                note,
                timing,
                velocity,
                channel,
                ..
            }
            | NoteEvent::NoteOff {
                note,
                timing,
                velocity,
                channel,
                ..
            } => {
                let note_state = NoteState::new(
                    *note,
                    if output_channel > 0 {
                        output_channel
                    } else {
                        *channel
                    },
                    *velocity,
                    *timing,
                );
                match event {
                    NoteEvent::NoteOn { .. } => self.process_note_on(&note_state),
                    NoteEvent::NoteOff { .. } => self.process_note_off(&note_state),
                    _ => (),
                }
            }
            _ => self.midi_events.push(*event),
        }
    }

    fn process_note_on(&mut self, note_state: &NoteState) {
        // Add the played note to the vector of current notes held.
        self.current_input_notes_on.push(*note_state);

        // If the note changed, turn off the previous notes before adding the new ones.
        if note_state.note != self.last_note_on.note && self.last_note_on.is_active() {
            self.stop_chord(&note_state.velocity, &note_state.timing);
        }

        // Play the received note with the associated mapping.
        self.play_mapped_notes(note_state);
    }

    fn process_note_off(&mut self, note_state: &NoteState) {
        // For every note off, remove the received note from the vector of current notes held.
        self.current_input_notes_on
            .retain(|ns| ns.note != note_state.note);

        // Turn off the corresponding notes for the current note off if it's the same as the last played note.
        // Otherwise, it means the released note was not active, so we don't need to do anything (case of multiple notes held)
        if note_state.note == self.last_note_on.note {
            self.stop_chord(&note_state.velocity, &note_state.timing);

            // If there are no more notes held, stop the current notes.
            if self.current_input_notes_on.is_empty() {
                self.last_note_on.reset();
                self.generated_chord.clear();
            } else {
                // If there were still some notes held, play the last one.
                let new_note_state = &self.current_input_notes_on.last().unwrap().clone();
                self.play_mapped_notes(new_note_state);
            }
        }
    }

    fn play_mapped_notes(&mut self, note_state: &NoteState) {
        self.map_notes(note_state);
        self.play_chord(&note_state.timing);
        self.last_note_on = *note_state;
    }

    /**
     * Calculate the generated chord from the last note held given the parameters.
     */
    fn map_notes(&mut self, note_state: &NoteState) {
        self.generated_chord.clear();
        let base_note = note_state.note.unwrap() % 12;
        // Exit if the transposition is deactivated for this note.
        if !self.params.notes[base_note as usize].active.value() {
            // Just play the base note.
            self.generated_chord.push(*note_state);
            return;
        }

        // Create a copy of the note state to map with the transposition.
        let mut mapped_state = *note_state;
        let octave_transpose = self.params.octave_transpose.value() as u8;
        let note_transpose = self.params.notes[base_note as usize].transpose.value() as i8;

        // Include the base note at its original octave if there's an octave transpose.
        if octave_transpose != 0 {
            self.generated_chord
                .push(*mapped_state.transposed(note_transpose));
        }

        // Map the intervals that are not 0 and remove identicals.
        let mut intervals: HashSet<i32> = self.params.notes[base_note as usize]
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

        // Map the intervals to the note state and build the chord.
        for interval in intervals {
            let mapped_note =
                (mapped_state.note.unwrap() as i32 + octave_transpose as i32 * 12 + interval) as u8;
            if mapped_note > 127 {
                continue;
            }
            self.generated_chord.push(NoteState::new(
                mapped_note,
                note_state.channel,
                note_state.velocity,
                note_state.timing,
            ));
        }
    }

    /**
     * Play all the notes in the generated chord
     */
    fn play_chord(&mut self, timing: &u32) {
        for note_state in &self.generated_chord {
            self.midi_events.push(NoteEvent::NoteOn {
                note: note_state.note.unwrap(),
                timing: *timing,
                velocity: note_state.velocity,
                channel: note_state.channel,
                voice_id: None,
            });
        }
    }

    /**
     * Stop all the notes in the generated chord
     */
    fn stop_chord(&mut self, velocity: &f32, timing: &u32) {
        for note_state in &self.generated_chord {
            self.midi_events.push(NoteEvent::NoteOff {
                note: note_state.note.unwrap(),
                timing: *timing,
                velocity: *velocity,
                channel: note_state.channel,
                voice_id: None,
            });
        }
    }

    /**
     * Clear the processed events
     */
    pub fn clear_events(&mut self) {
        self.midi_events.clear();
    }

    /**
     * Return the processed events and clear the buffer
     */
    pub fn get_events(&self) -> &Vec<NoteEvent<()>> {
        &self.midi_events
    }
}
