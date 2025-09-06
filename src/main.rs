//TODO
// Enemies with gifs
// Enemies movement
// Enemy AI
// Enemy in minimap


#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::thread;
use std::time::{Duration, Instant};
use raylib::ffi::TextFormat;
use raylib::prelude::*;
use raylib::prelude::RaylibDraw;
use std::f32::consts::PI;
use std::fs::File;
use std::io::BufReader;

mod framebuffers;
mod maze;
mod player;
mod line;
mod caster;
mod texture;
mod sprites;

use line::line;
use maze::{Maze,load_maze};
use caster::{cast_ray, Intersect};
use player::{Player, process_events};
use framebuffers::FrameBuffer;
use texture::TextureManager;
use sprites::Sprite;

const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);

fn cell_to_color(cell: char) -> Color {
    match cell {
        '+' => {
            return Color::from_hex("836c92").unwrap();
        },
        '-' => {
            return Color::from_hex("836c92").unwrap();
        },
        '|' => {
            return Color::from_hex("836c92").unwrap();
        },
        'g' => {
            return Color::GREEN;
        },
        'p' => {
            return Color::YELLOW;
        },
        'c' => {
            return Color::WHITE;
        },
        _ => {
            return Color::BLACK;
        },
        
    }
}

fn draw_cell(
  framebuffer: &mut FrameBuffer,
  xo: usize,
  yo: usize,
  block_size: usize,
  cell: char,
) {
  if cell == ' ' {
    return;
  }
  let color = cell_to_color(cell);

  for x in xo..xo + block_size {
    for y in yo..yo + block_size {
      framebuffer.set_pixel(x as i32, y as i32, color);
    }
  }
}

pub fn render_maze(
  framebuffer: &mut FrameBuffer,
  maze: &Maze,
  block_size: usize,
  player: &Player,
) {
  for (row_index, row) in maze.iter().enumerate() {
    for (col_index, &cell) in row.iter().enumerate() {
      let xo = col_index * block_size;
      let yo = row_index * block_size;
      if cell == 'c' {
        draw_cell(framebuffer, xo, yo + 7, 5, cell);
      } else {
        draw_cell(framebuffer, xo, yo, block_size, cell);
      }
    }
  }

  

  draw_cell(
    framebuffer,
    (player.pos.x / 7.0) as usize,
    (player.pos.y / 6.8) as usize,
    7,
    'p'
  );
}

fn render_world(
  framebuffer: &mut FrameBuffer,
  texture_manager: &TextureManager,
  maze: &Maze,
  block_size: usize,
  player: &Player,
  z_buffer: &mut Vec<f32>,
) {
  let num_rays = framebuffer.image_width;

  // let hw = framebuffer.width as f32 / 2.0;   
  let hh = framebuffer.image_height as f32 / 2.0;  


  for i in 0..num_rays {
    let current_ray = i as f32 / num_rays as f32; 
    let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
    let intersect = cast_ray(framebuffer, &maze, &player, a, block_size, false);

    
    let distance_to_wall = intersect.distance;
    z_buffer[i as usize] = distance_to_wall;
    let distance_to_projection_plane = 100.0;
        let stake_height = (hh / distance_to_wall) * distance_to_projection_plane;

        let stake_top = (hh - (stake_height / 2.0)) as isize;
        let stake_bottom = (hh + (stake_height / 2.0)) as isize;

        let texture_ref = if let Some(texture) = texture_manager.get_texture(intersect.impact) {
            texture
        } else {
            continue;
        };
        let tw_u = texture_ref.width();
        let th_u = texture_ref.height();

        let ys = stake_top.max(0) as usize;
        let ye = stake_bottom.min(framebuffer.image_height as isize - 1) as usize;

        for y in ys..=ye {
            let v = (y as f32 - ys as f32) / ((ye - ys).max(1) as f32);
            let tex_y = (v * th_u as f32) as u32;

            let color = texture_manager.get_pixel_color(
                intersect.impact,
                intersect.impact_x as u32 % tw_u as u32,
                tex_y,
            );

            framebuffer.set_pixel(i, y as i32, color);
        }
    }
}

fn render_sprites(
    framebuffer: &mut FrameBuffer,
    maze: &Maze,
    player: &Player,
    texture_manager: &TextureManager,
    block_size: usize,
    z_buffer: &Vec<f32>,
){
    
    for row in 0..maze.len() {
        for col in 0..maze[row].len() {
            match maze[row][col] {
                'c' | 'e' => {
                    let world_x = (col * block_size) as f32 + block_size as f32 / 2.0;
                    let world_y = (row * block_size) as f32 + block_size as f32 / 2.0;
                    let sprite = Sprite {
                        pos: Vector2 { x: world_x, y: world_y },
                        kind: maze[row][col],
                    };
                    draw_sprite(framebuffer, &player, &sprite, &texture_manager, &z_buffer);
                }
                _ => {}
            }
        }
    }
}

