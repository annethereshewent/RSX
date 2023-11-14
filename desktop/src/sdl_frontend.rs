use rsx::{gpu::GPU, spu::SPU};
use sdl2::{video::Window, EventPump, event::Event, render::Canvas, pixels::PixelFormatEnum, audio::AudioCallback, Sdl};

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
  canvas: Canvas<Window>
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

    Self {
      event_pump,
      canvas
    }
  }

  pub fn handle_events(&mut self) {
    for event in self.event_pump.poll_iter() {
      match event {
        Event::KeyDown { keycode: Some(_k), .. } => (),
        Event::KeyUp { keycode: Some(_k), .. } => (),
        Event::Quit { .. } => std::process::exit(0),
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