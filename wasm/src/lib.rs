extern crate rsx;
extern crate console_error_panic_hook;

use rsx::{cpu::CPU, spu::SPU};
use wasm_bindgen::prelude::*;
use std::{panic, collections::VecDeque};

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum HighInput {
  ButtonL2 = 0,
  ButtonR2 = 1,
  ButtonL1 = 2,
  ButtonR1 = 3,
  ButtonTriangle = 4,
  ButtonCircle = 5,
  ButtonCross = 6,
  ButtonSquare = 7
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum LowInput {
  ButtonSelect = 0,
  ButtonL3 = 1,
  ButtonR3 = 2,
  ButtonStart = 3,
  ButtonUp = 4,
  ButtonRight = 5,
  ButtonDown = 6,
  ButtonLeft = 7
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
  ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}


#[wasm_bindgen]
pub struct WasmEmulator {
  cpu: CPU,
  audio_samples: VecDeque<i16>
}

#[wasm_bindgen]
impl WasmEmulator {
  #[wasm_bindgen(constructor)]
  pub fn new(bios: &[u8], game_data: &[u8]) -> Self {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    Self {
      cpu: CPU::new(bios.to_vec(), game_data.to_vec(), true),
      audio_samples: VecDeque::new()
    }
  }
  pub fn run_frame(&mut self) {
    self.cpu.run_frame();
    self.cpu.bus.gpu.update_picture();
    self.push_samples();
  }

  pub fn get_framebuffer(&self) -> *const u8 {
    self.cpu.bus.gpu.picture.as_ptr()
  }

  pub fn get_memory_card(&self) -> *const u8 {
    self.cpu.bus.controllers.memory_card.card_pointer()
  }

  pub fn load_card(&mut self, memory_card: &[u8]) {
    self.cpu.bus.controllers.memory_card.load_card(memory_card)
  }

  pub fn update_audio_buffers(&mut self, left: &mut [f32], right: &mut [f32]) {
    let len = self.audio_samples.len();

    let (last_left, last_right) = if len > 1 {
      (SPU::to_f32(self.audio_samples[len - 2]), SPU::to_f32(self.audio_samples[len - 1]))
    } else {
      (0.0, 0.0)
    };

    let mut left_index = 0;
    let mut right_index = 0;
    for i in 0..left.len() * 2 {
      if let Some(sample) = self.audio_samples.pop_front() {
        let sample = SPU::to_f32(sample);
        if i % 2 == 0 {
          left[left_index] = sample;
          left_index += 1;
        } else {
          right[right_index] = sample;
          right_index += 1;
        }
      } else {
        if i % 2 == 0 {
          left[left_index] = last_left;
          left_index += 1;
        } else {
          right[right_index] = last_right;
          right_index += 1;
        }
      }
    }
  }

  fn push_samples(&mut self) {
    let samples: Vec<i16> = self.cpu.bus.spu.audio_buffer.drain(..).collect();

    for sample in samples.iter() {
      self.audio_samples.push_back(*sample);
    }

    while self.audio_samples.len() > 32768 {
      self.audio_samples.pop_front().unwrap();
    }
  }

  pub fn toggle_digital_mode(&mut self) -> bool {
    let joypad = &mut self.cpu.bus.controllers.joypad;

    joypad.digital_mode = !joypad.digital_mode;

    joypad.digital_mode
  }

  pub fn has_saved(&mut self) -> bool {
    let memory_card = &mut self.cpu.bus.controllers.memory_card;

    let has_saved = memory_card.has_saved;

    memory_card.has_saved = false;


    has_saved
  }

  pub fn update_input(&mut self, button: u8, value: bool, is_high_input: bool) {
    let joypad = &mut self.cpu.bus.controllers.joypad;

    if !is_high_input {
      joypad.set_low_input(button, value);
    } else {
      joypad.set_high_input(button, value);
    }
  }

  pub fn framebuffer_size(&self) -> usize {
    self.cpu.bus.gpu.picture.len()
  }

  pub fn memory_card_size(&self) -> usize {
    self.cpu.bus.controllers.memory_card.card_size()
  }

  pub fn get_dimensions(&self) -> Vec<u32> {
    let (width, height) = self.cpu.bus.gpu.get_dimensions();

    vec![width, height]
  }
}