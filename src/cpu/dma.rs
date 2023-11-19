use self::{dma_interrupt::DmaInterrupt, dma_channel::DmaChannel, dma_channel_control_register::SyncMode};

use super::{bus::Bus, interrupt::interrupt_register::Interrupt};

pub mod dma_interrupt;
pub mod dma_channel;
pub mod dma_channel_control_register;
pub mod dma_block_control_register;

#[derive(Copy, Clone)]
pub struct DMA {
  pub control: u32,
  pub interrupt: DmaInterrupt,
  pub channels: [DmaChannel; 7],
  active_count: i32,
  cycles: i32
}

impl DMA {
  pub fn new() -> Self {
    Self {
      // default value taken from specs
      control: 0x07654321,
      interrupt: DmaInterrupt::new(),
      channels: [
        DmaChannel::new(0),
        DmaChannel::new(1),
        DmaChannel::new(2),
        DmaChannel::new(3),
        DmaChannel::new(4),
        DmaChannel::new(5),
        DmaChannel::new(6)
      ],
      active_count: 0,
      cycles: 0
    }
  }

  pub fn tick_counter(&mut self, cycles: i32) {
    self.cycles += cycles;
  }

  pub fn tick(&mut self, bus: &mut Bus) -> i32 {
    let mut count = 0;

    for i in 0..self.channels.len() {
      let channel = self.channels[i];

      if channel.is_active() && self.is_master_enabled(channel.channel_id) {
        match channel.control.synchronization_mode() {
          SyncMode::LinkedList => self.tick_linked_list(channel.channel_id, bus),
          SyncMode::Manual => self.tick_manual(channel.channel_id, bus),
          SyncMode::Request => self.tick_request(channel.channel_id, bus)
        }
      } else {
        self.channels[i].finish();
      }

      count += self.active_count;
      self.active_count = 0;
    }

    count
  }

  fn tick_request(&mut self, channel_id: usize, bus: &mut Bus) {
    let channel = &mut self.channels[channel_id];

    let is_increment = channel.control.is_address_increment();

    let masked_address = channel.active_address & 0x1ffffc;


    if channel.control.is_from_ram() {
      let word = bus.mem_read_32(masked_address);

      match channel.channel_id {
        0 => bus.mdec.write_command(word),
        2 => bus.gpu.gp0(word),
        4 => bus.spu.dma_write(word),
        _ => panic!("unhandled transfer from ram to channel {}", channel.channel_id)
      }
    } else {
      todo!("tick request to RAM not implemented yet");
    }

    if is_increment {
      channel.active_address = channel.active_address.wrapping_add(4);
    } else {
      channel.active_address = channel.active_address.wrapping_sub(4);
    }

    channel.word_count -= 1;

    if channel.word_count == 0 {
      self.active_count += channel.block_size() as i32;

      channel.blocks_remaining -= 1;

      if channel.blocks_remaining > 0 {
        channel.word_count += channel.block_size();
        channel.gap_ticks += 1;

      } else {
        channel.finish();

        let channel_id = channel.channel_id;
        self.update_interrupt_flags(channel_id, bus);
      }
    }
  }

  fn update_interrupt_flags(&mut self, channel_id: usize, bus: &mut Bus) {
    let offset = channel_id as u32;
    if self.interrupt.is_dma_channel_irq_enabled(offset) {
      self.interrupt.set_irq_flag(offset);
    }

    if self.interrupt.update_master_flag() {
      let mut interrupts = bus.interrupts.get();

      interrupts.status.set_interrupt(Interrupt::Dma);

      bus.interrupts.set(interrupts);
    }
  }

  fn tick_manual(&mut self, channel_id: usize, bus: &mut Bus) {
    let channel = &mut self.channels[channel_id];

    let is_increment = channel.control.is_address_increment();

    let masked_address = channel.active_address & 0x1ffffc;

    if channel.control.is_from_ram() {
      let word = bus.mem_read_32(masked_address);

      if channel.channel_id == 2 {
        bus.gpu.gp0(word);
      } else {
        panic!("unhandled transfer from ram to channel {}", channel.channel_id);
      }
    } else {
      let value = match channel.channel_id {
        3 => {
          let data = bus.cdrom.read_dma();

          data
        }
        6 => {
          if channel.word_count == 1 {
            0xffffff
          } else {
            channel.active_address.wrapping_sub(4) & 0x1fffff
          }
        }
        _ => todo!("channel not supported yet: {channel_id}")
      };

      bus.mem_write_32(masked_address, value);
    }

    if is_increment {
      channel.active_address = channel.active_address.wrapping_add(4);
    } else {
      channel.active_address = channel.active_address.wrapping_sub(4);
    }

    channel.word_count -= 1;


    if channel.word_count == 0 {
      self.active_count += channel.block_size() as i32;

      channel.finish();

      let channel_id = channel.channel_id;
      self.update_interrupt_flags(channel_id, bus);
    }
  }

