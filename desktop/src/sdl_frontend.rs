use rustation::gpu::GPU;
use sdl2::{video::{Window}, Sdl, EventPump, event::Event, render::Canvas, pixels::PixelFormatEnum};

pub struct SdlFrontend {
  sdl_context: Sdl,
  event_pump: EventPump,
  canvas: Canvas<Window>
}


impl SdlFrontend {
  pub fn new() -> Self {
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video.window("RSX", 640, 480)
      .position_centered()
      .build()
      .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let event_pump = sdl_context.event_pump().unwrap();

    Self {
      sdl_context,
      event_pump,
      canvas
    }
  }

  pub fn handle_events(&mut self) {
    for event in self.event_pump.poll_iter() {
      match event {
        Event::KeyDown { keycode: Some(k), .. } => (),
        Event::KeyUp { keycode: Some(k), .. } => (),
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