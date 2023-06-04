pub(crate) use crate::note_info::NoteInfo;

pub(crate) struct Arpeggiator {
    samplerate: f32,
    tempo: Option<f64>,
    num_samples: usize, // Defined at the beginning of each process
    division: f64,
    next_beat_position: f64,
    time: u32,
    pub current_index: usize,
    pub note_info: NoteInfo,
}

impl Arpeggiator {
    pub fn new() -> Self {
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

    pub fn arpeggiate_free(&mut self, speed: f32, timings: &mut Vec<u32>) {
        // TODO: calculate the note duration from the BPM.
        let note_duration = (self.samplerate * 0.1 * (0.1 + (5.0 - 5.0 * speed))) as u32;
        if self.num_samples < note_duration.try_into().unwrap() {
            if (self.time + self.num_samples as u32) >= note_duration {
                let timing = std::cmp::max(
                    0,
                    std::cmp::min((note_duration - self.time) as usize, self.num_samples - 1),
                );
                timings.push(timing as u32);
            }
            self.time = (self.time + self.num_samples as u32) % note_duration;
        } else {
            while self.time < self.num_samples as u32 {
                let timing = self.time + note_duration;
                if timing < self.num_samples as u32 {
                    timings.push(timing);
                }
                self.time += note_duration;
            }
            self.time %= self.num_samples as u32;
        }
    }

    pub fn arpeggiate_sync(&mut self, beat_position: f64, rate: i32, timings: &mut Vec<u32>) {
        let samples_per_beat = self.samplerate as f64 / (self.tempo.unwrap_or(60.0) / 60.0);
        let mut timing: u32 = 0;

        while timing < self.num_samples as u32 {
            // Reset the position calculation if the division has changed.
            let last_division = NOTE_DIVISIONS[rate as usize].division;
            if self.division != last_division {
                // Update the current division from parameter
                self.division = last_division;
                self.next_beat_position = 0.0;
            }

            // We need to get the current quarter note and see what's the next candidate position to snap to the current time division.
            if self.next_beat_position == 0.0 {
                let mut nb_divisions = 1;
                while self.next_beat_position == 0.0 {
                    // For divisions greater than 1.0, we just snap to the next quarter note.
                    let next_division =
                        beat_position.floor() + (nb_divisions as f64 * self.division.min(1.0));
                    if next_division >= beat_position {
                        self.next_beat_position = next_division;
                    }

                    nb_divisions += 1;
                }
            }

            // The next "snapping" time division occurs in this block! We need to calculate the timing here and play the note.
            timing = ((self.next_beat_position - beat_position) * samples_per_beat) as u32;
            if timing < self.num_samples as u32 {
                timings.push(timing);
                self.next_beat_position += self.division;
            }
        }
    }
}

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
