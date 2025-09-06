use raylib::color::Color;

use crate::framebuffers::FrameBuffer;
use crate::maze::Maze;
use crate::player::Player;

pub struct Intersect {
  pub distance: f32,
  pub impact: char,
  pub impact_x: f32,
  pub impact_y: f32
}

pub fn cast_ray(
  frame_buffer: &mut FrameBuffer,
  maze: &Maze,
  player: &Player,
  a: f32,
  block_size: usize,
  draw_line: bool,
) -> Intersect {
  let mut d = 0.0;

  loop {
    let cos = d * a.cos();
    let sin = d * a.sin();
    let x = (player.pos.x + cos) as usize;
    let y = (player.pos.y + sin) as usize;

    let i = x / block_size;
    let j = y / block_size;

    if maze[j][i] != ' ' && maze[j][i] != 'c'  {
      return Intersect{
        distance: d,
        impact: maze[j][i],
        impact_x: x as f32,
        impact_y: y as f32
      };
    }

    if draw_line {
      frame_buffer.set_pixel(x as i32, y as i32, Color::WHITESMOKE);
    }

    d += 10.0;
  }
}




