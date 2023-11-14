use std::{rc::Rc, cell::Cell};

use crate::cpu::interrupt::interrupt_registers::InterruptRegisters;
use self::{voices::Voice, spu_control::{SpuControlRegister, RamTransferMode}, reverb::Reverb};

pub mod voices;
pub mod adsr;
pub mod spu_control;
pub mod reverb;

pub const FIFO_CAPACITY: usize = 32;
pub const SPU_RAM_SIZE: usize = 0x80000; // 512 kb

struct DataTransfer {
  control: u16,
  transfer_address: u32,
  current_address: u32,
  fifo: Vec<u16>,
}

impl DataTransfer {
  pub fn new() -> Self {
    Self {
      control: 0,
      transfer_address: 0,
      current_address: 0,
      fifo: Vec::with_capacity(FIFO_CAPACITY)
    }
  }
}

pub struct SPU {
  interrupts: Rc<Cell<InterruptRegisters>>,
  cpu_cycles: i32,
  voices: [Voice; 24],
  volume_left: i16,
  volume_right: i16,
  reverb_volume_left: i16,
  reverb_volume_right: i16,
  external_volume_left: i16,
  external_volume_right: i16,
  current_volume_left: i16,
  current_volume_right: i16,
  cd_volume_left: i16,
  cd_volume_right: i16,
  key_on: u32,
  key_off: u32,
  modulate_on: u32,
  noise_on: u32,
  echo_on: u32,
  control: SpuControlRegister,
  irq_status: bool,
  data_transfer: DataTransfer,
  sound_ram: Box<[u16]>,
  reverb: Reverb,
  endx: u32
}

pub const CPU_TO_APU_CYCLES: i32 = 768;

impl SPU {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      interrupts,
      cpu_cycles: 0,
      voices: [Voice::new(); 24],
      volume_left: 0,
      volume_right: 0,
      reverb_volume_left: 0,
      reverb_volume_right: 0,
      external_volume_left: 0,
      external_volume_right: 0,
      current_volume_left: 0,
      current_volume_right: 0,
      cd_volume_left: 0,
      cd_volume_right: 0,
      key_on: 0,
      key_off: 0,
      noise_on: 0,
      echo_on: 0,
      modulate_on: 0,
      control: SpuControlRegister::new(),
      irq_status: false,
      data_transfer: DataTransfer::new(),
      sound_ram: vec![0; SPU_RAM_SIZE / 2].into_boxed_slice(),
      reverb: Reverb::new(),
      endx: 0
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

  fn update_echo(&mut self) {
    for i in 0..self.voices.len() {
      if (self.echo_on >> i) & 0b1 == 1 {
        self.voices[i].update_echo();
      }
    }
  }

  fn update_key_off(&mut self) {
    for i in 0..self.voices.len() {
      if (self.key_off >> i) & 0b1 == 1 {
        self.voices[i].update_key_off();
      }
    }
    self.key_off = 0;
  }

  fn update_key_on(&mut self) {
    for i in 0..self.voices.len() {
      if (self.key_on >> i) & 0b1 == 1 {
        self.voices[i].update_key_on();
      }
    }

    self.key_on = 0;
  }

  fn update_noise(&mut self) {
    for i in 0..self.voices.len() {
      self.voices[i].update_noise((self.noise_on >> i) & 0b1 == 1);
    }
  }

  fn update_endx(&mut self) {
    self.endx = 0;

    for i in 0..self.voices.len() {
      if self.voices[i].endx {
        self.endx |= 1 << i;
      }
    }
  }


  // tick for one APU cycle
  fn tick(&mut self) {
    let mut output_left = 0.0;
    let mut output_right = 0.0;

    let mut modulator: i16 = 0;

    self.update_endx();

    for i in 0..self.voices.len() {
      let voice = &mut self.voices[i];

      if voice.is_disabled() {
        continue;
      }

      let (sample_left, sample_right) = voice.get_samples();

      output_left += sample_left;
      output_right += sample_right;

      let should_modulate = (self.modulate_on >> i) & 0b1 == 1;

      voice.tick(i > 0 && should_modulate, modulator, &mut self.sound_ram);

      modulator = voice.modulator;
    }

    // push the samples into the audio buffer
    if output_left > 0.0 || output_right > 0.0 {
      println!("received samples {output_left} and {output_right}");
    }
  }

