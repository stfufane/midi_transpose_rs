use midi_processor::MidiProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

mod midi_processor;
mod params;
mod note_info;
// mod arpeggiator;

use crate::params::MidiTransposerParams;

struct MidiTransposer {
    params: Arc<MidiTransposerParams>,

    processor: MidiProcessor,
}

impl Default for MidiTransposer {
    fn default() -> Self {
        let params = Arc::new(MidiTransposerParams::default());
        let processor = MidiProcessor::new(params.clone());
        Self { params, processor }
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

    // TODO : add processor initialization (arpeggiator samplerate)

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Process the incoming events.
        self.processor.clear_events();
        while let Some(event) = context.next_event() {
            self.processor
                .process_event(&event, buffer.samples(), context.transport());
        }
        // Send the processed events.
        for event in self.processor.get_events() {
            context.send_event(*event);
        }

        ProcessStatus::Normal
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
