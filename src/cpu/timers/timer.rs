use super::timer_mode::{TimerMode, SyncMode};

#[derive(Clone, Copy)]
pub struct Timer {
  pub value: u32,
  pub target_value: u32,
  pub mode: TimerMode,
  pub timer_id: usize,
  pub irq_inhibit: bool,
  is_running: bool,
  pub xblank_occurred: bool
}

impl Timer {
  pub fn new(timer_id: usize) -> Self {
    Self {
      value: 0,
      target_value: 0,
      mode: TimerMode::new(),
      timer_id,
      irq_inhibit: false,
      is_running: true,
      xblank_occurred: false
    }
  }

  pub fn check_target_irq(&mut self) -> bool {
    self.mode.set_target_reached(true);
    if self.mode.reset_on_target() {
      if self.target_value > 0 {
        self.value %= self.target_value;
      } else {
        self.value = 0;
      }
    }
    if self.mode.irq_on_target() && !self.irq_inhibit {
      if self.mode.one_shot_mode() {
        self.irq_inhibit = true;
      }
      return true;
    }

    false
  }

  pub fn check_sync_mode(&mut self, entering_xblank: bool) -> bool {
    if ![0,1].contains(&self.timer_id) {
      return false;
    }
    let mut trigger_irq = false;


    if self.mode.sync_enable() {
      match self.mode.sync_mode(self.timer_id) {
        SyncMode::PauseDuringXBlank => {
          if entering_xblank {
            self.is_running = false;
          }
        }
        SyncMode::ResetAtXBlank => {
          if entering_xblank {
            self.value = 0;

            if self.target_value == 0 && self.check_target_irq() {
              trigger_irq = true;
            }
          }
        }
        SyncMode::XBlankOnly => {
          if entering_xblank {
            self.is_running = true;
          } else {
            self.value = 0;

            if self.target_value == 0 && self.check_target_irq() {
              trigger_irq = true;
            }

            self.is_running = false;
          }
        }
        SyncMode::PauseThenFreeRun => {
          if !self.xblank_occurred {
            self.value = 0;
            self.is_running = false;
          } else if !entering_xblank {
            self.is_running = true;
          }

          if entering_xblank && !self.xblank_occurred {
            self.xblank_occurred = true;
          }
        }
      }
    }

    trigger_irq
  }

  fn check_timer2_sync_mode(&self) -> bool {
    !self.mode.sync_enable() || self.mode.is_free_run()
  }

  pub fn check_overflow_irq(&mut self) -> bool {
    self.value &= 0xffff;

    self.mode.set_overflow_reached(true);
    if self.mode.irq_on_overflow() && !self.irq_inhibit {
      if self.mode.one_shot_mode() {
        self.irq_inhibit = true;
      }
      return true
    }

    false
  }

  pub fn can_run(&self) -> bool {
    let clock_source = self.mode.clock_source();
    match self.timer_id {
      0 => clock_source & 0b1 == 0,
      1 => clock_source & 0b1 == 0,
      2 => clock_source == 0 || clock_source == 1 || self.check_timer2_sync_mode(),
      _ => unreachable!("can't happen")
    }
  }

  pub fn can_run_dotclock(&self) -> bool {
    self.timer_id == 0 && self.mode.clock_source() & 0b1 == 1
  }

  pub fn can_run_hblank_clock(&self) -> bool {
    self.timer_id == 1 && self.mode.clock_source() & 0b1 == 1
  }

  pub fn can_run_div8(&self) -> bool {
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

    // it'd be tempting to use u16 here and have wrapping_add take care of the overflow automatically, but
    // there could be a case where the target value is really close to the overflow, and if the value overflows
    // before the target check below, then there could be bugs as a result
    self.value += cycles as u32;

    let mut irq_triggered = false;

    if previous_val < self.target_value && self.value >= self.target_value {
      irq_triggered = self.check_target_irq();
    }


    if self.value > 0xffff {
      irq_triggered = self.check_overflow_irq();
    }

    irq_triggered
  }

  pub fn update_state(&mut self, in_xblank: bool) {
    if self.mode.sync_enable() {
      let mode = self.mode.sync_mode(self.timer_id);

      match mode {
        SyncMode::PauseDuringXBlank => self.is_running = !in_xblank,
        SyncMode::XBlankOnly => self.is_running = in_xblank,
        SyncMode::ResetAtXBlank => self.is_running = true,
        _ => ()
      }
    }
  }

  pub fn update_timer2_state(&mut self) {
    self.is_running = self.check_timer2_sync_mode();
  }
}