use nih_plug::context::process::Transport;
use nih_plug::prelude::NoteEvent;
use std::collections::HashSet;
use std::sync::Arc;

use crate::arpeggiator::{Arpeggiator, NOTE_DIVISIONS};
use crate::note_info::NoteInfo;
use crate::params::MidiTransposerParams;

pub(crate) struct MidiProcessor {
    /**
     * A non owning reference to the parameters
     */
    params: Arc<MidiTransposerParams>,

    /**
     * The arpeggiator
     */
    pub(crate) arpeggiator: Arpeggiator,

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
            arpeggiator: Arpeggiator::default(),
            midi_events: Vec::new(),
            current_note_held: NoteInfo::default(),
            notes_held: Vec::new(),
            generated_chord: Vec::new(),
        }
    }

    /**
     * Process a midi event
     */
    pub fn process_event(&mut self, event: &NoteEvent<()>) {
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
        self.notes_held.retain(|ni| ni.note != note_info.note);

        // Turn off the corresponding notes for the current note off if it's the same as the last played note.
        // Otherwise, it means the released note was not active, so we don't need to do anything (case of multiple notes held)
        if note_info.note == self.current_note_held.note {
            self.stop_chord(&note_info.velocity, &note_info.timing);

            // If there are no more notes held, stop the current notes.
            if self.notes_held.is_empty() {
                self.current_note_held.reset();

                // Stop the current note of the arpeggiator if it's running.
                if self.arpeggiator.note_info.is_active() {
                    self.midi_events.push(NoteEvent::NoteOff {
                        note: self.arpeggiator.note_info.note.unwrap(),
                        timing: note_info.timing,
                        velocity: note_info.velocity,
                        channel: self.arpeggiator.note_info.channel,
                        voice_id: None
                    });
                }
                self.arpeggiator.reset();
                self.generated_chord.clear();
            } else {
                // If there were still some notes held, play the last one.
                let new_note_info = &self.notes_held.last().unwrap().clone();
                self.build_and_play_chord(new_note_info);
            }
        }
    }

    fn build_and_play_chord(&mut self, note_info: &NoteInfo) {
        self.build_chord(note_info);
        self.play_chord(&note_info.timing);
        self.current_note_held = *note_info;
        self.arpeggiator.restart();
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

        // Create a copy of the note info to map with the transposition.
        let mut mapped_note_info = *note_info;
        let octave_transpose = self.params.octave_transpose.value() as u8;
        let note_transpose = self.params.notes[base_note as usize].transpose.value() as i8;

        // Include the base note at its original octave if there's an octave transpose.
        if octave_transpose != 0 {
            self.generated_chord
                .push(*mapped_note_info.transposed(note_transpose));
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

        // Map the intervals to the note info and build the chord.
        for interval in intervals {
            let mapped_note = (mapped_note_info.note.unwrap() as i32
                + octave_transpose as i32 * 12
                + interval) as u8;
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
        // The chord is not played if the arpeggiator is on.
        if self.params.arp.activated.value() {
            return;
        }

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
        // The chord is not stopped if the arpeggiator is on.
        if self.params.arp.activated.value() {
            return;
        }

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
     * Process the arpeggiator depending on the context (DAW transport or free)
     */
    pub fn process_arp(&mut self, samples: usize, transport: &Transport) {
        self.arpeggiator.set_process_info(transport.tempo, samples);

        // TODO later: define a callback for interval parameters to handle arpeggiator notes.

        if self.params.arp.synced.value() && transport.playing && transport.tempo.is_some() {
            self.arpeggiate_sync(transport.pos_beats().unwrap_or(0.0));
        } else {
            self.arpeggiate_free();
        }
    }

    fn arpeggiate_free(&mut self) {
        // TODO: calculate the note duration from the BPM.
        let note_duration = (self.arpeggiator.samplerate * 0.1 * (0.1 + (5.0 - 5.0 * 0.0))) as u32;
        if self.arpeggiator.num_samples < note_duration.try_into().unwrap() {
            if (self.arpeggiator.time + self.arpeggiator.num_samples as u32) >= note_duration {
                let timing = std::cmp::max(
                    0,
                    std::cmp::min(
                        (note_duration - self.arpeggiator.time) as usize,
                        self.arpeggiator.num_samples - 1,
                    ),
                );
                self.play_arp_note(timing as u32);
            }
            self.arpeggiator.time =
                (self.arpeggiator.time + self.arpeggiator.num_samples as u32) % note_duration;
        } else {
            while self.arpeggiator.time < self.arpeggiator.num_samples as u32 {
                let timing = self.arpeggiator.time + note_duration;
                if timing < self.arpeggiator.num_samples as u32 {
                    self.play_arp_note(timing);
                }
                self.arpeggiator.time += note_duration;
            }
            self.arpeggiator.time = self.arpeggiator.time % self.arpeggiator.num_samples as u32;
        }
    }

    fn arpeggiate_sync(&mut self, beat_position: f64) {
        let samples_per_beat =
            self.arpeggiator.samplerate as f64 / (self.arpeggiator.tempo.unwrap_or(60.0) as f64 / 60.0);
        let mut timing: u32 = 0;

        while timing < self.arpeggiator.num_samples as u32 {
            // Reset the position calculation if the division has changed.
            let last_division = NOTE_DIVISIONS[self.params.arp.rate.value() as usize].division;
            if self.arpeggiator.division != last_division {
                // Update the current division from parameter
                self.arpeggiator.division = last_division;
                self.arpeggiator.next_beat_position = 0.0;
            }

            // We need to get the current quarter note and see what's the next candidate position to snap to the current time division.
            if self.arpeggiator.next_beat_position == 0.0 {
                let mut nb_divisions = 1;
                while self.arpeggiator.next_beat_position == 0.0 {
                    // For divisions greater than 1.0, we just snap to the next quarter note.
                    let next_division = beat_position.floor()
                        + (nb_divisions as f64 * self.arpeggiator.division.min(1.0));
                    if next_division >= beat_position {
                        self.arpeggiator.next_beat_position = next_division as f64;
                    }

                    nb_divisions += 1;
                }
            }

            // The next "snapping" time division occurs in this block! We need to calculate the timing here and play the note.
            timing = ((self.arpeggiator.next_beat_position - beat_position)
                * samples_per_beat) as u32;
            if timing < self.arpeggiator.num_samples as u32 {
                self.play_arp_note(timing);
                self.arpeggiator.next_beat_position += self.arpeggiator.division;
            }
        }
    }

    fn play_arp_note(&mut self, timing: u32) {
        if self.arpeggiator.note_info.is_active() {
            self.midi_events.push(NoteEvent::NoteOff {
                note: self.arpeggiator.note_info.note.unwrap(),
                timing,
                velocity: self.arpeggiator.note_info.velocity,
                channel: self.arpeggiator.note_info.channel,
                voice_id: None,
            });
        }

        if !self.generated_chord.is_empty() {
            self.arpeggiator.note_info = self.generated_chord[self.arpeggiator.current_index];
            self.midi_events.push(NoteEvent::NoteOn {
                note: self.arpeggiator.note_info.note.unwrap(),
                timing,
                velocity: self.arpeggiator.note_info.velocity,
                channel: self.arpeggiator.note_info.channel,
                voice_id: None,
            });
            // Increment the index for the next note.
            self.arpeggiator.current_index =
                (self.arpeggiator.current_index + 1) % self.generated_chord.len();
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
