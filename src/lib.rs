use arpeggiator::Arpeggiator;
use nih_plug::{context::process::Transport, prelude::*};
use note_info::NoteInfo;
use std::{collections::HashSet, sync::Arc};

mod arpeggiator;
mod note_info;
mod params;

use crate::params::MidiTransposerParams;

struct MidiTransposer {
    params: Arc<MidiTransposerParams>,

    /**
     * The currently processed events (shared with chord processor and arpeggiator)
     */
    midi_events: Vec<NoteEvent<()>>,

    /**
     * The arpeggiator structure
     */
    arp: Arpeggiator,

    /**
     * The last note that has been pressed (accessed by chord processor and arpeggiator but not mutated)
     */
    current_note_held: NoteInfo,

    /**
     * The current generated chord
     */
    generated_chord: Vec<NoteInfo>,

    /**
     * The notes that are currently held
     */
    notes_held: Vec<NoteInfo>,
}

impl Default for MidiTransposer {
    fn default() -> Self {
        let params = Arc::new(MidiTransposerParams::default());

        Self {
            params,
            midi_events: Vec::new(),
            arp: Arpeggiator::new(),
            current_note_held: NoteInfo::default(),
            generated_chord: Vec::new(),
            notes_held: Vec::new(),
        }
    }
}

impl Plugin for MidiTransposer {
    const NAME: &'static str = "Midi Transposer";
    const VENDOR: &'static str = "Stfufane";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "albanese.stephane@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // This plugin doesn't have any audio IO
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    /**
     * Retrieve the sample rate to initialize the arpeggiator
     */
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.arp.set_samplerate(buffer_config.sample_rate);
        nih_dbg!("Sample rate: {}", buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.arp.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Process the incoming events.
        self.midi_events.clear();
        while let Some(event) = context.next_event() {
            // Exclude notes that are not from the filtered channel
            if self.params.in_channel.value() > 0
                && (event.channel().is_none()
                    || event.channel() != Some(self.params.in_channel.value() as u8 - 1))
            {
                self.midi_events.push(event);
                continue;
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
                        note,
                        if output_channel > 0 {
                            output_channel
                        } else {
                            channel
                        },
                        velocity,
                        timing,
                    );
                    match event {
                        NoteEvent::NoteOn { .. } => self.process_note_on(&note_info),
                        NoteEvent::NoteOff { .. } => self.process_note_off(&note_info),
                        _ => (),
                    }
                }
                _ => self.midi_events.push(event),
            }
        }

        // Process the arpeggiator
        if self.params.arp.activated.value() {
            self.process_arp(buffer.samples(), context.transport());
        }

        // Send the processed events.
        for event in self.midi_events.iter() {
            context.send_event(*event);
        }

        ProcessStatus::Normal
    }
}

impl MidiTransposer {
    /**
     * It's always the last note pressed that is used to generate the chord.
     * We keep track of all the notes pressed so when one is released, the previous one is played.
     */
    fn process_note_on(&mut self, note_info: &NoteInfo) {
        // Add the played note to the vector of current notes held.
        self.notes_held.push(*note_info);

        // If the note changed, turn off the previous notes before adding the new ones.
        if !self.params.arp.activated.value()
            && note_info.note != self.current_note_held.note
            && self.current_note_held.is_active()
        {
            self.stop_chord(&note_info.velocity, &note_info.timing);
        }

        // Play the received note with the associated mapping.
        self.build_chord(note_info);
        if !self.params.arp.activated.value() {
            self.play_chord(&note_info.timing);
        }
        self.current_note_held = *note_info;
        self.arp.restart();
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
            if !self.params.arp.activated.value() {
                self.stop_chord(&note_info.velocity, &note_info.timing);
            }

            // If there are no more notes held, stop the current notes.
            if self.notes_held.is_empty() {
                self.current_note_held.reset();
                self.generated_chord.clear();

                // Stop the current note of the arpeggiator if it's running.
                if self.arp.note_info.is_active() {
                    self.midi_events.push(NoteEvent::NoteOff {
                        note: self.arp.note_info.note.unwrap(),
                        timing: note_info.timing,
                        velocity: note_info.velocity,
                        channel: self.arp.note_info.channel,
                        voice_id: None,
                    });
                }
                self.arp.reset();
            } else {
                // If there were still some notes held, play the last one.
                let new_note_info = &self.notes_held.last().unwrap().clone();
                self.build_chord(new_note_info);
                if !self.params.arp.activated.value() {
                    self.play_chord(&new_note_info.timing);
                }
                self.current_note_held = *note_info;
                self.arp.restart();
            }
        }
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
        for note_info in self.generated_chord.iter() {
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
        for note_info in self.generated_chord.iter() {
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
    fn process_arp(&mut self, samples: usize, transport: &Transport) {
        // TODO later: define a callback for interval parameters to handle arpeggiator notes.
        self.arp
            .set_process_info(transport.tempo, samples, &self.params.arp);
        let mut timings: Vec<u32> = Vec::new();

        if self.arp.synced && transport.playing && transport.tempo.is_some() {
            self.arp
                .arpeggiate_sync(transport.pos_beats().unwrap_or(0.0), &mut timings);
        } else {
            self.arp
                .arpeggiate_free(self.params.arp.speed.value(), &mut timings);
        }

        for timing in timings {
            self.play_arp_note(timing);
        }
    }

    fn play_arp_note(&mut self, timing: u32) {
        if self.arp.note_info.is_active() {
            self.midi_events.push(NoteEvent::NoteOff {
                note: self.arp.note_info.note.unwrap(),
                timing,
                velocity: self.arp.note_info.velocity,
                channel: self.arp.note_info.channel,
                voice_id: None,
            });
        }

        if !self.generated_chord.is_empty() {
            self.arp.note_info = self.generated_chord[self.arp.current_index];
            self.midi_events.push(NoteEvent::NoteOn {
                note: self.arp.note_info.note.unwrap(),
                timing,
                velocity: self.arp.note_info.velocity,
                channel: self.arp.note_info.channel,
                voice_id: None,
            });
            // Increment the index for the next note.
            self.arp.current_index = (self.arp.current_index + 1) % self.generated_chord.len();
        }
    }
}

impl ClapPlugin for MidiTransposer {
    const CLAP_ID: &'static str = "com.stfufane.midi-transposer";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A MIDI transposer");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::NoteEffect, ClapFeature::Utility];
}

impl Vst3Plugin for MidiTransposer {
    const VST3_CLASS_ID: [u8; 16] = *b"MidiTransposer!!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Tools, Vst3SubCategory::Fx];
}

nih_export_clap!(MidiTransposer);
nih_export_vst3!(MidiTransposer);
