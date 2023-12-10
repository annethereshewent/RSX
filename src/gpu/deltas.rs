use super::{Coordinates2d, GPU, Vertex};

pub struct TextureDeltas {
  pub dudx: i64,
  pub dudy: i64,
  pub dvdx: i64,
  pub dvdy: i64
}

impl TextureDeltas {
  pub fn new(dudx: i64, dudy: i64, dvdx: i64, dvdy: i64) -> Self {
    Self {
      dudx,
      dudy,
      dvdx,
      dvdy
    }
  }
  // TODO: make this method and get_color_deltas have consistent parameters between the two
  pub fn get_texture_deltas(p: &mut [Coordinates2d], t: &[Coordinates2d], cross_product: i32) -> Self {
    let dudx_cp = GPU::cross_product(
      Coordinates2d::new(t[0].x, p[0].y),
      Coordinates2d::new(t[1].x, p[1].y),
      Coordinates2d::new(t[2].x, p[2].y)
    );

    let dudy_cp = GPU::cross_product(
      Coordinates2d::new(p[0].x, t[0].x),
      Coordinates2d::new(p[1].x, t[1].x),
      Coordinates2d::new(p[2].x, t[2].x)
    );

    let dvdx_cp = GPU::cross_product(
      Coordinates2d::new(t[0].y, p[0].y),
      Coordinates2d::new(t[1].y, p[1].y),
      Coordinates2d::new(t[2].y, p[2].y)
    );

    let dvdy_cp = GPU::cross_product(
      Coordinates2d::new(p[0].x, t[0].y),
      Coordinates2d::new(p[1].x, t[1].y),
      Coordinates2d::new(p[2].x, t[2].y)
    );

    let dudx = (dudx_cp << 12) as i64 / cross_product as i64;
    let dudy = (dudy_cp << 12) as i64 / cross_product as i64;

    let dvdx = (dvdx_cp << 12) as i64 / cross_product as i64;
    let dvdy = (dvdy_cp << 12) as i64 / cross_product as i64;

    Self::new(dudx, dudy, dvdx, dvdy)
  }
}

pub struct ColorDeltas {
  pub drdx: i64,
  pub drdy: i64,
  pub dgdx: i64,
  pub dgdy: i64,
  pub dbdx: i64,
  pub dbdy: i64
}

impl ColorDeltas {
  pub fn new(drdx: i64, drdy: i64, dgdx: i64, dgdy: i64, dbdx: i64, dbdy: i64) -> Self {
    Self {
      drdx,
      drdy,
      dbdx,
      dbdy,
      dgdx,
      dgdy
    }
  }

  pub fn get_color_deltas(v: &[Vertex], cross_product: i32) -> ColorDeltas {
    let drdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].c.r as i32, v[0].p.y),
      Coordinates2d::new(v[1].c.r as i32, v[1].p.y),
      Coordinates2d::new(v[2].c.r as i32, v[2].p.y)
    );

    let drdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].c.r as i32),
      Coordinates2d::new(v[1].p.x, v[1].c.r as i32),
      Coordinates2d::new(v[2].p.x, v[2].c.r as i32)
    );

    let dgdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].c.g as i32, v[0].p.y),
      Coordinates2d::new(v[1].c.g as i32, v[1].p.y),
      Coordinates2d::new(v[2].c.g as i32, v[2].p.y)
    );

    let dgdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].c.g as i32),
      Coordinates2d::new(v[1].p.x, v[1].c.g as i32),
      Coordinates2d::new(v[2].p.x, v[2].c.g as i32)
    );

    let dbdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].c.b as i32, v[0].p.y),
      Coordinates2d::new(v[1].c.b as i32, v[1].p.y),
      Coordinates2d::new(v[2].c.b as i32, v[2].p.y)
    );

    let dbdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].c.b as i32),
      Coordinates2d::new(v[1].p.x, v[1].c.b as i32),
      Coordinates2d::new(v[2].p.x, v[2].c.b as i32)
    );


    let drdx = (drdx_cp << 12) as i64 / cross_product as i64;
    let drdy = (drdy_cp << 12) as i64 / cross_product as i64;

    let dgdx = (dgdx_cp << 12) as i64 / cross_product as i64;
    let dgdy = (dgdy_cp << 12) as i64 / cross_product as i64;

    let dbdx = (dbdx_cp << 12) as i64 / cross_product as i64;
    let dbdy = (dbdy_cp << 12) as i64 / cross_product as i64;

    Self::new(drdx, drdy, dgdx, dgdy, dbdx, dbdy)
  }


}
