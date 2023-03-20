use nih_plug::{context::process::Transport, prelude::NoteEvent};
use std::sync::Arc;

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
    pub fn process_event(
        &mut self,
        event: &NoteEvent<()>,
        _nb_samples: usize,
        _transport: &Transport,
    ) {
        // TODO: read parameters to map notes
        // Exclude notes that are not from the filtered channel
        if self.params.in_channel.value() > 0
            && (event.channel() == None
                || event.channel() != Some(self.params.in_channel.value() as u8))
        {
            return;
        }
        match event {
            NoteEvent::NoteOn { .. } | NoteEvent::NoteOff { .. } => self.map_note(event),
            _ => self.midi_events.push(*event),
        }
    }

    fn map_note(&mut self, _event: &NoteEvent<()>) {}

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
