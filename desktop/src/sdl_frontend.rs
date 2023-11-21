use std::collections::HashMap;

use rsx::{gpu::GPU, spu::SPU, cpu::CPU, controllers::joypad::{LowInput, HighInput}};
use sdl2::{video::Window, EventPump, event::Event, render::Canvas, pixels::PixelFormatEnum, audio::AudioCallback, Sdl, sys::KeyCode, keyboard::Keycode, controller::{GameController, Button}};

pub struct PsxAudioCallback<'a> {
  pub spu: &'a mut SPU
}

impl AudioCallback for PsxAudioCallback<'_> {
  type Channel = i16;

  fn callback(&mut self, buf: &mut [Self::Channel]) {
    let mut index = 0;
    let buffer_index = self.spu.buffer_index;

    let (last_left, last_right) = if buffer_index > 1 {
      (self.spu.audio_buffer[buffer_index - 2], self.spu.audio_buffer[buffer_index - 1])
    } else {
      (0, 0)
    };

    for b in buf.iter_mut() {
      *b = if index >= buffer_index {
        if index % 2 == 0 { last_left } else { last_right }
      } else {
        self.spu.audio_buffer[index]
      };

      self.spu.previous_value = *b;
      index += 1;
    }

    self.spu.buffer_index = 0;
  }
}

pub struct SdlFrontend {
  event_pump: EventPump,
  canvas: Canvas<Window>,
  _controller: Option<GameController>,
  button_map: HashMap<Button, (bool, u8)>
}

impl SdlFrontend {
  pub fn new(sdl_context: &Sdl) -> Self {

    let video = sdl_context.video().unwrap();

    let window = video.window("RSX", 640, 480)
      .position_centered()
      .build()
      .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let event_pump = sdl_context.event_pump().unwrap();

    let game_controller_subsystem = sdl_context.game_controller().unwrap();

    let available = game_controller_subsystem
        .num_joysticks()
        .map_err(|e| format!("can't enumerate joysticks: {}", e)).unwrap();

    let _controller = (0..available)
      .find_map(|id| {
        match game_controller_subsystem.open(id) {
          Ok(c) => {
            Some(c)
          }
          Err(_) => {
            None
          }
        }
      });

    let mut button_map = HashMap::new();

    button_map.insert(Button::A, (false, LowInput::ButtonCross as u8));
    button_map.insert(Button::B, (false, LowInput::ButtonCircle as u8));
    button_map.insert(Button::X, (false, LowInput::ButtonSquare as u8));
    button_map.insert(Button::Y, (false, LowInput::ButtonTriangle as u8));

    button_map.insert(Button::DPadUp, (true, HighInput::ButtonUp as u8));
    button_map.insert(Button::DPadDown, (true, HighInput::ButtonDown as u8));
    button_map.insert(Button::DPadLeft, (true, HighInput::ButtonLeft as u8));
    button_map.insert(Button::DPadRight, (true, HighInput::ButtonRight as u8));

    button_map.insert(Button::Back, (true, HighInput::ButtonSelect as u8));
    button_map.insert(Button::Start, (true, HighInput::ButtonStart as u8));

    button_map.insert(Button::LeftShoulder, (false, LowInput::ButtonL1 as u8));
    button_map.insert(Button::RightShoulder, (false, LowInput::ButtonR1 as u8));

    button_map.insert(Button::LeftStick, (true, HighInput::ButtonL3 as u8));
    button_map.insert(Button::RightStick, (true, HighInput::ButtonR3 as u8));


    Self {
      event_pump,
      canvas,
      _controller,
      button_map
    }
  }

  pub fn handle_events(&mut self, cpu: &mut CPU) {
    let joypad = &mut cpu.bus.controllers.joypad;

    for event in self.event_pump.poll_iter() {
      match event {
        Event::KeyDown { keycode: Some(k), .. } => {
          if k == Keycode::T {
            println!("toggling debug on");
            cpu.debug_on = !cpu.debug_on;
          }
        },
        Event::KeyUp { keycode: Some(_k), .. } => (),
        Event::Quit { .. } => std::process::exit(0),
        Event::ControllerButtonDown { button, ..} => {
          if button == Button::Touchpad {
            println!("setting digital mode to {}", !joypad.digital_mode);
            joypad.digital_mode = !joypad.digital_mode;
          } else if let Some(input) = self.button_map.get(&button) {
            let (is_high_input, input) = *input;
            if !is_high_input {
              joypad.set_low_input(input, true);
            } else {
              joypad.set_high_input(input, true);
            }
          }
        }
        Event::ControllerButtonUp { button, .. } => {
          if let Some(input) = self.button_map.get(&button) {
            let (is_high_input, input) = *input;
            if !is_high_input {
              joypad.set_low_input(input, false);
            } else {
              joypad.set_high_input(input, false);
            }
          }
        }
        Event::ControllerAxisMotion { axis, value, .. } => {
          println!("you pressed {:?} with a value of {value}", axis);
        }
        _ => {},
    };
    }
  }

  pub fn render(&mut self, gpu: &mut GPU) {
    let (width, height) = gpu.get_dimensions();

    gpu.update_picture();

    let creator = self.canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, width as u32, height as u32)
        .unwrap();

    texture.update(None, &gpu.picture, width as usize * 3).unwrap();

    self.canvas.copy(&texture, None, None).unwrap();

    self.canvas.present();
  }
}