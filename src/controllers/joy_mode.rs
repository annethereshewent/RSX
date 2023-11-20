pub struct JoyMode {
  value: u16
}

impl JoyMode {
  pub fn new() -> Self {
    Self {
      value: 0
    }
  }

  pub fn write(&mut self, val: u16) {
    self.value = val;
  }

  pub fn baudrate_reload_factor(&self) -> u16 {
    match self.value & 0b11 {
      0 | 1 => 1,
      2 => 16,
      3 => 64,
      _ => unreachable!("can't happen")
    }
  }

  pub fn character_length(&self) -> u16 {
    match (self.value >> 1) & 0b11 {
      0 => 5,
      1 => 6,
      2 => 7,
      3 => 8,
      _ => unreachable!("can't happen")
    }
  }

  pub fn parity_enable(&self) -> bool {
    (self.value >> 4) & 0b1 == 1
  }

  pub fn parity_type(&self) -> bool {
    (self.value >> 5) & 0b1 == 1
  }

  pub fn is_inverse_polarity(&self) -> bool {
    (self.value >> 8) & 0b1 == 1
  }
}