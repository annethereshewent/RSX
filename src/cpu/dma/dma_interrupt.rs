#[derive(Copy, Clone)]
pub struct DmaInterrupt {
  pub val: u32
}

impl DmaInterrupt {
  pub fn new() -> Self {
    Self {
      val: 0
    }
  }

  pub fn write(&mut self, val: u32) {
    self.val &= 0xff00_0000;
    // this acknowledges any interrupts by clearing the bits (bits 24-30)
    self.val &= !(val & 0x7f00_0000);
    // per specs (and other emulators)
    self.val |= val & 0xff_803f;

    self.update_master_flag();
  }

  pub fn write_upper(&mut self, val: u32) {
    self.val &= 0xff00_0000;
    // this acknowledges any interrupts by clearing the bits (bits 24-30)
    self.val &= !((val << 16) & 0x7f00_0000);
    self.val |= (val << 16) & 0xff_0000;

    self.update_master_flag();
  }

  pub fn force_irq(&self) -> bool {
    (self.val >> 15) & 0b1 == 1
  }

  pub fn is_dma_channel_irq_enabled(&self, channel_number: u32) -> bool {
    let offset = 16 + channel_number;
    (self.val >> offset) & 0b1 == 1
  }

  pub fn irq_master_enable(&self) -> bool {
    (self.val >> 23) & 0b1 == 1
  }

  pub fn set_irq_flag(&mut self, channel_number: u32) {
    let offset = 24 + channel_number;
    self.val |= 1 << offset;
  }

  pub fn dma_channel_irq_flag(&self, channel_number: u32) -> bool {
    let offset = 24 + channel_number;

    (self.val >> offset) & 0b1 == 1
  }

  pub fn is_irq_pending(&self) -> bool {
    let irq_flags = (self.val >> 24) & 0x7f;
    let irq_enable = (self.val >> 16) & 0x7f;

    irq_flags & irq_enable > 0
  }

  pub fn irq_master_flag(&self) -> bool {
    (self.val >> 31) & 0b1 == 1
  }

  pub fn update_master_flag(&mut self) -> bool {
    let previous_master = self.irq_master_flag();

    if (self.force_irq()) || (self.irq_master_enable() && self.is_irq_pending()) {
      self.val |= 1 << 31;

      if !previous_master {
        return true;
      }

    } else {
      self.val &= !(1 << 31);
    }

    false
  }
}