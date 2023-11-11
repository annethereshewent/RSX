use super::{GPU, gpu_stat_register::ColorDepth};

impl GPU {
  pub fn update_picture(&mut self) {
    let mut x_start = self.display_vram_x_start;
    let mut y_start = self.display_vram_y_start;

    x_start += (self.display_horizontal_start - 608) * (self.get_dotclock() as u16);
    y_start += (self.display_line_start - 16) * 2;

    let (w, h) = self.get_dimensions();

    let mut i = 0;

    for x in x_start.. x_start + w {
      for y in y_start..y_start + h {
        match self.stat.display_color_depth {
          ColorDepth::FifteenBit => {
            let vram_address = self.get_vram_address(x as u32, y as u32);

            let color = ((self.vram[vram_address] as u16) | (self.vram[vram_address + 1] as u16) << 8);

            let (r, g, b) = self.translate_15bit_to_24(color);

            self.picture[i] = r;
            self.picture[i + 1] = g;
            self.picture[i + 2] = b;

          }
          ColorDepth::TwentyFourBit => {
            let vram_address = self.get_vram_address_24(x as u32, y as u32);

            self.picture[i] = self.vram[vram_address];
            self.picture[i + 1] = self.vram[vram_address + 1];
            self.picture[i + 2] = self.vram[vram_address + 2];
          }
        }
        i += 3;
      }
    }
  }

  fn translate_15bit_to_24(&self, val: u16) -> (u8, u8, u8) {
    let mut r = (val & 0x1f) as u8;
    let mut g = ((val >> 5) & 0x1f) as u8;
    let mut b = ((val >> 10) & 0x1f) as u8;

    r = (r << 3) | (r >> 2);
    g = (g << 3) | (g >> 2);
    b = (b << 3) | (b >> 2);

    (r, g, b)
  }

  pub fn get_dimensions(&self) -> (u16, u16) {
    let mut w = if self.display_horizontal_start <= self.display_horizontal_end {
      self.display_horizontal_end - self.display_horizontal_start
    } else {
      50
    };

    w = ((w / (self.get_dotclock()) as u16) + 2) & !0b11;
    let mut h = self.display_line_end - self.display_line_start;

    if self.stat.vertical_interlace {
      h *= 2;
    }

    (w, h)
  }
}