pub const DECAY_STEP: i32 = -8;
pub const RELEASE_STEP: i32 = -8;

pub enum AdsrMode {
  Linear,
  Exponential
}

#[derive(Copy, Clone)]
pub enum AdsrDirection {
  Increasing,
  Decreasing
}

#[derive(Copy, Clone)]
pub struct Adsr {
  pub value: u32,
  pub current_volume: i16,
  pub attack_direction: AdsrDirection,
  pub decay_direction: AdsrDirection,
  pub release_direction: AdsrDirection
}

impl Adsr {
  pub fn new() -> Self {
    Self {
      value: 0,
      current_volume: 0,
      attack_direction: AdsrDirection::Increasing,
      decay_direction: AdsrDirection::Increasing,
      release_direction: AdsrDirection::Increasing
    }
  }

  pub fn attack_mode(&self) -> AdsrMode {
    match self.value & 0b1 {
      0 => AdsrMode::Linear,
      1 => AdsrMode::Exponential,
      _ => unreachable!("can't happen")
    }
  }

  pub fn attack_shift(&self) -> u32 {
    (self.value >> 10) & 0b11111
  }

  pub fn attack_step(&self) -> u32 {
    (self.value >> 8) & 0b11
  }

  pub fn decay_shift(&self) -> u32 {
    (self.value >> 4) & 0xf
  }

  pub fn sustain_mode(&self) -> AdsrMode {
    match (self.value >> 31) & 0b1 {
      0 => AdsrMode::Linear,
      1 => AdsrMode::Exponential,
      _ => unreachable!("can't happen")
    }
  }

  pub fn sustain_direction(&self) -> AdsrDirection {
    match (self.value >> 30) & 0b1 {
      0 => AdsrDirection::Increasing,
      1 => AdsrDirection::Decreasing,
      _ => unreachable!("can't happen")
    }
  }

  pub fn sustain_shift(&self) -> u32 {
    (self.value >> 24) & 0x1f
  }

  pub fn sustain_step(&self) -> u32 {
    (self.value >> 22) & 0b11
  }

  pub fn release_mode(&self) -> AdsrMode {
    match (self.value >> 21) & 0b1 {
      0 => AdsrMode::Linear,
      1 => AdsrMode::Exponential,
      _ => unreachable!("can't happen")
    }
  }

  pub fn release_shift(&self) -> u32 {
    (self.value >> 16) & 0x1f
  }

}