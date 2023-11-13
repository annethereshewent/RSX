use super::adsr::Adsr;

#[derive(Copy, Clone)]
pub struct Voice {
  volume_left: i16,
  volume_right: i16,
  pitch: u16,
  start_address: u16,
  repeat_address: u16,
  adsr: Adsr
}

impl Voice {
  pub fn new() -> Self {
    Self {
      volume_left: 0,
      volume_right: 0,
      pitch: 0,
      start_address: 0,
      repeat_address: 0,
      adsr: Adsr::new()
    }
  }

  pub fn read_16(&self, offset: u32) -> u16 {
    match offset {
      0 => (self.volume_left / 2) as u16,
      2 => (self.volume_right / 2) as u16,
      4 => self.pitch,
      6 => self.start_address,
      8 => self.adsr.value as u16,
      0xa => (self.adsr.value >> 16) as u16,
      0xc => self.adsr.current_volume as u16,
      0xe => self.repeat_address,
      _ => panic!("invalid SPU register specified")
    }
  }

  pub fn write_16(&mut self, offset: u32, value: u16) {
    match offset {
      0 => self.volume_left = (value & 0x7fff) as i16,
      2 => self.volume_right = (value & 0x7fff) as i16,
      4 => self.pitch = value,
      6 => self.start_address = value,
      8 => {
        self.adsr.value &= !(0xffff0000);
        self.adsr.value |= value as u32;
      }
      0xa =>{
        self.adsr.value &= !(0xffff);
        self.adsr.value |= (value as u32) << 16
      }
      0xc => self.adsr.current_volume = value as i16,
      0xe => self.repeat_address = value,
      _ => panic!("invalid SPU register specified")
    }
  }
}