use nih_plug::prelude::*;

/**
 * Represents one interval slider for a note.
 */
#[derive(Params)]
pub struct IntervalParam {
    #[id = "interval"]
    pub interval: IntParam,
}

/**
 * Reprensents a note panel.
 * It can be muted and/or transposed, and it holds 12 intervals
 */
#[derive(Params)]
pub struct NoteParam {
    #[id = "active"]
    pub active: BoolParam,
    #[id = "transpose"]
    pub transpose: IntParam,
    #[nested(array, group = "Intervals")]
    pub intervals: [IntervalParam; 12],
}

#[derive(Params)]
pub struct ArpParams {
    #[id = "arp_on"]
    pub activated: BoolParam,
    #[id = "arp_sync"]
    pub synced: BoolParam,
    #[id = "arp_speed"]
    pub speed: FloatParam,
    #[id = "arp_rate"]
    pub rate: IntParam,
}

#[derive(Params)]
pub struct MidiTransposerParams {
    #[id = "in_channel"]
    pub in_channel: IntParam,
    #[id = "out_channel"]
    pub out_channel: IntParam,
    #[id = "octave_transpose"]
    pub octave_transpose: IntParam,
    #[nested(group = "Arpeggiator")]
    pub arp: ArpParams,
    #[nested(array, group = "Notes")]
    pub notes: [NoteParam; 12],
}

impl Default for MidiTransposerParams {
    fn default() -> Self {
        let all_notes: [usize; 12] = core::array::from_fn(|i| i + 1);
        Self {
            in_channel: IntParam::new("Input Channel", 1, IntRange::Linear { min: 0, max: 16 }),
            out_channel: IntParam::new("Output Channel", 1, IntRange::Linear { min: 0, max: 16 }),
            octave_transpose: IntParam::new(
                "Octave Transpose",
                0,
                IntRange::Linear { min: -1, max: 4 },
            ),
            arp: ArpParams {
                activated: BoolParam::new("Arp On", false),
                synced: BoolParam::new("Arp Sync", false),
                speed: FloatParam::new("Arp Speed", 1.0, FloatRange::Linear { min: 0.1, max: 10.0 }),
                rate: IntParam::new("Arp Rate", 0, IntRange::Linear { min: 0, max: 8 }),
            },
            notes: all_notes.map(|note| NoteParam {
                active: BoolParam::new(format!("Activate {note}"), true),
                transpose: IntParam::new(
                    format!("Transpose {note}"),
                    0,
                    IntRange::Linear { min: -12, max: 12 },
                ),
                intervals: all_notes.map(|interval| IntervalParam {
                    interval: IntParam::new(
                        format!("Interval {note} {interval}"),
                        0,
                        IntRange::Linear { min: -12, max: 12 },
                    ),
                }),
            }),
        }
    }
}
