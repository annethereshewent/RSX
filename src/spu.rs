use std::{rc::Rc, cell::Cell};

use crate::cpu::interrupt::interrupt_registers::InterruptRegisters;
use self::voices::Voice;

pub mod voices;
pub mod adsr;


pub struct SPU {
  interrupts: Rc<Cell<InterruptRegisters>>,
  cpu_cycles: i32,
  voices: [Voice; 24]
}

pub const CPU_TO_APU_CYCLES: i32 = 768;

impl SPU {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      interrupts,
      cpu_cycles: 0,
      voices: [Voice::new(); 24]
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

  }

  pub fn read_32(&self, address: u32) -> u32 {
    (self.read_16(address) as u32) | (self.read_16(address) as u32) << 16
  }

  pub fn read_16(&self, address: u32) -> u16 {
    match address {
      0x1f80_1c00..=0x1f80_1d7f => {
        let voice = ((address >> 4) & 0x1f) as usize;
        let offset = address & 0xf;

        self.voices[voice].read_16(offset)
      }
      _ => panic!("unsupported SPU address")
    }
  }

  pub fn write_16(&mut self, address: u32, val: u16) {
    match address {
      0x1f80_1c00..=0x1f80_1d7f => {
        let voice = ((address >> 4) & 0x1f) as usize;
        let offset = address & 0xf;

        self.voices[voice].write_16(offset, val);
      },
      _ => panic!("unsupported SPU address")
    }
  }
}