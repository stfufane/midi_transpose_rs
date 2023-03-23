use crate::note_state::NoteState;

pub struct Arpeggiator 
{
    samplerate: f32,
    division: i64,
    next_beat_position: i64,
    time: i32,
    current_index: usize,
    note_state: NoteState,
}

impl Default for Arpeggiator {
    fn default() -> Self {
        Self {
            samplerate: 0.0,
            division: 0,
            next_beat_position: 0,
            time: 0,
            current_index: 0,
            note_state: NoteState::default(),
        }
    }
}

impl Arpeggiator {
    pub fn new(samplerate: f32) -> Self {
        Self {
            samplerate,
            division: 0,
            next_beat_position: 0,
            time: 0,
            current_index: 0,
            note_state: NoteState::default(),
        }
    }

    pub fn reset(&mut self) {
        self.division = 0;
        self.next_beat_position = 0;
        self.time = 0;
        self.current_index = 0;
        self.note_state.reset();
    }

    pub fn restart(&mut self) {
        self.current_index = 0;
    }
}