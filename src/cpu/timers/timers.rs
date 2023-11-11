use std::{rc::Rc, cell::Cell};

use crate::cpu::interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt};

use super::timer::Timer;

pub struct Timers {
  t: [Timer; 3],
  interrupts: Rc<Cell<InterruptRegisters>>,
  div8: i32
}

impl Timers {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      t: [
        Timer::new(0),
        Timer::new(1),
        Timer::new(2)
      ],
      interrupts,
      div8: 0
    }
  }

  pub fn read(&mut self, address: u32) -> u16 {
    let timer_id = ((address & 0x30) >> 4) as usize;
    let offset = address & 0xcc;

    let timer = &mut self.t[timer_id];

    match offset {
      0 => timer.value,
      4 => {
        let val = timer.mode.val;

        timer.mode.set_overflow_reached(false);
        timer.mode.set_target_reached(false);

        val
      }
      8 => timer.target_value,
      _ => panic!("unsupported offset given to timer io: {offset}")
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    let mut already_ran = false;

    for i in 0..self.t.len() {
      let mut timer = self.t[i];

      if timer.timer_id == 2 {
        // always tick the div8 clock
        self.div8 += cycles;
        let ticks = self.div8 / 8;
        self.div8 &= 7;

        if timer.can_run_div8() {
          if timer.tick(ticks) {
            self.assert_interrupt(&mut timer);
          }
          already_ran = true;
        }
      }
      if !already_ran && timer.can_run() && timer.tick(cycles) {
        self.assert_interrupt(&mut timer);
      }
      self.t[i] = timer;
    }
  }

  pub fn set_hblank(&mut self, value: bool) {
    let mut timer = self.t[0];
    let trigger_irq = timer.check_sync_mode(value);

    if trigger_irq {
      self.assert_interrupt(&mut timer);
    }

    self.t[0] = timer;

    if value {
      let mut timer = self.t[1];
      if timer.can_run_hblank_clock() && timer.tick(1) {
        self.assert_interrupt(&mut timer);

      }
      self.t[1] = timer;
    }
  }

  pub fn set_vblank(&mut self, value: bool) {
    let trigger_irq = self.t[1].check_sync_mode(value);

    if trigger_irq {
      let mut timer = self.t[1];
      self.assert_interrupt(&mut timer);
      self.t[1] = timer;
    }
  }

  pub fn tick_dotclock(&mut self, cycles: i32) {
    let mut timer = self.t[0];

    if timer.can_run_dotclock() && timer.tick(cycles) {
      self.assert_interrupt(&mut timer);
    }

    self.t[0] = timer;
  }


  pub fn assert_interrupt(&mut self, timer: &mut Timer) {
    let mut interrupts = self.interrupts.get();

    let interrupt_type = match timer.timer_id {
      0 => Interrupt::Timer0,
      1 => Interrupt::Timer1,
      2 => Interrupt::Timer2,
      _ => unreachable!("can't happen")
    };

    interrupts.status.set_interrupt(interrupt_type);

    self.interrupts.set(interrupts);
  }


  pub fn write(&mut self, address: u32, value: u16) {
    let timer_id = ((address & 0x30) >> 4) as usize;
    let offset = address & 0xc;

    let timer = &mut self.t[timer_id];

    match offset {
      0 => {
        timer.value = value;
        timer.irq_inhibit = false;
      }
      4 => {
        // timer is reset to 0 on writes to mode
        timer.value = 0;
        timer.mode.write(value);
        timer.irq_inhibit = false;
        timer.xblank_occurred = false;
      }
      8 => timer.target_value = value,
      _ => panic!("unsupported offset given to timer io: {offset}")
    }
  }
}