use self::dma_interrupt::DmaInterrupt;

pub mod dma_interrupt;

pub struct DMA {
  pub control: u32,
  pub interrupt: DmaInterrupt
}

impl DMA {
  pub fn new() -> Self {
    Self {
      // default value taken from specs
      control: 0x07654321,
      interrupt: DmaInterrupt::new()
    }
  }
}