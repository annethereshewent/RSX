use crate::cpu::instruction::Instruction;

use self::bus::Bus;

pub mod bus;
pub mod execute;
pub mod instruction;

pub enum Cause {
  SysCall = 0x8,

}

pub struct COP0 {
  pub sr: u32,
  pub cause: u32,
  pub epc: u32
}

impl COP0 {
  pub fn bev(&self) -> bool {
    (self.sr >> 22) & 0b1 == 1
  }

  pub fn is_cache_disabled(&self) -> bool {
    self.sr & 0x10000 == 0
  }

  pub fn enter_exception(&mut self, cause: Cause) -> u32 {
    let exception_address: u32 = if self.bev() {
      0xbfc0_0180
    } else {
      0x8000_0080
    };

    let mode = self.sr & 0x3f;
    self.sr &=  !0x3f;
    self.sr |= (mode << 2) & 0x3f;

    self.cause = (cause as u32) << 2;

    exception_address
  }
}

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
  pub next_pc: u32,
  current_pc: u32,
  pub r: [u32; 32],
  pub cop0: COP0,
  hi: u32,
  low: u32,
  bus: Bus,
  delayed_register: usize,
  delayed_load: Option<u32>,
  out_registers: Vec<OutRegister>
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      pc: 0xbfc0_0000,
      next_pc: 0xbfc0_0004,
      current_pc: 0xbfc0_0000,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios),
      delayed_register: 0,
      delayed_load: None,
      out_registers: Vec::new(),
      cop0: COP0 {
        sr: 0,
        cause: 0,
        epc: 0
      }
    }
  }

  pub fn exception(&mut self, cause: Cause) {
    let exception_address = self.cop0.enter_exception(cause);

    self.cop0.epc = self.current_pc;
    self.pc = exception_address;
    self.next_pc = self.pc.wrapping_add(4);
  }

  pub fn step(&mut self) {
    self.current_pc = self.pc;
    let instr = self.bus.mem_read_32(self.pc);

    if let Some(delayed_load) = self.delayed_load {
      self.set_reg(self.delayed_register, delayed_load)
    }

    self.delayed_load = None;
    self.delayed_register = 0;

    // println!("executing instruction {:032b} at address {:08x}", instr, self.current_pc);

    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    self.execute(Instruction::new(instr));

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