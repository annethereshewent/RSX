const RAM_SIZE: usize = 2 * 1024 * 1024;

pub struct Bus {
  bios: Vec<u8>,
  ram: [u8; RAM_SIZE]
}

impl Bus {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      bios,
      ram: [0; RAM_SIZE]
    }
  }

  fn mem_read_8(&self, address: u32) -> u8 {
    match address {
      0xbfc0_0000..=0xbfc7_ffff => self.bios[(address - 0xbfc0_0000) as usize],
      0xa000_0000..=0xa01f_ffff => self.ram[(address - 0xa000_0000) as usize],
      _ => panic!("not implemented: {:08x}", address)
    }
  }

  pub fn mem_read_32(&self, address: u32) -> u32 {
    if (address & 0b11) != 0 {
      panic!("unaligned address received: {:032b}", address);
    }
    (self.mem_read_8(address) as u32) | ((self.mem_read_8(address + 1) as u32) << 8) | ((self.mem_read_8(address + 2) as u32) << 16) | ((self.mem_read_8(address + 3) as u32) << 24)
  }

  pub fn mem_write_8(&mut self, address: u32, value: u8) {
    match address {
      0xa000_0000..=0xa01f_ffff => self.ram[(address - 0xa000_0000) as usize] = value,
      _ => ()
    }
  }

  pub fn mem_write_32(&mut self, address: u32, value: u32) {
    if (address & 0b11) != 0 {
      panic!("unaligned address received: {:X}", address);
    }

    match address {
      0x1f80_1000..=0x1f80_1023 => println!("ignoring store to MEMCTRL address {:08x}", address),
      0x1f80_1060 => println!("ignoring write to RAM_SIZE register at address 0x1f80_1060"),
      0xfffe_0130 => println!("ignoring write to CACHE_CONTROL register at address 0xfffe_0130"),
      _ => {
        self.mem_write_8(address, (value & 0xff) as u8);
        self.mem_write_8(address + 1, ((value >> 8) & 0xff) as u8);
        self.mem_write_8(address + 2, ((value >> 16) & 0xff) as u8);
        self.mem_write_8(address + 3, (value >> 24) as u8);
      }
    }


  }
}