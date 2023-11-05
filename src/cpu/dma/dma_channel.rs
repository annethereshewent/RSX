use super::{dma_channel_control_register::{DmaChannelControlRegister, SyncMode}, dma_block_control_register::DmaBlockControlRegister};


#[derive(Clone, Copy)]
pub struct DmaChannel {
  pub base_address: u32,
  pub control: DmaChannelControlRegister,
  pub block_control: DmaBlockControlRegister
}

impl DmaChannel {
  pub fn new() -> Self {
    Self {
      base_address: 0,
      control: DmaChannelControlRegister::new(),
      block_control: DmaBlockControlRegister::new()
    }
  }

  pub fn is_active(&self) -> bool {
    let trigger = match self.control.synchronization_mode() {
      SyncMode::Manual => self.control.manual_trigger(),
      _ => true
    };

    self.control.is_enabled() && trigger
  }

  pub fn finish(&mut self) {
    self.control.set_enabled(false);
    self.control.set_trigger(false);
  }

  pub fn block_size(&self) -> u32 {
    match self.control.synchronization_mode() {
      SyncMode::Manual => self.block_control.block_size(),
      SyncMode::Request => self.block_control.block_size() * self.block_control.block_count(),
      SyncMode::LinkedList => 0
    }
  }
}