#[derive(Clone, Copy, PartialEq)]
pub enum TextureColors {
  FourBit = 0,
  EightBit = 1,
  FifteenBit = 2
}

#[derive(Clone, Copy)]
pub enum Field {
  Bottom = 0,
  Top = 1
}

#[derive(Clone, Copy)]
pub enum SemiTransparency {
  Half,
  Add,
  Subtract,
  AddQuarter,
}

#[derive(Clone, Copy)]
pub enum VideoMode {
  Ntsc = 0,
  Pal = 1
}

#[derive(Clone, Copy)]
pub enum ColorDepth {
  FifteenBit = 0,
  TwentyFourBit = 1
}

#[derive(Clone, Copy)]
pub enum  DmaDirection {
  Off = 0,
  Fifo = 1,
  CputoGP0 = 2,
  GpuReadToCpu = 3
}

pub struct GpuStatRegister {
  pub texture_x_base: u8,
  pub texture_y_base1: u8,
  pub texture_y_base2: u8,

  pub semi_transparency: SemiTransparency,
  pub texture_colors: TextureColors,
  pub dither_enabled: bool,
  pub draw_to_display: bool,
  pub force_mask_bit: bool,
  pub preserved_masked_pixels: bool,
  pub interlace_field: Field,
  pub reverse_flag: bool,
  pub hres1: u8,
  pub hres2: u8,
  pub horizontal_resolution: u16,
  pub vres: u8,
  pub vertical_resolution: u16,
  pub video_mode: VideoMode,
  pub display_color_depth: ColorDepth,
  pub vertical_interlace: bool,
  pub display_enable: bool,
  pub irq_enabled: bool,
  pub dma_dir: DmaDirection,
  pub ready_for_command: bool,
  pub ready_vram_to_cpu: bool,
  pub ready_rcv_dma_block: bool,
  pub even_odd: bool
}

impl GpuStatRegister {
  pub fn new() -> Self {
    Self {
      texture_x_base: 0,
      texture_y_base1: 0,
      texture_y_base2: 0,
      semi_transparency: SemiTransparency::Half,
      texture_colors: TextureColors::FourBit,
      dither_enabled: false,
      draw_to_display: false,
      force_mask_bit: false,
      preserved_masked_pixels: false,
      interlace_field: Field::Bottom,
      reverse_flag: false,
      hres1: 0,
      hres2: 0,
      vres: 0,
      video_mode: VideoMode::Ntsc,
      display_color_depth: ColorDepth::FifteenBit,
      vertical_interlace: false,
      display_enable: false,
      irq_enabled: false,
      dma_dir: DmaDirection::Off,
      ready_for_command: true,
      ready_rcv_dma_block: true,
      ready_vram_to_cpu: true,
      even_odd: false,
      vertical_resolution: 240,
      horizontal_resolution: 320
    }
  }

  pub fn update_draw_mode(&mut self, val: u32) {
    self.texture_x_base = (val & 0xf) as u8;
    self.texture_y_base1 = (val & 0x10) as u8;
    self.semi_transparency = match (val >> 5) & 0b11 {
      0 => SemiTransparency::Half,
      1 => SemiTransparency::Add,
      2 => SemiTransparency::Subtract,
      3 => SemiTransparency::AddQuarter,
      _ => unreachable!("can't happen")
    };

    self.texture_colors = match (val >> 7) & 0b11 {
      0 => TextureColors::FourBit,
      1 => TextureColors::EightBit,
      2 => TextureColors::FifteenBit,
      n => panic!("unhandled texture depth received: {n}")
    };

    self.video_mode = if ((val >> 3)) & 0b1 == 0 {
      VideoMode::Ntsc
    } else {
      VideoMode::Pal
    };

    self.dither_enabled = ((val >> 9) & 0b1) == 1;
    self.draw_to_display = ((val >> 10) & 0b1) == 1;
    self.texture_y_base2 = ((val >> 11) & 0b1) as u8;
  }

