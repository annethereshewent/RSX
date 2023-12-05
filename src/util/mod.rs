pub fn read_word(bytes: &[u8], index: usize) -> u32 {
  (bytes[index] as u32) | (bytes[index + 1] as u32) << 8 | (bytes[index + 2] as u32) << 16 | (bytes[index + 3] as u32) << 24
}

pub fn read_half(bytes: &[u8], index: usize) -> u16 {
  (bytes[index] as u16) | (bytes[index + 1] as u16) << 8
}