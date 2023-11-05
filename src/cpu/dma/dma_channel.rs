use super::{dma_channel_control_register::DmaChannelControlRegister, dma_block_control_register::DmaBlockControlRegister};


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
}