  fn push_fifo(&mut self, val: u16) {
    if self.data_transfer.fifo.len() < FIFO_CAPACITY {
      self.data_transfer.fifo.push(val);
    }
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
      0x1f80_1d80 => self.volume_left as u16,
      0x1f80_1d82 => self.volume_right as u16,
      0x1f80_1d84 => self.reverb_volume_left as u16,
      0x1f80_1d86 => self.reverb_volume_right as u16,
      0x1f80_1d88 => self.key_on as u16,
      0x1f80_1d8a => (self.key_on >> 16) as u16,
      0x1f80_1d8c => self.key_off as u16,
      0x1f80_1d8e => (self.key_off >> 16) as u16,
      0x1f80_1d90 => self.modulate_on as u16,
      0x1f80_1d92 => (self.modulate_on >> 16) as u16,
      0x1f80_1d94 => self.noise_on as u16,
      0x1f80_1d96 => (self.noise_on >> 16) as u16,
      0x1f80_1d98 => self.echo_on as u16,
      0x1f80_1d9a => (self.echo_on >> 16) as u16,
      0x1f80_1d9c => self.endx as u16,
      0x1f80_1d9e => (self.endx >> 16) as u16,
      0x1f80_1da2 => (self.reverb.mbase / 8) as u16,
      0x1f80_1daa => self.control.read(),
      0x1f80_1dac => self.data_transfer.control,
      0x1f80_1dae => {
        let control = self.control.read();

        let mut value = (control & 0x20) << 2;
        value |= (self.irq_status as u16) << 6;
        value |= control & 0x3f;

        value
      }
      0x1f80_1db0 => self.cd_volume_left as u16,
      0x1f80_1db2 => self.cd_volume_right as u16,
      0x1f80_1db4 => self.external_volume_left as u16,
      0x1f80_1db6 => self.external_volume_right as u16,
      0x1f80_1db8 => self.current_volume_left as u16,
      0x1f80_1dba => self.current_volume_right as u16,
      _ => panic!("reading from unsupported SPU address: {:X}", address)
    }
  }

  pub fn write_16(&mut self, address: u32, val: u16) {
    match address {
      0x1f80_1c00..=0x1f80_1d7f => {
        let voice = ((address >> 4) & 0x1f) as usize;
        let offset = address & 0xf;

        self.voices[voice].write_16(offset, val);
      },
      0x1f80_1d80 => self.volume_left = val as i16,
      0x1f80_1d82 => self.volume_right = val as i16,
      0x1f80_1d84 => self.reverb_volume_left = val as i16,
      0x1f80_1d86 => self.reverb_volume_right = val as i16,
      0x1f80_1d88 => {
        self.key_on &= 0xffff0000;
        self.key_on |= val as u32;

        self.update_key_on();
      }
      0x1f80_1d8a => {
        self.key_on &= 0xffff;
        self.key_on |= (val as u32) << 16;

        self.update_key_on();
      }
      0x1f80_1d8c => {
        self.key_off &= 0xffff0000;
        self.key_off |= val as u32;

        self.update_key_off();
      }
      0x1f80_1d8e => {
        self.key_off &= 0xffff;
        self.key_off |= (val as u32) << 16;

        self.update_key_off();
      }
      0x1f80_1d90 => {
        self.modulate_on &= 0xffff0000;
        self.modulate_on |= val as u32;
      }
      0x1f80_1d92 => {
        self.modulate_on &= 0xffff;
        self.modulate_on |= (val as u32) << 16;
      }
      0x1f80_1d94 => {
        self.noise_on &= 0xffff0000;
        self.noise_on |= val as u32;

        self.update_noise();
      }
      0x1f80_1d96 => {
        self.noise_on &= 0xffff;
        self.noise_on |= (val as u32) << 16;

        self.update_noise();
      }
      0x1f80_1d98 => {
        self.echo_on &= 0xffff0000;
        self.echo_on |= val as u32;

        self.update_echo();
      }
      0x1f80_1d9a => {
        self.echo_on &= 0xffff;
        self.echo_on |= (val as u32) << 16;

        self.update_echo();
      }
      0x1f80_1da2 => self.reverb.write_mbase(val),
      0x1f80_1da6 => {
        self.data_transfer.transfer_address = (val as u32) * 8;
        self.data_transfer.current_address = (val as u32) * 8;
      }
      0x1f80_1da8 => self.push_fifo(val),
      0x1f80_1daa => {
        self.control.write(val);

        if !self.control.irq9_enable() {
          self.irq_status = false;
        }

        if self.control.transfer_mode() == RamTransferMode::ManualWrite {
          // do the manual transfer from fifo to sound ram
          while !self.data_transfer.fifo.is_empty() {
            let sample = self.data_transfer.fifo.remove(0);

            let address = self.data_transfer.current_address;

            self.sound_ram[(address / 2 ) as usize] = sample;

            self.data_transfer.current_address = (self.data_transfer.current_address + 2) & 0x7ffff;
          }
        }
      }
      0x1f80_1dac => self.data_transfer.control = val,
      0x1f80_1db0 => self.cd_volume_left = val as i16,
      0x1f80_1db2 => self.cd_volume_right = val as i16,
      0x1f80_1db4 => self.external_volume_left = val as i16,
      0x1f80_1db6 => self.external_volume_right = val as i16,
      0x1f80_1db8 => self.current_volume_left = val as i16,
      0x1f80_1dba => self.current_volume_right = val as i16,
      0x1f80_1dc0..=0x1f80_1dff => self.reverb.write_16(address, val),
      _ => panic!("writing to unsupported SPU address: {:X}", address)
    }
  }
}