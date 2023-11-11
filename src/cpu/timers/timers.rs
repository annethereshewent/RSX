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

  pub fn read(&self, address: u32) -> u16 {
    let timer_id = ((address & 0x30) >> 4) as usize;
    let offset = address & 0xcc;

    let timer = self.t[timer_id];

    match offset {
      0 => timer.value,
      4 => timer.mode.val,
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

        if timer.run_div8() {
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
      }
      8 => timer.target_value = value,
      _ => panic!("unsupported offset given to timer io: {offset}")
    }
  }
}