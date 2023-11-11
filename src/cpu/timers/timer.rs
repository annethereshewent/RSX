use super::timer_mode::TimerMode;

#[derive(Clone, Copy)]
pub struct Timer {
  pub value: u16,
  pub target_value: u16,
  pub mode: TimerMode,
  pub timer_id: usize,
  pub irq_inhibit: bool,
  is_running: bool
}

impl Timer {
  pub fn new(timer_id: usize) -> Self {
    Self {
      value: 0,
      target_value: 0,
      mode: TimerMode::new(),
      timer_id,
      irq_inhibit: false,
      is_running: true
    }
  }

  pub fn check_target_irq(&mut self) -> bool {
    if self.mode.reset_on_target() {
      self.value %= self.target_value;
    }
    if self.mode.irq_on_target() && !self.irq_inhibit {
      if self.mode.one_shot_mode() {
        self.irq_inhibit = true;
      }
      return true;
    }

    false
  }

  pub fn can_run(&self) -> bool {
    let clock_source = self.mode.clock_source();
    match self.timer_id {
      0 => clock_source & 0b1 == 0,
      1 => clock_source & 0b1 == 0,
      2 => clock_source == 0 || clock_source == 1,
      _ => unreachable!("can't happen")
    }
  }

  pub fn run_div8(&self) -> bool {
    let clock_source = self.mode.clock_source();
    self.timer_id == 2 && (clock_source == 2 || clock_source == 3)
  }

  pub fn tick(&mut self, cycles: i32) -> bool {
    if self.mode.reset_on_target() && self.target_value == 0 && self.value == 0 {
      return self.check_target_irq();
    }

    if cycles == 0 || !self.is_running {
      return false;
    }
    let previous_val = self.value;

    self.value = self.value.wrapping_add(cycles as u16);

    let mut irq_triggered = false;

    if previous_val < self.target_value && self.value >= self.target_value {
      irq_triggered = self.check_target_irq();
    }

    if previous_val < self.value || self.value == 0xffff {
      if self.mode.irq_on_overflow() && !self.irq_inhibit {
        if self.mode.one_shot_mode() {
          self.irq_inhibit = true;
        }
        irq_triggered = true;
      }
    }

    irq_triggered
  }
}