use std::cmp;

pub fn read_word(bytes: &[u8], offset: usize) -> u32 {
  (bytes[offset] as u32) | (bytes[offset + 1] as u32) << 8 | (bytes[offset + 2] as u32) << 16 | (bytes[offset + 3] as u32) << 24
}

pub fn read_half(bytes: &[u8], offset: usize) -> u16 {
  (bytes[offset] as u16) | (bytes[offset + 1] as u16) << 8
}

pub fn clamp<T: PartialOrd>(val: T, min_val: T, max_val: T) -> T {
  if val < min_val {
    return min_val;
  }

  if val > max_val {
    return max_val;
  }

  val
}

pub fn min3<T: Ord>(a: T, b: T, c: T) -> T {
  cmp::min(a, cmp::min(b, c))
}

pub fn max3<T: Ord>(a: T, b: T, c: T) -> T {
  cmp::max(a, cmp::max(b, c))
}