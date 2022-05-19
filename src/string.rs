#[derive(Debug, Clone)]
pub struct String<const LEN: usize> {
    buffer: [u8; LEN],
    position: usize,
}

impl<const LEN: usize> String<LEN> {
    pub const fn new() -> Self {
        String {
            buffer: [0; LEN],
            position: 0,
        }
    }

    pub fn push(&mut self, ch: u8) {
        if self.position < self.buffer.len() {
            self.buffer[self.position] = ch;
            self.position += 1;
        }
    }

    pub fn clear(&mut self) {
        self.position = 0;
    }
}

impl<const LEN: usize> AsRef<[u8]> for String<LEN> {
    fn as_ref(&self) -> &[u8] {
        &self.buffer[0..self.position]
    }
}