  fn tick_linked_list(&mut self, channel_id: usize, bus: &mut Bus) {
    let channel = &mut self.channels[channel_id];

    if !channel.control.is_from_ram() {
      panic!("linked list DMA from RAM not supported");
    }

    if channel.channel_id != 2 {
      panic!("Only GPU channel supported in linked list mode");
    }

    if channel.gap_ticks > 0 {
      channel.gap_ticks += 1;
      return;
    }

    let header = bus.mem_read_32(channel.active_address);

    let mut word_count = header >> 24;

    while word_count > 0 {
      channel.active_address = (channel.active_address + 4) & 0x1ffffc;

      let val = bus.mem_read_32(channel.active_address);

      bus.gpu.gp0(val);

      word_count -= 1;
    }

    self.active_count += word_count as i32;
    channel.active_address = header & 0x1ffffc;

    if (header & 0xffffff) == 0xffffff {
      channel.finish();

      let channel_id = channel.channel_id;
      self.update_interrupt_flags(channel_id, bus);
    } else {
      channel.gap_ticks += 1;
    }
  }

  fn is_master_enabled(&mut self, channel_id: usize) -> bool {
    (self.control & (1 << ((channel_id << 2) + 3))) != 0
  }

  pub fn tick_gap(&mut self) {
    for channel in &mut self.channels {
      if channel.gap_ticks > 0 {
        channel.gap_ticks -= self.cycles as i32;
      }
    }
    self.cycles = 0;
  }

  pub fn chopping_enabled(&self) -> bool {
    for channel in self.channels {
      if channel.is_active() && channel.control.chopping_enabled() {
        return true;
      }
    }

    return false;
  }

  pub fn in_gap(&self) -> bool {
    for channel in self.channels {
      if channel.gap_ticks > 0 {
        return true;
      }
    }

    false
  }

  pub fn is_active(&self) -> bool {
    for channel in self.channels {
      if channel.is_active() {
        return true;
      }
    }

    false
  }

  pub fn read(&self, address: u32) -> u32 {
    let offset = address - 0x1f80_1080;

    let major = (offset & 0x70) >> 4;
    let minor = offset & 0xf;

    match major {
      0..=6 => {
        let channel = self.channels[major as usize];

        match minor {
          0 => channel.base_address,
          4 => channel.block_control.val,
          8 => channel.control.val,
          _ => panic!("unhandled dma read at offset {:X}", offset)
        }
      },
      7 => {
        match minor {
          0 => self.control,
          4 => self.interrupt.val,
          6 => self.interrupt.val >> 16,
          _ => panic!("unhandled DMA read at offset {:X}", offset)
        }
      }
      _ => panic!("unhandled DMA read at offset {:X}", offset)
    }
  }

  pub fn write(&mut self, address: u32, value: u32) {
    let offset = address - 0x1f80_1080;

    let major = (offset & 0x70) >> 4;
    let minor = offset & 0xf;

    match major {
      0..=6 => {
        let mut channel = self.channels[major as usize];

        match minor {
          0 => channel.base_address = value & 0xff_fffc,
          4 => {
            channel.block_control.val = value;
          },
          8 => channel.control.val = value,
          _ => panic!("unhandled dma read at offset {:X}", offset)
        }

        if channel.is_active() {
          channel.active_address = channel.base_address & 0x1f_fffc;

          match channel.control.synchronization_mode() {
            SyncMode::LinkedList => {
              channel.word_count = 1;
            }
            SyncMode::Manual => {
              channel.word_count = channel.block_size();
            }
            SyncMode::Request => {
              channel.word_count = channel.block_size();
              channel.blocks_remaining = channel.block_control.block_count();
            }
          }

          self.active_count = 0;
        }

        self.channels[major as usize] = channel;
      },
      7 => {
        match minor {
          0 => self.control = value,
          4 => {
            self.interrupt.write(value)
          }
          _ => panic!("unhandled DMA write at offset {:X}", offset)
        }
      }
      _ => panic!("unhandled DMA write at offset {:X}", offset)
    }
  }
}