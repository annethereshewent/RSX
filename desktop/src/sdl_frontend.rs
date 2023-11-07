use sdl2::{video::{Window}, Sdl, EventPump, event::Event};

pub struct SdlFrontend {
  sdl_context: Sdl,
  window: Window,
  event_pump: EventPump
}


impl SdlFrontend {
  pub fn new() -> Self {
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video.window("Playstation Emulator", 640, 480)
      .position_centered()
      .build()
      .unwrap();

    let event_pump = sdl_context.event_pump().unwrap();

    Self {
      sdl_context,
      window,
      event_pump
    }
  }

  pub fn handle_events(&mut self) {
    for event in self.event_pump.poll_iter() {
      match event {
        Event::KeyDown { keycode: Some(k), .. } => println!("you pressed {:?}", k),
        Event::KeyUp { keycode: Some(k), .. } => println!("you pressed {:?}", k),
        Event::Quit { .. } => std::process::exit(0),
        _ => {},
    };
    }
  }
}