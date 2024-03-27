#[derive(Debug, Clone, Copy)]
pub struct NoteInfo {
    pub note: u8,
    pub channel: u8,
    pub velocity: f32,
    pub timing: u32,
}

impl NoteInfo {
    pub fn new(note: u8, channel: u8, velocity: f32, timing: u32) -> Self {
        Self {
            note,
            channel,
            velocity,
            timing,
        }
    }

    pub fn with_transposition(&self, transposition: i8) -> Self {
        Self {
            note: (self.note as i8 + transposition) as u8,
            channel: self.channel,
            velocity: self.velocity,
            timing: self.timing,
        }
    }
}
