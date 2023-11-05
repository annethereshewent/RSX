use self::{dma_interrupt::DmaInterrupt, dma_channel::DmaChannel};

pub mod dma_interrupt;
pub mod dma_channel;
pub mod dma_channel_control_register;
pub mod dma_block_control_register;

pub struct DMA {
  pub control: u32,
  pub interrupt: DmaInterrupt,
  pub channels: [DmaChannel; 7]
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
      ]
    }
  }
}