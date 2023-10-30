pub struct Bus {
  bios: Vec<u8>
}

impl Bus {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      bios
    }
  }

  fn mem_read_8(&self, address: u32) -> u8 {
    match address {
      0xbfc0000..=0xC03ffff => self.bios[(address - 0xbfc0000) as usize],
      _ => 0
    }
  }

  pub fn mem_read_32(&self, address: u32) -> u32 {
    (self.mem_read_8(address) as u32) | ((self.mem_read_8(address + 1) as u32) << 8) | ((self.mem_read_8(address + 2) as u32) << 16) | ((self.mem_read_8(address + 3) as u32) << 24)
  }
}