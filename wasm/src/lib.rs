extern crate rsx;

use rsx::cpu::CPU;
use wasm_bindgen::prelude::*;

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
  pub fn new() -> Self {
    Self {
      cpu: CPU::new(None, None)
    }
  }
  pub fn run_frame(&mut self) {
    self.cpu.run_frame();
  }

  pub fn get_framebuffer(&self) -> *const u8 {
    self.cpu.bus.gpu.picture.as_ptr()
  }

  pub fn update_audio_buffers() {

  }

  pub fn load_bios(&mut self, bios: &[u8]) {
    console_log!("received bios {:?}", bios);
  }
}