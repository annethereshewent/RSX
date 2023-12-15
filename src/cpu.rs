use std::{rc::Rc, cell::Cell, fs::{File, self}};

use crate::{cpu::instruction::Instruction, gpu::{CYCLES_PER_SCANLINE, NUM_SCANLINES_PER_FRAME, GPU_FREQUENCY}, util};

use self::{bus::Bus, dma::DMA, interrupt::interrupt_registers::InterruptRegisters, gte::Gte};

pub mod bus;
pub mod execute;
pub mod instruction;
pub mod dma;
pub mod counter;
pub mod interrupt;
pub mod timers;
pub mod gte;
pub mod mdec;

// 33.868MHZ
pub const CPU_FREQUENCY: f64 = 33_868_800.0;

pub const CYCLES_PER_FRAME: i64 = ((CYCLES_PER_SCANLINE * NUM_SCANLINES_PER_FRAME) as f64 * (CPU_FREQUENCY / GPU_FREQUENCY)) as i64;


#[derive(Clone, Copy)]
struct IsolatedCacheLine {
  valid: usize,
  tag: u32,
  data: [u32; 4]
}

impl IsolatedCacheLine {
  pub fn new() -> Self {
    Self {
      valid: 0xdeadbeef,
      tag: 0xdeadbeef,
      data: [0xdeadbeef; 4]
    }
  }
}

#[derive(Clone, Copy)]
pub enum Cause {
  Interrupt = 0x0,
  LoadAddressError = 0x4,
  StoreAddressError = 0x5,
  IBusError = 0x6,
  DBusError = 0x7,
  SysCall = 0x8,
  Break = 0x9,
  IllegalInstruction = 0xa,
  CoprocessorError = 0xb,
  Overflow = 0xc
}

pub struct COP0 {
  pub sr: u32,
  pub cause: u32,
  pub epc: u32,
  pub jumpdest: u32,
  pub bad_vaddr: u32,
  pub dcic: u32
}

impl COP0 {
  pub fn bev(&self) -> bool {
    (self.sr >> 22) & 0b1 == 1
  }

  pub fn is_cache_disabled(&self) -> bool {
    self.sr & 0x10000 == 0
  }

  pub fn interrupts_ready(&self) -> bool {
    self.sr & 0b1 == 1 && self.interrupt_mask() != 0
  }

  pub fn interrupt_mask(&self) -> u8 {
    ((self.sr >> 8) as u8) & ((self.cause >> 8) as u8)
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

    self.cause &= !0x7c;
    self.cause |= (cause as u32) << 2;

    exception_address
  }

  pub fn return_from_exception(&mut self) {
    let mode = self.sr & 0x3f;
    self.sr &= !0xf;
    self.sr |= mode >> 2;
  }

  pub fn set_interrupt(&mut self, set_active: bool) {
    if set_active {
      self.cause |= 1 << 10;
    } else {
      self.cause &= !(1 << 10);
    }
  }
}

pub struct CPU {
  pub pc: u32,
  pub next_pc: u32,
  current_pc: u32,
  pub r: [u32; 32],
  cop0: COP0,
  branch: bool,
  delay_slot: bool,
  hi: u32,
  low: u32,
  pub bus: Bus,
  load: Option<(usize, u32)>,
  dma: Rc<Cell<DMA>>,
  interrupts: Rc<Cell<InterruptRegisters>>,
  current_instruction: u32,
  isolated_cache: [IsolatedCacheLine; 256],
  pub gte: Gte,
  pub debug_on: bool,
  output: String,
  pub exe_file: Option<String>
}

