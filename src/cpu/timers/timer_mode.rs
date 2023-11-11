pub enum SyncMode {
  PauseDuringXBlank,
  ResetAtXBlank,
  XBlankOnly,
  PauseThenFreeRun
}

#[derive(Clone, Copy)]
pub struct TimerMode {
  pub val: u16
}

impl TimerMode {
  pub fn new() -> Self {
    Self {
      val: 0
    }
  }

  pub fn write(&mut self, val: u16) {
    // clear the bottom bits except bits 10-12
    self.val &= 0b111 << 10;
    // set bit 10 after writing to this register
    self.val |= 1 << 10;
    // finally set the lower 9 bits to the value given
    self.val |= val & 0x3ff;
  }

  pub fn reset_on_target(&self) -> bool {
    (self.val >> 3) & 0b1 == 1
  }

  pub fn irq_on_target(&self) -> bool {
    (self.val >> 4) & 0b1 == 1
  }

  pub fn irq_on_overflow(&self) -> bool {
    (self.val >> 5) & 0b1 == 1
  }

  pub fn one_shot_mode(&self) -> bool {
    (self.val >> 6) & 0b1 == 0
  }

  pub fn clock_source(&self) -> u16 {
    (self.val >> 8) & 0b11
  }

  pub fn sync_enable(&self) -> bool {
    self.val & 0b1 == 1
  }

  pub fn is_free_run(&self) -> bool {
    let mode = (self.val >> 1) & 0b11;

    ![0,3].contains(&mode)
  }

  pub fn sync_mode(&self, timer_id: usize) -> SyncMode {
    if timer_id == 2 {
      unreachable!("shouldn't happen");
    } else {
      match (self.val >> 1) & 0b11 {
        0 => SyncMode::PauseDuringXBlank,
        1 => SyncMode::ResetAtXBlank,
        2 => SyncMode::XBlankOnly,
        3 => SyncMode::PauseThenFreeRun,
        _ => unreachable!("can't happen")
      }
    }
  }


  pub fn set_target_reached(&mut self, target: bool) {
    if target {
      self.val |= 1 << 11;
    } else {
      self.val &= !(1 << 11);
    }
  }

  pub fn set_overflow_reached(&mut self, overflow: bool) {
    if overflow {
      self.val |= 1 << 12;
    } else {
      self.val &= !(1 << 12);
    }
  }

}