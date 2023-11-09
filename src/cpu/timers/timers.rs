use crate::cpu::counter::Counter;

use super::timer::Timer;

#[derive(Clone, Copy)]
pub struct Timers {
  t: [Timer; 3]
}

impl Timers {
  pub fn new() -> Self {
    Self {
      t: [Timer::new(); 3]
    }
  }

  pub fn read(&self, address: u32) -> u16 {
    let timer_id = ((address & 0x30) >> 4) as usize;
    let offset = address & 0xcc;

    let timer = self.t[timer_id];

    match offset {
      0 => timer.value,
      4 => timer.mode,
      8 => timer.target_value,
      _ => panic!("unsupported offset given to timer io: {offset}")
    }
  }

  pub fn tick(&mut self, cycles: i64) {

  }

  pub fn write(&mut self, address: u32, value: u16) {
    let timer_id = ((address & 0x30) >> 4) as usize;
    let offset = address & 0xc;

    let timer = &mut self.t[timer_id];

    match offset {
      0 => timer.value = value,
      4 => {
        // timer is reset to 0 on writes to mode
        timer.value = 0;


        // clear the bottom bits except bits 10-12
        timer.mode &= 0b111 << 10;
        // set bit 10 after writing to this register
        timer.mode |= 1 << 10;
        // finally set the lower 9 bits to the value given
        timer.mode |= value & 0x3ff;
      }
      8 => timer.target_value = value,
      _ => panic!("unsupported offset given to timer io: {offset}")
    }
  }
}