fn draw_sprite(
    framebuffer: &mut FrameBuffer,
    player: &Player,
    sprite: &Sprite,
    texture_manager: &TextureManager,
    z_buffer: &Vec<f32>
) {
    
    let dx = sprite.pos.x - player.pos.x;
    let dy = sprite.pos.y - player.pos.y;
    let distance = (dx*dx + dy*dy).sqrt();

    
    let angle_to_sprite = dy.atan2(dx);
    let mut angle_diff = angle_to_sprite - player.a;
    while angle_diff > PI { angle_diff -= 2.0 * PI; }
    while angle_diff < -PI { angle_diff += 2.0 * PI; }

    
    if angle_diff.abs() > player.fov / 2.0 {
        return;
    }

    let scale = 5.0; 
    let sprite_size = ((framebuffer.image_height as f32 / distance) * scale) as usize;

    let middle_screen = framebuffer.image_width as f32 / 2.0;
    let screen_x = middle_screen + angle_diff * framebuffer.image_width as f32 / player.fov;

    let start_x = (screen_x as isize - (sprite_size as isize) / 2).max(0) as usize;
    let half_screen = framebuffer.image_height as usize / 2;
    let half_sprite = sprite_size / 2;
    let start_y = half_screen.saturating_sub(half_sprite);

    let end_x = (start_x + sprite_size).min(framebuffer.image_width as usize);
    let end_y = (start_y + sprite_size).min(framebuffer.image_height as usize);

    for x in start_x..end_x {
        
        if distance < z_buffer[x] {
            for y in start_y..end_y {
                let tx = ((x - start_x) * 128 / sprite_size) as u32;
                let ty = ((y - start_y) * 128 / sprite_size) as u32;

                let color = texture_manager.get_pixel_color(sprite.kind, tx, ty);

                if color != TRANSPARENT_COLOR {
                    framebuffer.set_pixel(x as i32, y as i32, color );
                }
            }
        }
    }
}

fn main() {
    let framebuffer_width = 1300;
    let framebuffer_height = 900;
    let window_width = 1300;
    let window_height = 900;
    let block_size = 100;
    let mut start = false;
    let mut win = false;
    let framebuffer_color = Color::BLACK;
    let mut maze = Vec::new();

    let (mut window, mut raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Pacman Maze")
        .log_level(TraceLogLevel::LOG_ALL)
        .build();

    let mut framebuffer = FrameBuffer::new(
        framebuffer_width,
        framebuffer_height,
        framebuffer_color,
        1 
    );

    let texture_manager = texture::TextureManager::new(
        &mut window,
        &mut raylib_thread
    );



    

    let mut player = Player {
        pos: Vector2::new(150.0, 150.0),
        a: PI / 3.0,
        fov: PI / 3.0,
    };

    let texture = texture_manager.get_texture('#');

    let startpic = window.load_texture(&mut raylib_thread, "assets/start.png").unwrap();
    let winpic = window.load_texture(&mut raylib_thread, "assets/win.png").unwrap();

    

    

    while !window.window_should_close() {
        framebuffer.clear();

        if window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            break;
        }

        if !start{
            

            if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
                maze = load_maze("maze.txt");
                start = true;
            }
            if window.is_key_pressed(KeyboardKey::KEY_SPACE) {
                maze = load_maze("maze2.txt");
                start = true;
            }
            if window.is_key_pressed(KeyboardKey::KEY_E) {
                maze = load_maze("maze1.txt");
                start = true;
            }

            framebuffer.swap_buffers_image(&mut window, &raylib_thread, &startpic);
            thread::sleep(Duration::from_millis(8));
            continue;
        }
        
        if win {
            if window.is_key_pressed(KeyboardKey::KEY_R) {
                win = false;
                start = false;
                player.pos = Vector2::new(150.0, 150.0);
            }
            framebuffer.swap_buffers_image(&mut window, &raylib_thread, &winpic);
            thread::sleep(Duration::from_millis(8));
            continue;
        }
        
        if maze.iter().all(|row| row.iter().all(|&cell| cell != 'c')) {
            win = true;
        }

        process_events(&mut player, &mut window, &mut maze, block_size);

        for x in 0..framebuffer.image_width {
            for y in ((framebuffer.image_height)/2)..framebuffer.image_height {
                framebuffer.set_pixel(x, y, Color::from_hex("5c9599").unwrap());
            }
        }

        let mut z_buffer = vec![f32::MAX; framebuffer.image_width as usize];
        render_world(&mut framebuffer, &texture_manager, &maze, block_size, &player, &mut z_buffer);
        render_maze(&mut framebuffer, &maze, 15, &player);
        render_sprites(&mut framebuffer, &maze, &player, &texture_manager, block_size, &z_buffer);

        
        

        framebuffer.swap_buffers(&mut window, &raylib_thread);
        thread::sleep(Duration::from_millis(8));
    }


}
