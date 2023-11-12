use std::cmp;

use super::{GPU, gpu_stat_register::ColorDepth};

impl GPU {
  fn get_2d_area(pos1: (i32, i32), pos2: (i32, i32), pos3: (i32, i32)) -> i32 {
    (pos2.0 - pos1.0) * (pos3.1 - pos1.1) - (pos2.1 - pos1.1) * (pos3.0 - pos1.0)
  }

  pub fn update_picture(&mut self) {
    let mut x_start = self.display_vram_x_start as u32;
    let mut y_start = self.display_vram_y_start as u32;

    x_start += ((self.display_horizontal_start as u32) - 608) / (self.get_dotclock() as u32);
    y_start += ((self.display_line_start as u32) - 16) * 2;

    let (w, h) = self.get_dimensions();

    let mut i = 0;

    for y in y_start..y_start + h {
      for x in x_start..x_start + w {
        match self.stat.display_color_depth {
          ColorDepth::FifteenBit => {
            let vram_address = self.get_vram_address(x as u32, y as u32);

            let color = (self.vram[vram_address] as u16) | (self.vram[vram_address + 1] as u16) << 8;

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

  pub fn get_dimensions(&self) -> (u32, u32) {
    let dotclock = self.get_dotclock() as u32;
    let mut w = if self.display_horizontal_start <= self.display_horizontal_end {
      self.display_horizontal_end - self.display_horizontal_start
    } else {
      50
    } as u32;

    w = ((w / dotclock) + 2) & !0b11;
    let mut h = (self.display_line_end - self.display_line_start) as u32;

    if self.stat.vertical_interlace {
      h *= 2;
    }

    (w, h)
  }

  pub fn render_pixel(&mut self, position: (i32, i32), color: (u8, u8, u8)) {
    let vram_address = self.get_vram_address(position.0 as u32, position.1 as u32);

    let value = GPU::color_to_u16(color);

    self.vram[vram_address] = value as u8;
    self.vram[vram_address + 1] = (value >> 8) as u8;
  }

  pub fn rasterize_triangle(&mut self, c: &mut [(u8, u8, u8)], p: &mut [(i32, i32)]) {
    let mut area = GPU::get_2d_area(p[0], p[1], p[2]);

    if area == 0 {
      return;
    }

    if area < 0 {
      p.swap(1, 2);
      c.swap(1, 2);

      area = -area;
    }

    let mut min_x = cmp::min(p[0].0, cmp::min(p[1].0, p[2].0));
    let mut min_y = cmp::min(p[0].1, cmp::min(p[1].1, p[2].1));

    let mut max_x = cmp::max(p[0].0, cmp::max(p[1].0, p[2].0));
    let mut max_y = cmp::max(p[0].1, cmp::max(p[1].1, p[2].1));

    let diff_x = max_x - min_x;
    let diff_y = max_y - min_y;

    if min_x >= 1024 ||
      max_x >= 1024 ||
      min_y >= 512 ||
      min_x >= 512 ||
      min_x < 0 ||
      min_y < 0 ||
      max_x < 0 ||
      min_y < 0 ||
      diff_x >= 1024 ||
      diff_y >= 512 {
        return;
    }

    min_x = cmp::max(min_x, self.drawing_area_left as i32);
    min_y = cmp::max(min_y, self.drawing_area_top as i32);

    max_x = cmp::min(max_x, self.drawing_area_right as i32);
    max_y = cmp::min(max_y, self.drawing_area_bottom as i32);

    let a01 = (p[0].1 - p[1].1) as i32;
    let b01 = (p[1].0 - p[0].0) as i32;

    let a12 = (p[1].1 - p[2].1) as i32;
    let b12 = (p[2].0 - p[1].0) as i32;

    let a20 = (p[2].1 - p[0].1) as i32;
    let b20 = (p[0].0 - p[2].0) as i32;

    let mut curr_p = (min_x, min_y);

    let mut w0_row = GPU::get_2d_area(p[1], p[2], curr_p) as i32;
    let mut w1_row = GPU::get_2d_area(p[2], p[0], curr_p) as i32;
    let mut w2_row = GPU::get_2d_area(p[0], p[1], curr_p) as i32;

    let w0_bias = -(GPU::is_top_left(b12, a12) as i32);
    let w1_bias = -(GPU::is_top_left(b20, a20) as i32);
    let w2_bias = -(GPU::is_top_left(b01, a01) as i32);

    while curr_p.1 < max_y {
      curr_p.0 = min_x;

      let mut w0 = w0_row;
      let mut w1 = w1_row;
      let mut w2 = w2_row;
      while curr_p.0 < max_x {
        if ((w0 + w0_bias) | (w1 + w1_bias) | (w2 + w2_bias)) >= 0 {
          let vec_3d = (w0, w1, w2);

          let color = GPU::interpolate_color(area as i32, vec_3d, c[0], c[1], c[2]);

          self.render_pixel(curr_p, color);
        }
        w0 += a12;
        w1 += a20;
        w2 += a01;

        curr_p.0 += 1;
      }
      w0_row += b12;
      w1_row += b20;
      w2_row += b01;

      curr_p.1 += 1;
    }
  }

  fn color_to_u16(color: (u8, u8, u8)) -> u16 {
    let mut pixel = 0;

    pixel |= ((color.0 as u16) & 0xf8) >> 3;
    pixel |= ((color.1 as u16) & 0xf8) << 2;
    pixel |= ((color.2 as u16) & 0xf8) << 7;

    pixel
  }

  fn is_top_left(x: i32, y: i32) -> bool {
    (y < 0) || ((x < 0) && (y == 0))
  }

  fn interpolate_color(area: i32, vec_3d: (i32,i32,i32), color0: (u8,u8,u8), color1: (u8,u8,u8), color2: (u8,u8,u8)) -> (u8, u8, u8) {
    let color0_r = color0.0 as i32;
    let color1_r = color1.0 as i32;
    let color2_r = color2.0 as i32;

    let color0_g = color0.1 as i32;
    let color1_g = color1.1 as i32;
    let color2_g = color2.1 as i32;

    let color0_b = color0.2 as i32;
    let color1_b = color1.2 as i32;
    let color2_b = color2.2 as i32;


    let r = ((vec_3d.0 * color0_r + vec_3d.1 * color1_r + vec_3d.2 * color2_r) / area) as u8;
    let g = ((vec_3d.0 * color0_g + vec_3d.1 * color1_g + vec_3d.2 * color2_g) / area) as u8;
    let b = ((vec_3d.0 * color0_b + vec_3d.1 * color1_b + vec_3d.2 * color2_b) / area) as u8;

    (r,g,b)
  }
}