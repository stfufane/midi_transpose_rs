pub(crate) use crate::note_info::NoteInfo;

pub struct NoteDivision {
    label: &'static str,
    pub division: f64,
}

pub const NOTE_DIVISIONS: [NoteDivision; 9] = [
    NoteDivision {
        label: "1/1",
        division: 4.0,
    },
    NoteDivision {
        label: "1/2",
        division: 2.0,
    },
    NoteDivision {
        label: "1/4.d",
        division: 1.5,
    },
    NoteDivision {
        label: "1/4",
        division: 1.0,
    },
    NoteDivision {
        label: "1/8d",
        division: 0.75,
    },
    NoteDivision {
        label: "1/4.t",
        division: 2.0 / 3.0,
    },
    NoteDivision {
        label: "1/8",
        division: 0.5,
    },
    NoteDivision {
        label: "1/8.t",
        division: 1.0 / 3.0,
    },
    NoteDivision {
        label: "1/16",
        division: 0.25,
    },
];

pub(crate) struct Arpeggiator {
    pub samplerate: f32,
    pub tempo: Option<f64>,
    pub num_samples: usize, // Defined at the beginning of each process
    pub division: f64,
    pub next_beat_position: f64,
    pub time: u32,
    pub current_index: usize,
    pub note_info: NoteInfo,
}

impl Default for Arpeggiator {
    fn default() -> Self {
        Self {
            samplerate: 0.0,
            tempo: None,
            num_samples: 0,
            division: 0.0,
            next_beat_position: 0.0,
            time: 0,
            current_index: 0,
            note_info: NoteInfo::default(),
        }
    }
}

impl Arpeggiator {
    pub fn reset(&mut self) {
        self.tempo = None;
        self.num_samples = 0;
        self.division = 0.0;
        self.next_beat_position = 0.0;
        self.time = 0;
        self.current_index = 0;
        self.note_info.reset();
    }

    /**
     * Called at initialization
     */
    pub fn set_samplerate(&mut self, samplerate: f32) {
        self.samplerate = samplerate;
    }

    /**
     * Called every process to define the right information to process the arpeggiator
     */
    pub fn set_process_info(&mut self, tempo: Option<f64>, num_samples: usize) {
        self.tempo = tempo;
        self.num_samples = num_samples;
        // TODO calculate some information here.
    }

    pub fn restart(&mut self) {
        self.current_index = 0;
    }
}
