use std::{collections::VecDeque, rc::Rc, cell::Cell};

use crate::cpu::interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt};

use self::{joy_control::JoyControl, joy_mode::JoyMode, joypad::Joypad, memory_card::MemoryCard};

pub mod joy_control;
pub mod joy_mode;
pub mod joypad;
pub mod memory_card;

#[derive(Clone, Copy, PartialEq)]
pub enum ControllerDevice {
  None,
  Controller,
  MemoryCard
}

pub struct Controllers {
  ctrl: JoyControl,
  pub joypad: Joypad,
  pub memory_card: MemoryCard,
  mode: JoyMode,
  baudrate_timer: i32,
  rx_fifo: VecDeque<u8>,
  tx_fifo: VecDeque<u8>,
  interrupt: bool,
  tx_ready_1: bool,
  tx_ready_2: bool,
  cycles: i32,
  currently_transferring: bool,
  rx_parity_error: bool,
  active_device: ControllerDevice,
  in_acknowledge: bool,
  ack_input: bool,
  interrupts: Rc<Cell<InterruptRegisters>>
}

impl Controllers {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      ctrl: JoyControl::new(),
      baudrate_timer: 0,
      mode: JoyMode::new(),
      rx_fifo: VecDeque::new(),
      tx_fifo: VecDeque::new(),
      interrupt: false,
      tx_ready_1: false,
      tx_ready_2: false,
      currently_transferring: false,
      cycles: 0,
      rx_parity_error: false,
      active_device: ControllerDevice::None,
      joypad: Joypad::new(),
      memory_card: MemoryCard::new(),
      in_acknowledge: false,
      ack_input: false,
      interrupts
    }
  }

  pub fn write_joy_control(&mut self, value: u16) {
    self.ctrl.write(value);

    if self.ctrl.reset() {
      // reset most joy registers to 0
      self.write_joy_mode(0);
      self.write_joy_control(0);
      self.write_reload_value(0);

      self.rx_fifo.clear();

      self.tx_ready_1 = true;
      self.tx_ready_2 = true;
    }

    if self.ctrl.acknowledge() {
      // reset bits 3,9 of joystat
      self.rx_parity_error = false;
      self.interrupt = false;
    }

    if self.ctrl.joy_select() {
      // do something with either joypad1 or 2
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    if self.currently_transferring {
      self.cycles -= cycles;

      if self.cycles <= 0 {
        self.transfer_byte();
      }
    } else if self.in_acknowledge {
      self.cycles -= cycles;

      if self.cycles <= 0 {
        self.in_acknowledge = false;
        self.ack_input = false;

        let mut interrupts = self.interrupts.get();

        interrupts.status.set_interrupt(Interrupt::Controller);
      }
    }
  }

  pub fn write_reload_value(&mut self, value: u16) {
    self.baudrate_timer = value as i32;
  }

  pub fn write_joy_mode(&mut self, value: u16) {
    self.mode.write(value);
  }

  pub fn read_byte(&mut self) -> u8 {
    if !self.rx_fifo.is_empty() {
      return self.rx_fifo.pop_front().unwrap();
    }

    0
  }

  pub fn read_control(&self) -> u16 {
    self.ctrl.read()
  }

  pub fn queue_byte(&mut self, value: u8) {
    self.tx_fifo.push_back(value);

    self.tx_ready_1 = true;
    self.tx_ready_2 = false;

    self.currently_transferring = true;

    self.cycles = (self.baudrate_timer as i32 & !1) * 8;
  }

  pub fn transfer_byte(&mut self) {
    if !self.ctrl.tx_enable() || self.tx_fifo.is_empty() {
      return;
    }

    self.currently_transferring = false;

    // controller 2 is currently unsupported, return back whatever
    if self.ctrl.desired_slot() == 1 {
      self.rx_fifo.push_back(0xff);
      return;
    }

    let command = self.tx_fifo.pop_front().unwrap();

    if self.active_device == ControllerDevice::None {
      if command == 0x1 {
        self.active_device = ControllerDevice::Controller
      } else if command == 0x81 {
        self.active_device = ControllerDevice::MemoryCard
      }
    }

    let mut enable = false;

    let response = match self.active_device {
      ControllerDevice::Controller => {
        let response = self.joypad.reply(command);
        if self.joypad.ack() {
          self.cycles += 338;
          self.in_acknowledge = true;
          self.ack_input = true;
          enable = true;
        } else {
          self.ack_input = false;
        }

        response
      }
      ControllerDevice::MemoryCard => {
        0xff
      }
      _ => unreachable!("can't happen")
    };

    self.rx_fifo.push_back(response);

    if !enable {
      self.active_device = ControllerDevice::None;
    }

    self.tx_ready_2 = true;
  }

  pub fn read_stat(&self) -> u32 {
    let mut value = self.tx_ready_1 as u32;

    value |= (!self.rx_fifo.is_empty() as u32) << 1;
    value |= (self.tx_ready_2 as u32) << 2;
    value |= (self.rx_parity_error as u32) << 3;
    value |= (self.interrupt as u32) << 9;
    value |= (self.baudrate_timer as u32) << 11;

    value
  }
}