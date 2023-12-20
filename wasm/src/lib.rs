extern crate rsx;
extern crate console_error_panic_hook;

use rsx::cpu::CPU;
use wasm_bindgen::prelude::*;
use std::panic;

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
  cpu: CPU
}

#[wasm_bindgen]
impl WasmEmulator {
  #[wasm_bindgen(constructor)]
  pub fn new(bios: &[u8], game_data: &[u8]) -> Self {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    Self {
      cpu: CPU::new(bios.to_vec(), game_data.to_vec(), true)
    }
  }
  pub fn run_frame(&mut self) {
    self.cpu.run_frame();
    self.cpu.bus.gpu.update_picture();
  }

  pub fn get_framebuffer(&self) -> *const u8 {
    self.cpu.bus.gpu.picture.as_ptr()
  }

  pub fn update_audio_buffers() {

  }

  pub fn framebuffer_size(&self) -> usize {
    self.cpu.bus.gpu.picture.len()
  }

  pub fn get_dimensions(&self) -> Vec<u32> {
    let (width, height) = self.cpu.bus.gpu.get_dimensions();

    vec![width, height]
  }
}