use nih_plug::{context::process::Transport, prelude::NoteEvent};
use std::collections::HashSet;
use std::sync::Arc;

use crate::note_info::NoteInfo;
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
     * The last note that has been pressed
     */
    current_note_held: NoteInfo,

    /**
     * The notes that are currently held
     */
    notes_held: Vec<NoteInfo>,

    /**
     * The mapped notes that are generated from the last note held.
     */
    generated_chord: Vec<NoteInfo>,
}

impl MidiProcessor {
    pub fn new(params: Arc<MidiTransposerParams>) -> Self {
        Self {
            params,
            midi_events: Vec::new(),
            current_note_held: NoteInfo::default(),
            notes_held: Vec::new(),
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
                let note_info = NoteInfo::new(
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
                    NoteEvent::NoteOn { .. } => self.process_note_on(&note_info),
                    NoteEvent::NoteOff { .. } => self.process_note_off(&note_info),
                    _ => (),
                }
            }
            _ => self.midi_events.push(*event),
        }
    }

    /**
     * It's always the last note pressed that is used to generate the chord.
     * We keep track of all the notes pressed so when one is released, the previous one is played.
     */
    fn process_note_on(&mut self, note_info: &NoteInfo) {
        // Add the played note to the vector of current notes held.
        self.notes_held.push(*note_info);

        // If the note changed, turn off the previous notes before adding the new ones.
        if note_info.note != self.current_note_held.note && self.current_note_held.is_active() {
            self.stop_chord(&note_info.velocity, &note_info.timing);
        }

        // Play the received note with the associated mapping.
        self.build_and_play_chord(note_info);
    }

    /**
     * For notes off, we have to check if the note released is the one currently played or one in the pool of notes held.
     */
    fn process_note_off(&mut self, note_info: &NoteInfo) {
        // For every note off, remove the received note from the vector of current notes held.
        self.notes_held.retain(|ns| ns.note != note_info.note);

        // Turn off the corresponding notes for the current note off if it's the same as the last played note.
        // Otherwise, it means the released note was not active, so we don't need to do anything (case of multiple notes held)
        if note_info.note == self.current_note_held.note {
            self.stop_chord(&note_info.velocity, &note_info.timing);

            // If there are no more notes held, stop the current notes.
            if self.notes_held.is_empty() {
                self.current_note_held.reset();
                self.generated_chord.clear();
            } else {
                // If there were still some notes held, play the last one.
                let new_note_state = &self.notes_held.last().unwrap().clone();
                self.build_and_play_chord(new_note_state);
            }
        }
    }

    fn build_and_play_chord(&mut self, note_info: &NoteInfo) {
        self.build_chord(note_info);
        self.play_chord(&note_info.timing);
        self.current_note_held = *note_info;
    }

    /**
     * Calculate the generated chord from the last note held given the parameters.
     */
    fn build_chord(&mut self, note_info: &NoteInfo) {
        self.generated_chord.clear();
        let base_note = note_info.note.unwrap() % 12;
        // Exit if the transposition is deactivated for this note.
        if !self.params.notes[base_note as usize].active.value() {
            // Just play the base note.
            self.generated_chord.push(*note_info);
            return;
        }

        // Create a copy of the note state to map with the transposition.
        let mut mapped_state = *note_info;
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
            self.generated_chord.push(NoteInfo::new(
                mapped_note,
                note_info.channel,
                note_info.velocity,
                note_info.timing,
            ));
        }
    }

    /**
     * Play all the notes in the generated chord
     */
    fn play_chord(&mut self, timing: &u32) {
        for note_info in &self.generated_chord {
            self.midi_events.push(NoteEvent::NoteOn {
                note: note_info.note.unwrap(),
                timing: *timing,
                velocity: note_info.velocity,
                channel: note_info.channel,
                voice_id: None,
            });
        }
    }

    /**
     * Stop all the notes in the generated chord
     */
    fn stop_chord(&mut self, velocity: &f32, timing: &u32) {
        for note_info in &self.generated_chord {
            self.midi_events.push(NoteEvent::NoteOff {
                note: note_info.note.unwrap(),
                timing: *timing,
                velocity: *velocity,
                channel: note_info.channel,
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
