use std::{cell::Cell, collections::VecDeque, fs::File, rc::Rc, sync::{Arc, Mutex}};

use crate::{gpu::GPU, spu::SPU, cdrom::Cdrom, controllers::Controllers};

use super::{counter::Counter, interrupt::interrupt_registers::InterruptRegisters, timers::timers::Timers, dma::DMA, mdec::Mdec};

const RAM_SIZE: usize = 2 * 1024 * 1024;

const EXP2_WRITE_ADDR: u32 = 0x1f802021;
const EXP2_READ_ADDR: u32 = 0x1f802023;

#[derive(Copy, Clone)]
pub enum Device {
  Timers = 0,
  Controllers = 1,
  SPU = 2,
  GPU = 3
}

impl Device {
  pub fn from(index: usize) -> Self {
    match index {
      0 => Device::Timers,
      1 => Device::Controllers,
      2 => Device::SPU,
      3 => Device::GPU,
      _ => panic!("invalid device specified: {index}")
    }
  }
}

// @TODO: Refactor all of the mem_read and mem_loads into one generic method
pub struct Bus {
  bios: Vec<u8>,
  pub ram: Box<[u8]>,
  pub counter: Counter,
  pub gpu: GPU,
  pub spu: SPU,
  pub cdrom: Cdrom,
  pub interrupts: Rc<Cell<InterruptRegisters>>,
  pub timers: Timers,
  dma: Rc<Cell<DMA>>,
  pub mdec: Mdec,
  pub controllers: Controllers,
  pub cycles: i32,
  pub cache_control: u32,
  exp2_buffer: Vec<u8>,
  scratchpad: Box<[u8]>,
  last_device_sync: [i32; 4],
  pub last_sync: i32
}

impl Bus {
  pub fn new(
    bios: Vec<u8>,
    interrupts: Rc<Cell<InterruptRegisters>>,
    dma: Rc<Cell<DMA>>,
    game_file: Option<File>,
    game_bytes: Option<Vec<u8>>,
    is_wasm: bool,
    audio_samples: Arc<Mutex<VecDeque<i16>>>) -> Self
  {
    Self {
      bios,
      ram: vec![0; RAM_SIZE].into_boxed_slice(),
      gpu: GPU::new(interrupts.clone()),
      spu: SPU::new(audio_samples),
      timers: Timers::new(interrupts.clone()),
      cdrom: Cdrom::new(interrupts.clone(), game_file, game_bytes),
      controllers: Controllers::new(interrupts.clone(), is_wasm),
      counter: Counter::new(),
      interrupts,
      dma,
      cycles: 0,
      cache_control: 0,
      exp2_buffer: Vec::new(),
      mdec: Mdec::new(),
      scratchpad: vec![0; 0x400].into_boxed_slice(),
      last_device_sync: [0; 4],
      last_sync: 0
    }
  }

  pub fn translate_address(address: u32) -> u32 {
    match address >> 29 {
      0b000..=0b011 => address,
      0b100 => address & 0x7fff_ffff,
      0b101 => address & 0x1fff_ffff,
      0b110..=0b111 => address,
      _ => unreachable!("not possible")
    }
  }

