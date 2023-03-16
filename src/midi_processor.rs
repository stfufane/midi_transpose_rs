use std::sync::Arc;
use nih_plug::prelude::NoteEvent;

use crate::params::MidiTransposerParams;

struct NoteState {
    note: Option<u8>,
    channel: usize,
    velocity: f32,
}

impl Default for NoteState {
    fn default() -> Self {
        Self {
            note: None,
            channel: 0,
            velocity: 0.0,
        }
    }
}

impl NoteState {
    fn reset(&mut self) {
        self.note = None;
        self.channel = 0;
        self.velocity = 0.0;
    }

    fn is_active(&self) -> bool {
        self.note.is_some()
    }
}

pub struct MidiProcessor {
    /**
     * A non owning reference to the parameters
     */
    params: Arc<MidiTransposerParams>,

    /**
     * The currently processed events
     */
    midi_events: Vec<NoteEvent<()>>,
}

impl MidiProcessor {
    pub fn new(params: Arc<MidiTransposerParams>) -> Self {
        Self {
            params,
            midi_events: Vec::new(),
        }
    }   

    /**
     * Process a midi event
     */
    pub fn process_event(&mut self, event: NoteEvent<()>) {
        // TODO: read parameters to map notes
        match event {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => self.midi_events.push(NoteEvent::NoteOn {
                timing,
                voice_id,
                channel: 15 - channel,
                note: 127 - note,
                velocity: 1.0 - velocity,
            }),
            NoteEvent::NoteOff {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => self.midi_events.push(NoteEvent::NoteOff {
                timing,
                voice_id,
                channel: 15 - channel,
                note: 127 - note,
                velocity: 1.0 - velocity,
            }),
            _ => (),
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