  pub fn update_display_mode(&mut self, val: u32) {
    self.hres1 = (val & 0b11) as u8;
    self.hres2 = ((val >> 6) & 0b1) as u8;

    self.vres = ((val >> 2) & 0b1) as u8;

    self.vertical_resolution = 240;
    self.horizontal_resolution = if self.hres2 == 1 {
      368
    } else {
      match self.hres1 {
        0 => 256,
        1 => 320,
        2 => 512,
        3 => 640,
        _ => unreachable!("can't happen")
      }
    };

    self.display_color_depth = if ((val >> 4) & 0b1) == 1 {
      ColorDepth::TwentyFourBit
    } else {
      ColorDepth::FifteenBit
    };

    self.vertical_interlace = (val >> 5) & 0b1 == 1;

    if self.vertical_interlace && self.vres == 1 {
      self.vertical_resolution = 480;
    }

    if (val >> 7) & 0b1 == 1 {
      panic!("unsupported display mode found: reverse flag setting");
    }
  }

  pub fn update_dma_dir(&mut self, val: u32) {
    self.dma_dir = match val & 0b11 {
      0 => DmaDirection::Off,
      1 => DmaDirection::Fifo,
      2 => DmaDirection::CputoGP0,
      3 => DmaDirection::GpuReadToCpu,
      _ => unreachable!("can't happen")
    }
  }

  pub fn set_mask_attributes(&mut self, val: u32) {
    self.force_mask_bit = val & 0b1 == 1;
    self.preserved_masked_pixels = (val >> 1) & 0b1 == 1;
  }

  pub fn reset(&mut self) {
    self.irq_enabled = false;

    self.texture_x_base = 0;
    self.texture_y_base1 = 0;
    self.semi_transparency = SemiTransparency::Half;
    self.texture_colors = TextureColors::FourBit;
    self.dither_enabled = false;
    self.draw_to_display = false;
    self.texture_y_base2 = 0;
    self.force_mask_bit = false;
    self.preserved_masked_pixels = false;
    self.dma_dir = DmaDirection::Off;
    self.display_enable = false;
    self.hres1 = 0;
    self.hres2 = 0;
    self.vres = 0;
    self.vertical_resolution = 240;
    self.horizontal_resolution = 320;
    self.video_mode = VideoMode::Ntsc;
    self.vertical_interlace = true;
    self.display_color_depth = ColorDepth::FifteenBit;
  }

  pub fn value(&self, interlace_line: bool) -> u32 {
    let mut result = 0u32;

    result |= self.texture_x_base as u32;
    result |= (self.texture_y_base1 as u32) << 4;
    result |= (self.semi_transparency as u32) << 5;
    result |= (self.texture_colors as u32) << 7;
    result |= (self.dither_enabled as u32) << 9;
    result |= (self.draw_to_display as u32 ) << 10;
    result |= (self.force_mask_bit as u32) << 11;
    result |= (self.preserved_masked_pixels as u32) << 12;
    result |= (self.interlace_field as u32) << 13;
    result |= (self.texture_y_base2 as u32) << 15;
    result |= (self.hres2 as u32) << 16;
    result |= (self.hres1 as u32) << 17;
    result |= (self.vres as u32) << 19;
    result |= (self.video_mode as u32) << 20;
    result |= (self.display_color_depth as u32) << 21;
    result |= (self.vertical_interlace as u32) << 22;
    result |= (self.display_enable as u32) << 23;
    result |= (self.irq_enabled as u32) << 24;
    result |= (0b111) << 26;
    result |= (self.dma_dir as u32) << 29;
    result |= (interlace_line as u32) << 31;

    let dma_request = match self.dma_dir {
      DmaDirection::Off => 0,
      DmaDirection::Fifo => 1,
      DmaDirection::CputoGP0 => (result >> 28) & 0b1,
      DmaDirection::GpuReadToCpu => (result >> 27) & 0b1
    };

    result |= dma_request << 25;

    result
  }
}