  pub fn mem_read_8(&mut self, address: u32) -> u8 {
    let address = Bus::translate_address(address);

    match address {
      0x1f00_0000..=0x1f08_0000 => 0xff,
      0x1f80_0000..=0x1f80_03ff => {
        let offset = (address - 0x1f80_0000) as usize;

        self.scratchpad[offset]
      }
      0x1f80_1040 => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.read_byte()
      }
      0x1f80_1080..=0x1f80_10ff => self.dma.get().read(address) as u8,
      0x1f80_1800..=0x1f80_1803 => self.cdrom.read(address),
      0x1fc0_0000..=0x1fc7_ffff => self.bios[(address - 0x1fc0_0000) as usize],
      0x0000_0000..=0x001f_ffff => {
        self.ram[address as usize]
      }
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_read_32(&mut self, address: u32) -> u32 {
    if (address & 0b11) != 0 {
      panic!("unaligned address received: {:08x}", address);
    }

    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => {
        let offset = address as usize;
        (self.ram[offset] as u32) | ((self.ram[offset + 1] as u32) << 8) | ((self.ram[offset + 2] as u32) << 16) | ((self.ram[offset + 3] as u32) << 24)

      }
      // 0x1f00_0000..=0x1f08_0000 => 0xffffffff,
      0x1fc0_0000..=0x1fc7_ffff => {
        let offset = (address - 0x1fc0_0000) as usize;
        (self.bios[offset] as u32) | ((self.bios[offset + 1] as u32) << 8) | ((self.bios[offset + 2] as u32) << 16) | ((self.bios[offset + 3] as u32) << 24)
      }
      0x1f80_0000..=0x1f80_03ff => {
        let offset = (address - 0x1f80_0000) as usize;

        (self.scratchpad[offset] as u32) | (self.scratchpad[offset + 1] as u32) << 8 | (self.scratchpad[offset + 2] as u32) << 16 | (self.scratchpad[offset + 3] as u32) << 24
      }
      0x1f80_1014 => 0x2009_31e1,
      0x1f80_1044 => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.read_stat() as u32
      }
      0x1f80_1060 => 0xb88,
      0x1f80_1070 => {
        self.interrupts.get().status.read()
      }
      0x1f80_1074 => {
        self.interrupts.get().mask.read()
      }
      0x1f80_1080..=0x1f80_10ff => self.dma.get().read(address),
      0x1f80_1100..=0x1f80_112b => {
        self.tick_device(Device::Timers);

        self.timers.read(address) as u32
      }
      0x1f80_1810..=0x1f80_1817 => {

        let offset = address - 0x1f80_1810;

        match offset {
          0 => {
            self.tick_device(Device::GPU);

            self.gpu.gpuread()
          }
          4 => {
            self.tick_device(Device::GPU);

            self.gpu.stat_value()
          }
          _ => panic!("invalid GPU register: {offset}")
        }
      }
      0x1f80_1824 => self.mdec.read_status(),
      0x1f80_1c00..=0x1f80_1e7f => {
        self.tick_device(Device::SPU);

        self.spu.read_32(address)
      }
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_read_16(&mut self, address: u32) -> u16 {
    if (address & 0b1) != 0 {
      panic!("unaligned address received: {:032b}", address);
    }

    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => {

        let offset = address as usize;
        (self.ram[offset] as u16) | ((self.ram[offset + 1] as u16) << 8)
      }
      // 0x1f00_0000..=0x1f08_0000 => 0xffffffff,
      0x1f80_0000..=0x1f80_03ff => {
        let offset = (address - 0x1f80_0000) as usize;

        (self.scratchpad[offset] as u16) | (self.scratchpad[offset + 1] as u16) << 8
      }
      0x1f80_1100..=0x1f80_112b => {
        self.tick_device(Device::Timers);

        self.timers.read(address) as u16
      }
      0x1fc0_0000..=0x1fc7_ffff => {
        let offset = (address - 0x1fc0_0000) as usize;
        (self.bios[offset] as u16) | ((self.bios[offset + 1] as u16) << 8)
      }

      0x1f80_1c00..=0x1f80_1e7f => {
        self.tick_device(Device::SPU);

        self.spu.read_16(address)
      }
      0x1f80_1044 => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.read_stat() as u16
      }
      0x1f80_104a => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.read_control()
      }
      0x1f80_1070 => {

        self.interrupts.get().status.read() as u16
      }
      0x1f80_1074 => {

        self.interrupts.get().mask.read() as u16
      }
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_write_8(&mut self, address: u32, value: u8) {
    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => self.ram[address as usize] = value,
      0x1f80_0000..=0x1f80_03ff => {
        let offset = (address - 0x1f80_0000) as usize;

        self.scratchpad[offset] = value;
      }
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1040 => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.queue_byte(value);
      }
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_1070..=0x1f80_1074 => panic!("unimplemented writes to interrupt registers"),
      0x1f80_1080..=0x1f80_10ff => {
        let mut dma = self.dma.get();

        dma.write(address, value as u32);

        self.dma.set(dma);
      }
      0x1f80_1800..=0x1f80_1803 => self.cdrom.write(address, value),
      0x1f80_1100..=0x1f80_112b => {
        self.tick_device(Device::Timers);

        self.timers.write(address, value as u32);
      }
      0x1f80_1c00..=0x1f80_1e7f => panic!("8 bit writes to spu not supported"),
      0x1f80_2000..=0x1f80_207f  => self.write_expansion_2(address, value),
      0xfffe_0130 => self.cache_control = value as u32,
      _ => panic!("write to unsupported address: {:08x}", address)
    }
  }

  pub fn mem_write_16(&mut self, address: u32, value: u16) {
    if (address & 0b1) != 0 {
      panic!("unaligned address received: {:X}", address);
    }

    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => {
        let offset = address as usize;

        self.ram[offset] = (value & 0xff) as u8;
        self.ram[offset + 1] = ((value >> 8) & 0xff) as u8;
      }
      0x1f80_0000..=0x1f80_03ff => {
        let offset = (address - 0x1f80_0000) as usize;

        self.scratchpad[offset] = value as u8;
        self.scratchpad[offset + 1] = (value >> 8) as u8;
      }
      0x1f80_1c00..=0x1f80_1e80 => {
        self.tick_device(Device::SPU);

        self.spu.write_16(address, value);
      }
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1048 => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.write_joy_mode(value);
      }
      0x1f80_104a => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.write_joy_control(value);
      }
      0x1f80_104e => {
        self.tick_device(Device::GPU);
        self.tick_device(Device::Controllers);

        self.controllers.write_reload_value(value)
      }
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_1070 => {
        let mut interrupts = self.interrupts.get();

        interrupts.acknowledge_irq(value as u32);

        self.interrupts.set(interrupts);
      }
      0x1f80_1074 => {
        let mut interrupts = self.interrupts.get();

        interrupts.mask.write(value as u32);

        self.interrupts.set(interrupts);
      }
      0x1f80_1100..=0x1f80_112b => {
        self.tick_device(Device::Timers);

        self.timers.write(address, value as u32);
      }
      0xfffe_0130 => self.cache_control = value as u32,
      _ => panic!("write to unsupported address: {:08x}", address)
    }
  }

  pub fn mem_write_32(&mut self, address: u32, value: u32) {
    if (address & 0b11) != 0 {
      panic!("unaligned address received: {:X}", address);
    }

    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => {
        let offset = address as usize;

        self.ram[offset] = (value & 0xff) as u8;
        self.ram[offset + 1] = ((value >> 8) & 0xff) as u8;
        self.ram[offset + 2] = ((value >> 16) & 0xff) as u8;
        self.ram[offset + 3] = ((value >> 24)) as u8;
      }
      0x1f80_0000..=0x1f80_03ff => {
        let offset = (address - 0x1f80_0000) as usize;

        self.scratchpad[offset] = value as u8;
        self.scratchpad[offset + 1] = (value >> 8) as u8;
        self.scratchpad[offset + 2] = (value >> 16) as u8;
        self.scratchpad[offset + 3] = (value >> 24) as u8;
      }
      0x1f80_1c00..=0x1f80_1e80 => panic!("32 bit writes to SPU not supported"),
      0x1f80_1000..=0x1f80_1023 => (), // println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => (), // println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_1070 => {
        let mut interrupts = self.interrupts.get();

        interrupts.acknowledge_irq(value);

        self.interrupts.set(interrupts);
      }
      0x1f80_1074 => {
        let mut interrupts = self.interrupts.get();

        interrupts.mask.write(value);

        self.interrupts.set(interrupts);
      }
      0x1f80_1080..=0x1f80_10ff => {
        let mut dma = self.dma.get();
        dma.write(address, value);

        self.dma.set(dma);
      }
      0x1f80_1100..=0x1f80_112b => {
        self.tick_device(Device::Timers);

        self.timers.write(address, value);
      }
      0x1f80_1810..=0x1f80_1817 => {
        let offset = address - 0x1f80_1810;

        self.tick_device(Device::GPU);

        match offset {
          0 => self.gpu.gp0(value),
          4 => self.gpu.gp1(value),
          _ => panic!("GPU write register not implemented yet: {offset}")
        }
      }
      0x1f80_1820 => self.mdec.write_command(value),
      0x1f80_1824 => self.mdec.write_control(value),
      0xfffe_0130 => self.cache_control = value,
      _ => panic!("write to unsupported address: {:06x}", address)
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    self.cycles += cycles;

    // syncing cdrom and dma on every tick since it was causing issues otherwise.
    self.cdrom.tick_counter(cycles, &mut self.spu);

    let mut dma = self.dma.get();
    dma.tick_counter(cycles);
    self.dma.set(dma);
  }

  pub fn reset_cycles(&mut self) {
    let cycles = self.cycles;
    self.cycles = 0;

    for i in 0..self.last_device_sync.len() {
      self.last_device_sync[i] -= cycles;
    }

    self.last_sync -= cycles;
  }

  pub fn sync_devices(&mut self) {
    for i in 0..self.last_device_sync.len() {
      let device = Device::from(i);

      self.tick_device(device);
    }

    self.last_sync = self.cycles;
  }

  fn tick_device(&mut self, device: Device) {
    let cycles = self.cycles - self.last_device_sync[device as usize];

    match device {
      Device::Timers => self.timers.tick(cycles),
      Device::Controllers => self.controllers.tick(cycles),
      Device::SPU => {
        let mut interrupts = self.interrupts.get();

        self.spu.tick_counter(cycles, &mut interrupts);

        self.interrupts.set(interrupts);
      }
      Device::GPU => self.gpu.tick_counter(cycles, &mut self.timers)
    }

    self.last_device_sync[device as usize] = self.cycles;
  }

  fn write_expansion_2(&mut self, address: u32, val: u8) {
    if address == EXP2_WRITE_ADDR && val != 0xd {
      if val == 0xa {
        self.exp2_buffer.clear();

        return;
      }
      self.exp2_buffer.push(val);
    }
  }

  pub fn cache_enabled(&self) -> bool {
    (self.cache_control >> 11) & 0b1 == 1
  }
}