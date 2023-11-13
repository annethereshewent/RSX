use std::{rc::Rc, cell::Cell};

use crate::cpu::interrupt::interrupt_registers::InterruptRegisters;

pub struct SPU {
  interrupts: Rc<Cell<InterruptRegisters>>,
  cpu_cycles: i32,
  cycles: i32
}

pub const CPU_TO_APU_CYCLES: i32 = 768;

impl SPU {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      interrupts,
      cycles: 0,
      cpu_cycles: 0
    }
  }

  // update counter until 768 cycles have passed
  pub fn tick_counter(&mut self, cycles: i32) {
    self.cpu_cycles += cycles;

    while self.cpu_cycles >= 768 {
      self.cpu_cycles -= 768;
      self.tick();
    }
  }

  // tick for one APU cycle
  fn tick(&mut self) {
    self.cycles += 1;
  }
}