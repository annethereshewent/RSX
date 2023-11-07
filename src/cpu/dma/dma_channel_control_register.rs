#[derive(Clone, Copy)]
pub struct DmaChannelControlRegister {
  pub val: u32
}

pub enum SyncMode {
  Manual = 0,
  Request = 1,
  LinkedList = 2
}

impl DmaChannelControlRegister {
  pub fn new() -> Self {
    DmaChannelControlRegister { val: 0 }
  }

  pub fn is_from_ram(&self) -> bool {
    self.val & 0b1 == 1
  }

  pub fn is_address_increment(&self) -> bool {
    (self.val >> 1) & 0b1 == 0
  }

  pub fn chopping_enabled(&self) -> bool {
    (self.val >> 8) & 0b1 == 1
  }

  pub fn synchronization_mode(&self) -> SyncMode {
    match (self.val >> 9) & 0b11 {
      0 => SyncMode::Manual,
      1 => SyncMode::Request,
      2 => SyncMode::LinkedList,
      _ => panic!("invalid sync mode specified")
    }
  }

  pub fn chopping_dma_window(&self) -> u32 {
    (self.val >> 16) & 0b111
  }

  pub fn chopping_cpu_window(&self) -> u32 {
    (self.val >> 22) & 0b111
  }

  pub fn is_enabled(&self) -> bool {
    (self.val >> 24) & 0b1 == 1
  }

  pub fn manual_trigger(&self) -> bool {
    (self.val >> 28) & 0b1 == 1
  }

  pub fn set_enabled(&mut self, enabled: bool) {
    if enabled {
      self.val |= 1 << 24;
    } else {
      self.val &= !(1 << 24);
    }
  }

  pub fn set_trigger(&mut self, enabled: bool) {
    if enabled {
      self.val |= 1 << 28;
    } else {
      self.val &= !(1 << 28);
    }
  }


}