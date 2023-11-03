const RAM_SIZE: usize = 2 * 1024 * 1024;

pub struct Bus {
  pub bios: Vec<u8>,
  ram: [u8; RAM_SIZE]
}

impl Bus {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      bios,
      ram: [0; RAM_SIZE]
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
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_write_8(&mut self, address: u32, value: u8) {
    let address = Bus::translate_address(address);

    match address {
      0x0000_0000..=0x001f_ffff => self.ram[address as usize] = value,
      0x1f80_1d80..=0x1f80_1dbc => println!("ignoring writes to SPU registers"),
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
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
      0x1f80_1d80..=0x1f80_1dbc => println!("ignoring writes to SPU registers"),
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
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
      0x1f80_1d80..=0x1f80_1dbc => println!("ignoring writes to SPU registers"),
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0x1f80_2041 => println!("ignoring writes to EXPANSION 2"),
      0xfffe_0130 => println!("ignoring write to CACHE_CONTROL register at address 0xfffe_0130"),
      _ => panic!("write to unsupported address: {:06x}", address)
    }
  }
}