use std::cmp;

use crate::util;


#[derive(Copy, Clone, PartialEq)]
pub enum SweepMode {
  Linear,
  Exponential
}

#[derive(Copy, Clone, PartialEq)]
pub enum SweepDirection {
  Increasing,
  Decreasing
}


#[derive(Copy, Clone)]
pub struct VolumeSweep {
  pub volume: i16,
  pub is_active: bool,
  sweep_mode: SweepMode,
  sweep_direction: SweepDirection,
  sweep_phase_positive: bool,
  sweep_shift: u8,
  sweep_step: i16,
  cycles: u32,
}

impl VolumeSweep {
  pub fn new() -> Self {
    Self {
      volume: 0,
      is_active: false,
      sweep_mode: SweepMode::Linear,
      sweep_direction: SweepDirection::Increasing,
      sweep_phase_positive: false,
      sweep_shift: 0,
      sweep_step: 0,
      cycles: 0
    }
  }

  pub fn set_envelope_params(&mut self, value: u16) {
    println!("envelope is now active.");
    self.cycles = 0;

    self.sweep_mode = match (value >> 14) & 0b1 {
      0 => SweepMode::Linear,
      1 => SweepMode::Exponential,
      _ => unreachable!()
    };

    self.sweep_direction = match (value >> 13) & 0b1 {
      0 => SweepDirection::Increasing,
      1 => SweepDirection::Decreasing,
      _ => unreachable!()
    };

    self.sweep_phase_positive = (value >> 12) & 0b1 == 1;

    self.sweep_shift = ((value >> 2) & 0x1f) as u8;

    let temp = (value & 0x3) as i16;
    if self.sweep_direction == SweepDirection::Increasing {
      self.sweep_step = 7 - temp;
    } else {
      self.sweep_step = -8 + temp;
    }
  }

  pub fn tick(&mut self) {
    if self.is_active {
      if self.cycles > 0 {
        self.cycles -= 1;
      }

      let mut cycles = 1 << cmp::max(0, self.sweep_shift as i16 - 11);
      let mut step = (self.sweep_step as i32) << cmp::max(0, 11 - self.sweep_shift as i32);

      if self.sweep_mode == SweepMode::Exponential {
        if self.sweep_direction == SweepDirection::Increasing {
          if self.volume > 0x6000 {
            cycles *= 4;
          }
        } else {
          step = (step * self.volume as i32) >> 15;
        }
      }

      if self.cycles <= 0 {
        self.cycles += cycles;

        let new_volume = util::clamp(step + self.volume as i32, 0, 0x7fff) as i16;

        self.volume = new_volume;

        if self.sweep_direction == SweepDirection::Increasing && new_volume == 0x7fff || self.sweep_direction == SweepDirection::Decreasing && new_volume == 0 {
          self.is_active = false;
          self.cycles = 0;
        }
      }
    }
  }
}