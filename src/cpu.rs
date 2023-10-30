use crate::cpu::instruction::Instruction;

use self::bus::Bus;

pub mod bus;
pub mod execute;
pub mod instruction;

struct OutRegister {
  pub val: u32,
  pub reg: usize
}

impl OutRegister {
  pub fn new(val: u32, reg: usize) -> Self {
    Self {
      val,
      reg
    }
  }
}

pub struct CPU {
  pub pc: u32,
  pub r: [u32; 32],
  pub sr: u32,
  hi: u32,
  low: u32,
  bus: Bus,
  pipeline: [u32; 2],
  previous_pc: u32,
  delayed_register: usize,
  delayed_load: Option<u32>,
  out_registers: Vec<OutRegister>
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      pc: 0xbfc0_0000,
      sr: 0,
      previous_pc: 0,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios),
      pipeline: [0; 2],
      delayed_register: 0,
      delayed_load: None,
      out_registers: Vec::new()
    }
  }

  pub fn step(&mut self) {
    let next_instr = self.bus.mem_read_32(self.pc);

    let instr = self.pipeline[0];

    self.pipeline[0] = self.pipeline[1];
    self.pipeline[1] = next_instr;

    if let Some(delayed_load) = self.delayed_load {
      self.set_reg(self.delayed_register, delayed_load)
    }

    self.delayed_load = None;
    self.delayed_register = 0;

    println!("executing instruction {:032b} at address {:08x}", instr, self.previous_pc);

    self.pc = self.pc.wrapping_add(4);

    let should_update = self.execute(Instruction::new(instr));

    if should_update {
      self.previous_pc = self.pc - 4;
    }

    while !self.out_registers.is_empty() {
      let register = self.out_registers.pop().unwrap();
      self.r[register.reg] = register.val;
    }
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.out_registers.push(OutRegister::new(val, rt))
    }
  }
}