
#[derive(Debug, Clone, Copy)]
pub struct NoteState {
    pub note: Option<u8>,
    pub channel: u8,
    pub velocity: f32,
    pub timing: u32,
}

impl Default for NoteState {
    fn default() -> Self {
        Self {
            note: None,
            channel: 0,
            velocity: 0.0,
            timing: 0,
        }
    }
}

impl NoteState {
    pub fn new(note: u8, channel: u8, velocity: f32, timing: u32) -> Self {
        Self {
            note: Some(note),
            channel,
            velocity,
            timing,
        }
    }

    pub fn transposed(&mut self, transposition: i8) -> &mut Self {
        if let Some(note) = self.note {
            self.note = Some((note as i8 + transposition) as u8);
        }
        self
    }

    pub fn reset(&mut self) {
        self.note = None;
        self.channel = 0;
        self.velocity = 0.0;
        self.timing = 0;
    }

    pub fn is_active(&self) -> bool {
        self.note.is_some()
    }
}