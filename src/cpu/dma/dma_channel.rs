use super::{dma_channel_control_register::{DmaChannelControlRegister, SyncMode}, dma_block_control_register::DmaBlockControlRegister};

#[derive(Clone, Copy)]
pub struct DmaChannel {
  pub base_address: u32,
  pub control: DmaChannelControlRegister,
  pub block_control: DmaBlockControlRegister,
  pub channel_id: usize,
  pub word_count: u32,
  pub blocks_remaining: u32,
  pub active_address: u32,
  pub gap_started: bool,
  pub gap_ticks: i32
}

impl DmaChannel {
  pub fn new(channel_id: usize) -> Self {
    Self {
      base_address: 0,
      control: DmaChannelControlRegister::new(),
      block_control: DmaBlockControlRegister::new(),
      channel_id,
      word_count: 0,
      blocks_remaining: 0,
      active_address: 0,
      gap_started: false,
      gap_ticks: 0
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
    // revisit this later
    // match self.control.synchronization_mode() {
    //   SyncMode::Manual => {
    //     self.block_control.block_size()
    //   },
    //   SyncMode::Request => {
    //     self.block_control.block_size() * self.block_control.block_count()
    //   },
    //   SyncMode::LinkedList => 0
    // }

    if self.block_control.block_size() > 0 { self.block_control.block_size() } else { 0x10000 }
  }
}