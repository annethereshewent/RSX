use super::{Coordinates2d, GPU, Vertex};

pub struct TextureDeltas {
  pub dudx: f64,
  pub dudy: f64,
  pub dvdx: f64,
  pub dvdy: f64
}

impl TextureDeltas {
  pub fn new(dudx: f64, dudy: f64, dvdx: f64, dvdy: f64) -> Self {
    Self {
      dudx,
      dudy,
      dvdx,
      dvdy
    }
  }

  pub fn get_texture_deltas(v: &[Vertex], cross_product: i32) -> Self {
    let dudx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].uv.x, v[0].p.y),
      Coordinates2d::new(v[1].uv.x, v[1].p.y),
      Coordinates2d::new(v[2].uv.x, v[2].p.y)
    );

    let dudy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].uv.x),
      Coordinates2d::new(v[1].p.x, v[1].uv.x),
      Coordinates2d::new(v[2].p.x, v[2].uv.x)
    );

    let dvdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].uv.y, v[0].p.y),
      Coordinates2d::new(v[1].uv.y, v[1].p.y),
      Coordinates2d::new(v[2].uv.y, v[2].p.y)
    );

    let dvdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].uv.y),
      Coordinates2d::new(v[1].p.x, v[1].uv.y),
      Coordinates2d::new(v[2].p.x, v[2].uv.y)
    );

    let dudx = dudx_cp as f64 / cross_product as f64;
    let dudy = dudy_cp as f64 / cross_product as f64;

    let dvdx = dvdx_cp as f64 / cross_product as f64;
    let dvdy = dvdy_cp as f64 / cross_product as f64;

    Self::new(dudx, dudy, dvdx, dvdy)
  }
}

pub struct ColorDeltas {
  pub drdx: f64,
  pub drdy: f64,
  pub dgdx: f64,
  pub dgdy: f64,
  pub dbdx: f64,
  pub dbdy: f64
}

impl ColorDeltas {
  pub fn new(drdx: f64, drdy: f64, dgdx: f64, dgdy: f64, dbdx: f64, dbdy: f64) -> Self {
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


    let drdx = drdx_cp as f64 / cross_product as f64;
    let drdy = drdy_cp as f64 / cross_product as f64;

    let dgdx = dgdx_cp as f64 / cross_product as f64;
    let dgdy = dgdy_cp as f64 / cross_product as f64;

    let dbdx = dbdx_cp as f64 / cross_product as f64;
    let dbdy = dbdy_cp as f64 / cross_product as f64;

    Self::new(drdx, drdy, dgdx, dgdy, dbdx, dbdy)
  }


}
