pub struct JoyControl {
  value: u16
}

impl JoyControl {

  pub fn new() -> Self {
    Self {
      value: 0
    }
  }

  pub fn read(&self) -> u16 {
    self.value
  }

  pub fn write(&mut self, val: u16) -> bool {
    self.value = val;

    if (val >> 4) & 0b1 == 1 {
      // reset joystat bits 3 and 9
    }

    if (val >> 6) & 0b1 == 1 {
      // reset most joy registers in this case

      return true
    }

    false
  }

  pub fn tx_enable(&self) -> bool {
    self.value & 0b1 == 1
  }

  pub fn joy_select(&self) -> bool {
    (self.value >> 1) & 0b1 == 1
  }

  pub fn rx_enable(&self) -> bool {
    (self.value >> 2) & 0b1 == 1
  }

  pub fn acknowledge(&self) -> bool {
    (self.value >> 4) & 0b1 == 1
  }

  pub fn reset(&self) -> bool {
    (self.value >> 6) & 0b1 == 1
  }

  pub fn rx_interrupt_mode(&self) -> u16 {
    2u16.pow((self.value as u32 >> 8) & 0b11)
  }

  pub fn tx_interrupt_enable(&self) -> bool {
    (self.value >> 10) & 0b1 == 1
  }

  pub fn rx_interrupt_enable(&self) -> bool {
    (self.value >> 11) & 0b1 == 1
  }

  pub fn ack_interrupt_enable(&self) -> bool {
    (self.value >> 12) & 0b1 == 1
  }

  pub fn desired_slot(&self) -> usize {
    ((self.value >> 13) & 0b1) as usize
  }
}