use arp_processor::ArpProcessor;
use chord_processor::ChordProcessor;
use nih_plug::prelude::*;
use note_info::NoteInfo;
use params::MidiTransposerParams;
use std::sync::{atomic::AtomicBool, Arc};

mod arp_processor;
mod chord_processor;
mod note_info;
mod params;

enum ProcessorType {
    Chord,
    Arpeggio,
}

#[derive(Debug, Clone)]
struct NotesState {
    pub notes_held: Vec<NoteInfo>,
    pub current_note_held: Option<NoteInfo>,
    pub current_chord: u128,
}

impl Default for NotesState {
    fn default() -> Self {
        Self {
            notes_held: Vec::with_capacity(24),
            current_note_held: None,
            current_chord: 0,
        }
    }
}

pub(crate) trait MidiProcessor {
    fn process(&mut self, notes_state: &NotesState, nb_samples: usize) -> ProcessStatus;
    fn arp_reset(&mut self, on_off: bool);
}

struct MidiTransposer {
    pub(crate) params: Arc<MidiTransposerParams>,

    /**
     * The type of processor that is currently active
     */
    processor_type: ProcessorType,

    /**
     * The chord processor
     */
    chord_processor: ChordProcessor,

    /**
     * The arpeggio processor
     */
    arp_processor: ArpProcessor,

    /**
     * Will be set by the param callback to know at the beginning of the process if the arp should be reset
     */
    should_reset_arp: Arc<AtomicBool>,

    /**
     * The state of the notes played
     */
    notes_state: NotesState,
}

impl MidiTransposer {
    fn process_note_on(&mut self, note_info: &NoteInfo) {
        self.notes_state.notes_held.push(*note_info);
        self.notes_state.current_chord =
            ChordProcessor::build_chord(self.params.clone(), note_info);
        self.notes_state.current_note_held = Some(*note_info);
    }

    fn process_note_off(&mut self, note_info: &NoteInfo) {
        self.notes_state
            .notes_held
            .retain(|n| n.note != note_info.note);
        if self.notes_state.notes_held.is_empty() {
            self.notes_state.current_note_held = None;
            self.notes_state.current_chord = 0;
        } else {
            self.notes_state.current_note_held = Some(*self.notes_state.notes_held.last().unwrap());
            self.notes_state.current_chord = ChordProcessor::build_chord(
                self.params.clone(),
                self.notes_state.current_note_held.as_ref().unwrap(),
            );
        }
    }

    fn update_processor(&mut self) {
        let arp_activated = self.params.arp.activated.value();
        self.processor_type = if arp_activated {
            ProcessorType::Arpeggio
        } else {
            ProcessorType::Chord
        };
        self.chord_processor.arp_reset(arp_activated);
        self.arp_processor.arp_reset(arp_activated);
    }
}

impl Default for MidiTransposer {
    fn default() -> Self {
        let should_reset_arp = Arc::new(AtomicBool::new(true));
        Self {
            params: Arc::new(MidiTransposerParams::new(should_reset_arp.clone())),
            processor_type: ProcessorType::Chord,
            chord_processor: ChordProcessor::default(),
            arp_processor: ArpProcessor::default(),
            should_reset_arp,
            notes_state: NotesState::default(),
        }
    }
}

impl Plugin for MidiTransposer {
    const NAME: &'static str = "Midi Transposer";
    const VENDOR: &'static str = "Stfefane";
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

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        nih_trace!("Initializing MidiTransposer");
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Check if the arpeggiator has been turned on/off to reset it and notify the processors.
        if self
            .should_reset_arp
            .compare_exchange(
                true,
                false,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            self.update_processor();
        }

        // Process the incoming events.
        while let Some(event) = context.next_event() {
            // Exclude notes that are not from the filtered channel
            if self.params.in_channel.value() > 0
                && (event.channel().is_none()
                    || event.channel() != Some(self.params.in_channel.value() as u8 - 1))
            {
                context.send_event(event);
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
                        _ => context.send_event(event),
                    }
                }
                _ => context.send_event(event),
            }
        }

        match self.processor_type {
            ProcessorType::Chord => self
                .chord_processor
                .process(&self.notes_state, buffer.samples()),
            ProcessorType::Arpeggio => self
                .arp_processor
                .process(&self.notes_state, buffer.samples()),
        }
    }
}

impl ClapPlugin for MidiTransposer {
    const CLAP_ID: &'static str = "com.stfefane.midi-transposer";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A note to chords midi plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::NoteEffect, ClapFeature::Utility];
}

impl Vst3Plugin for MidiTransposer {
    const VST3_CLASS_ID: [u8; 16] = *b"MidiTransposerSt";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Tools, Vst3SubCategory::Fx];
}

nih_export_clap!(MidiTransposer);
nih_export_vst3!(MidiTransposer);
