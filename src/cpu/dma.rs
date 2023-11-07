use self::{dma_interrupt::DmaInterrupt, dma_channel::DmaChannel};

use super::scheduler::{Scheduler, Schedulable};

pub mod dma_interrupt;
pub mod dma_channel;
pub mod dma_channel_control_register;
pub mod dma_block_control_register;

pub const DMA_CYCLES: i32 = 128;

pub struct DMA {
  pub control: u32,
  pub interrupt: DmaInterrupt,
  pub channels: [DmaChannel; 7],
  period_counter: i32
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
      ],
      period_counter: 0
    }
  }

  pub fn step(&mut self, scheduler: &mut Scheduler) {
    let elapsed = scheduler.get_elapsed_cycles(Schedulable::Dma);

    self.period_counter += elapsed;
    self.period_counter %= DMA_CYCLES;

    let next_sync = DMA_CYCLES - self.period_counter;

    // TODO: actually add dma code here

    scheduler.schedule_next_event(next_sync, Schedulable::Dma);
  }
}