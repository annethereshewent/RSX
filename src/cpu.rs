use crate::cpu::instruction::Instruction;

use self::bus::Bus;

pub mod bus;
pub mod execute;
pub mod instruction;

pub struct CPU {
  pub pc: u32,
  pub r: [u32; 32],
  hi: u32,
  low: u32,
  bus: Bus
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      pc: 0xbfc0000,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios)
    }
  }
  pub fn step(&mut self) {
    let instr = self.bus.mem_read_32(self.pc);

    println!("{:032b}", instr);

    self.execute(Instruction::new(instr));

    self.pc = self.pc.wrapping_add(4);
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.r[rt] = val
    }
  }
}