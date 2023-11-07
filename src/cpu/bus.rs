use crate::gpu::GPU;

use super::dma::{DMA, dma_channel_control_register::SyncMode, dma_channel::DmaChannel};

const RAM_SIZE: usize = 2 * 1024 * 1024;

pub struct Bus {
  bios: Vec<u8>,
  ram: [u8; RAM_SIZE],
  dma: DMA,
  pub gpu: GPU
}

impl Bus {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      bios,
      ram: [0; RAM_SIZE],
      dma: DMA::new(),
      gpu: GPU::new()
    }
  }

  fn translate_address(address: u32) -> u32 {
    match address >> 29 {
      0b000..=0b011 => address,
      0b100 => address & 0x7fff_ffff,
      0b101 => address & 0x1fff_ffff,
      0b110..=0b111 => address,
      _ => unreachable!("not possible")
    }
  }

  pub fn mem_read_8(&self, address: u32) -> u8 {
    let address = Bus::translate_address(address);

    match address {
      0x1f00_0000..=0x1f08_0000 => 0xff,
      0x1fc0_0000..=0x1fc7_ffff => self.bios[(address - 0x1fc0_0000) as usize],
      0x0000_0000..=0x001f_ffff => self.ram[address as usize],
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_read_32(&self, address: u32) -> u32 {
    if (address & 0b11) != 0 {
      panic!("unaligned address received: {:032b}", address);
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
      0x1f80_1070..=0x1f80_1077 => {
        println!("ignoring reads to interrupt control registers");
        0
      }
      0x1f80_1080..=0x1f80_10ff => {
        let offset = address - 0x1f80_1080;

        let major = (offset & 0x70) >> 4;
        let minor = offset & 0xf;

        match major {
          0..=6 => {
            let channel = self.dma.channels[major as usize];

            match minor {
              0 => channel.base_address,
              4 => channel.block_control.val,
              8 => channel.control.val,
              _ => panic!("unhandled dma read at offset {:X}", offset)
            }
          },
          7 => {
            match minor {
              0 => self.dma.control,
              4 => self.dma.interrupt.val,
              6 => self.dma.interrupt.val >> 16,
              _ => panic!("unhandled DMA read at offset {:X}", offset)
            }
          }
          _ => panic!("unhandled DMA read at offset {:X}", offset)
        }
      }
      0x1f80_1100..=0x1f80_1130 => {
        println!("ignoring reads to timer registers");
        0
      }
      0x1f80_1810..=0x1f80_1817 => {
        let offset = address - 0x1f80_1810;

        // if offset == 4 {
        //   println!("attempting read from GPUSTAT");
        //   return 0x1c000000;
        // }

        match offset {
          0 => {
            println!("returning 0 for GPUREAD register");
            0
          }
          4 => self.gpu.stat.value(),
          _ => todo!("GPU read register not implemented yet: {offset}")
        }
      }
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_read_16(&self, address: u32) -> u16 {
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
      0x1fc0_0000..=0x1fc7_ffff => {
        let offset = (address - 0x1fc0_0000) as usize;
        (self.bios[offset] as u16) | ((self.bios[offset + 1] as u16) << 8)
      }

      0x1f80_1c00..=0x1f80_1e80 => {
        // println!("ignoring reads to SPU registers");
        0
      }
      0x1f80_1070..=0x1f80_1077 => {
        println!("ignoring reads to interrupt control registers");
        0
      }
      0x1f80_1080..=0x1f80_10ff => {
        println!("ignoring reads to DMA");
        0
      }
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_write_8(&mut self, address: u32, value: u8) {
    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => self.ram[address as usize] = value,
      0x1f80_1c00..=0x1f80_1e80 => {
        // println!("ignoring writes to SPU registers");
      }
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_1070..=0x1f80_1077 => println!("ignoring writes to interrupt control registers"),
      0x1f80_1100..=0x1f80_1130 => println!("ignoring writes to timer registers"),
      0x1f80_2041 => println!("ignoring writes to EXPANSION 2"),
      0xfffe_0130 => println!("ignoring write to CACHE_CONTROL register at address 0xfffe_0130"),
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
      0x1f80_1c00..=0x1f80_1e80 => {
        // println!("ignoring writes to SPU registers");
      }
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_1070..=0x1f80_1077 => println!("ignoring writes to interrupt control registers"),
      0x1f80_1100..=0x1f80_1130 => println!("ignoring writes to timer registers"),
      0x1f80_2041 => println!("ignoring writes to EXPANSION 2"),
      0xfffe_0130 => println!("ignoring write to CACHE_CONTROL register at address 0xfffe_0130"),
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
      0x1f80_1c00..=0x1f80_1e80 => {
        // println!("ignoring writes to SPU registers");
      }
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_1070..=0x1f80_1077 => println!("ignoring writes to interrupt control registers"),
      0x1f80_1080..=0x1f80_10ff => {
        let offset = address - 0x1f80_1080;

        let major = (offset & 0x70) >> 4;
        let minor = offset & 0xf;

        match major {
          0..=6 => {
            let mut channel = self.dma.channels[major as usize];

            match minor {
              0 => channel.base_address = value & 0xff_fffc,
              4 => {
                channel.block_control.val = value;
              },
              8 => channel.control.val = value,
              _ => panic!("unhandled dma read at offset {:X}", offset)
            }

            if channel.is_active() {
              self.do_dma(&mut channel);
            }

            self.dma.channels[major as usize] = channel;
          },
          7 => {
            match minor {
              0 => self.dma.control = value,
              4 => self.dma.interrupt.write(value),
              _ => panic!("unhandled DMA read at offset {:X}", offset)
            }
          }
          _ => panic!("unhandled DMA read at offset {:X}", offset)
        }
      }
      0x1f80_1100..=0x1f80_1130 => println!("ignoring writes to timer registers"),
      0x1f80_1810..=0x1f80_1817 => {
        let offset = address - 0x1f80_1810;

        match offset {
          0 => self.gpu.gp0(value),
          4 => self.gpu.gp1(value),
          _ => panic!("GPU write register not implemented yet: {offset}")
        }
      }
      0x1f80_2041 => println!("ignoring writes to EXPANSION 2"),
      0xfffe_0130 => println!("ignoring write to CACHE_CONTROL register at address 0xfffe_0130"),
      _ => panic!("write to unsupported address: {:06x}", address)
    }
  }

  fn do_dma(&mut self, channel: &mut DmaChannel) {
    match channel.control.synchronization_mode() {
      SyncMode::LinkedList => self.do_dma_linked_list(channel),
      _ => self.do_dma_block(channel)
    }
  }

  fn do_dma_block(&mut self, channel: &mut DmaChannel) {
    let mut word_count = channel.block_size();

    let mut base_address = channel.base_address;

    let is_increment = channel.control.is_address_increment();

    while word_count > 0 {
      let masked_address = base_address & 0x1ffffc;

      if channel.control.is_from_ram() {
        let word = self.mem_read_32(masked_address);

        if channel.channel_id == 2 {
          self.gpu.gp0(word);
        } else {
          panic!("unhandled transfer from ram to channel {}", channel.channel_id);
        }
      } else {
        let value = match channel.channel_id {
          6 => {
            if word_count == 1 {
              0xffffff
            } else {
              base_address.wrapping_sub(4) & 0x1fffff
            }
          }
          _ => todo!("channel not supported yet")
        };

        self.mem_write_32(masked_address, value);
      }

      if is_increment {
        base_address = base_address.wrapping_add(4);
      } else {
        base_address = base_address.wrapping_sub(4);
      }

      word_count -= 1;
    }

    channel.finish();
  }

  fn do_dma_linked_list(&mut self, channel: &mut DmaChannel) {
    let mut base_address = channel.base_address & 0x1f_fffc;

    if !channel.control.is_from_ram() {
      todo!("linked list DMA from RAM not yet implemented");
    }

    if channel.channel_id != 2 {
      panic!("Only GPU channel supported in linked list mode");
    }

    loop {
      let header = self.mem_read_32(base_address);

      let mut word_count = header >> 24;

      while word_count > 0 {
        base_address = (base_address + 4) & 0x1ffffc;

        let val = self.mem_read_32(base_address);

        self.gpu.gp0(val);

        word_count -= 1;
      }

      base_address = header & 0x1ffffc;

      if (header & 0xffffff) == 0xffffff {
        break;
      }
    }

    channel.finish()
  }
}