impl CPU {
  pub fn new(bios: Vec<u8>, game_file: File) -> Self {
    let interrupts = Rc::new(Cell::new(InterruptRegisters::new()));
    let dma = Rc::new(Cell::new(DMA::new()));

    Self {
      pc: 0xbfc0_0000,
      next_pc: 0xbfc0_0004,
      current_pc: 0xbfc0_0000,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios, interrupts.clone(), dma.clone(), game_file),
      load: None,
      branch: false,
      delay_slot: false,
      cop0: COP0 {
        sr: 0,
        cause: 0,
        epc: 0,
        jumpdest: 0,
        bad_vaddr: 0,
        dcic: 0
      },
      dma,
      interrupts,
      current_instruction: 0,
      isolated_cache: [IsolatedCacheLine::new(); 256],
      gte: Gte::new(),
      debug_on: false,
      output: "".to_string(),
      exe_file: None
    }
  }

  pub fn exception(&mut self, cause: Cause) {
    let exception_address = self.cop0.enter_exception(cause);

    self.cop0.epc = match cause {
      Cause::Interrupt => self.pc,
      _ => self.current_pc
    };

    let coprocessor_exception = if matches!(cause, Cause::Break) {
      0
    } else {
      (self.current_instruction >> 26) & 0b11
    };

    self.cop0.cause |= coprocessor_exception << 28;

    if self.delay_slot {
      self.cop0.epc = self.cop0.epc.wrapping_sub(4);
      self.cop0.cause |= 1 << 31;
      self.cop0.jumpdest = self.pc;
    } else {
      self.cop0.cause &= !(1 << 31);
    }

    self.pc = exception_address;
    self.next_pc = self.pc.wrapping_add(4);
  }

  pub fn check_irqs(&mut self) {
    self.cop0.set_interrupt(self.interrupts.get().pending());
  }

  pub fn step(&mut self) {
    let mut dma = self.dma.get();

    if dma.is_active() {
      if dma.in_gap() {
        dma.tick_gap();
        self.dma.set(dma);

        if !dma.chopping_enabled() {
          return;
        }
      } else {
        let count = dma.tick(&mut self.bus);
        self.dma.set(dma);

        self.bus.tick(count);

        return;
      }
    }

    self.current_pc = self.pc;

    if self.current_pc == 0x80030000 {
      if let Some(exe_file) = &self.exe_file {
        let exe_file = exe_file.clone();
        self.load_exe(exe_file.as_str());
      }
    }

    self.check_irqs();

    let instr = self.fetch_instruction();
    self.current_instruction = instr;

    self.delay_slot = self.branch;
    self.branch = false;

    if self.current_pc & 0b11 != 0 {
      self.cop0.bad_vaddr = self.current_pc;
      self.exception(Cause::LoadAddressError);

      self.execute_load_delay();

      return;
    }

    // check if we need to handle an interrupt by checking cop0 status register and interrupt mask bits in cause and sr
    if self.cop0.interrupts_ready() {
      self.exception(Cause::Interrupt);

      self.execute_load_delay();

      return;
    }

    if self.debug_on {
      println!("executing instruction {:032b} at address {:08x}", instr, self.current_pc);
    }

    self.update_tty();

    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    self.tick_instruction();

    self.execute(Instruction::new(instr));
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.r[rt] = val;
    }
  }

  fn update_tty(&mut self) {
    if self.pc == 0xb0 && self.r[9] == 0x3d {
      let mut buf: Vec<u8> = Vec::new();

      buf.push(self.r[4] as u8);
      buf.push((self.r[4] >> 8) as u8);
      buf.push((self.r[4] >> 16) as u8);
      buf.push((self.r[4] >> 24 ) as u8);


      self.output += &String::from_utf8(buf).unwrap();

      if self.output.contains("\n") {
        print!("{}", self.output);
        self.output = "".to_string();
      }
    }
  }

  pub fn load_exe(&mut self, filename: &str) {
    let bytes = fs::read(filename).unwrap();

    let mut index = 0x10;

    self.pc = util::read_word(&bytes, index);
    self.next_pc = self.pc + 4;

    index += 4;

    self.r[28] = util::read_word(&bytes, index);

    index += 4;

    let file_dest = util::read_word(&bytes, index);

    index += 4;

    let file_size = util::read_word(&bytes, index);

    index += 0x10 + 4;

    let sp_base = util::read_word(&bytes, index);

    index += 4;

    if sp_base != 0 {
      let sp_offset = util::read_word(&bytes, index);

      self.r[29] = sp_base + sp_offset;
      self.r[30] = self.r[29];
    }

    index = 0x800;

    for i in 0..file_size {
      self.bus.ram[((file_dest + i) & 0x1f_ffff) as usize] = bytes[index];
      index += 1;
    }
  }

  fn write_to_cache(&mut self, address: u32, value: u32) {
    let line = ((address >> 4) & 0xff) as usize;
    let index = ((address >> 2) & 0b11) as usize;

    let cache_line = &mut self.isolated_cache[line];

    if (self.bus.cache_control >> 2) & 0b1 == 1 {
      cache_line.tag = value;
    } else {
      cache_line.data[index] = value;
    }

    // this invalidates the cache line, as index cannot be greater than 3
    cache_line.valid = 4;
  }

  pub fn read_from_cache(&mut self, address: u32) -> u32 {
    let line = ((address >> 4) & 0xff) as usize;
    let index = ((address >> 2) & 0b11) as usize;

    let cache_line = &mut self.isolated_cache[line];

    if (self.bus.cache_control >> 2) & 0b1 == 1 {
      return cache_line.tag;
    }

    cache_line.data[index]
  }

  fn fetch_instruction_cache(&mut self) -> u32 {
    let tag = self.pc & 0x7ffff000;

    let line = ((self.pc >> 4) & 0xff) as usize;
    let index = ((self.pc >> 2) & 0x3) as usize;

    let address = Bus::translate_address(self.pc);

    let cache_line = &mut self.isolated_cache[line];

    if (cache_line.tag != tag) || (cache_line.valid > index) {
      // invalidate the cache
      let mut address = (address & !0xf) + (4 * index as u32);

      for i in index..4 {
        let value = self.bus.mem_read_32(address);

        cache_line.data[i] = value;

        address += 4;
      }

      cache_line.tag = tag;
      cache_line.valid = index;

      self.bus.tick(5);
    }

    cache_line.data[index]
  }

  pub fn fetch_instruction(&mut self) -> u32 {
    if self.bus.cache_enabled() && self.pc < 0xa0000000 {
      return self.fetch_instruction_cache();
    }

    self.bus.tick(5);

    self.bus.mem_read_32(self.pc)
  }

  pub fn store_32(&mut self, address: u32, value: u32) {
    if !self.cop0.is_cache_disabled() {
      self.write_to_cache(address, value);

      return;
    }

    self.bus.tick(5);

    self.bus.mem_write_32(address, value);
  }

  pub fn store_16(&mut self, address: u32, value: u16) {
    if !self.cop0.is_cache_disabled() {
      self.write_to_cache(address, value as u32);

      return;
    }

    self.bus.tick(5);

    self.bus.mem_write_16(address, value)
  }

  pub fn store_8(&mut self, address: u32, value: u8) {
    if !self.cop0.is_cache_disabled() {
      self.write_to_cache(address, value as u32);

      return;
    }

    self.bus.tick(5);

    self.bus.mem_write_8(address, value)
  }

  // TODO: refactor this into just one method
  pub fn load_32(&mut self, address: u32) -> u32 {
    if !self.cop0.is_cache_disabled() {
      return self.read_from_cache(address);
    }

    self.bus.tick(5);

    self.bus.mem_read_32(address)
  }

  pub fn load_16(&mut self, address: u32) -> u16 {
    if !self.cop0.is_cache_disabled() {
      return self.read_from_cache(address) as u16;
    }

    self.bus.tick(5);

    self.bus.mem_read_16(address)
  }

  pub fn load_8(&mut self, address: u32) -> u8 {
    if !self.cop0.is_cache_disabled() {
      return self.read_from_cache(address) as u8;
    }

    self.bus.tick(5);

    self.bus.mem_read_8(address)
  }

  pub fn tick_instruction(&mut self) {
    self.bus.tick(1);
  }
}