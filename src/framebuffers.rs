use std::vec;

use raylib::prelude::*;

pub struct FrameBuffer{
    pub color_buffer: Image,
    pub image_width: i32, 
    pub image_height: i32,
    pixel_size: i32, // Size of each pixel in the framebuffer
    _color: Color
}

impl FrameBuffer {

    pub fn new(image_width: i32, image_height: i32, color: Color, pixel_size: i32) -> Self {
        let color_buffer = Image::gen_image_color(image_width, image_height, color);
        Self {
            color_buffer,
            image_width,
            image_height,
            pixel_size,
            _color: color
        }
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && x < self.image_width && y >= 0 && y < self.image_height {
            self.color_buffer.draw_pixel(
                x as i32, 
                y as i32, 
                color
            )
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Color {
        if x >= 0 && x < self.image_width && y >= 0 && y < self.image_height {
            let image_data = self.color_buffer.get_image_data();
            let idx = (y * self.image_width + x) as usize;
            if idx < image_data.len() {
                image_data[idx]
            } else {
                Color::BLANK
            }
        }
        else {
            Color::BLANK
        }
    }

    pub fn draw_image(&self, output_file_name: &str) {
        self.color_buffer.export_image(output_file_name);
        println!("Image created and saved as '{}'!", output_file_name);
    }

    pub fn clear(&mut self) {
        self.color_buffer = Image::gen_image_color(self.image_width, self.image_height, self._color);
    }

    pub fn swap_buffers(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread
    ) {
        let fps = window.get_fps();
        if let Ok(texture) = window.load_texture_from_image(raylib_thread, &self.color_buffer) {
            let mut d = window.begin_drawing(raylib_thread);
            let screen_width = d.get_screen_width() as f32;
            let screen_height = d.get_screen_height() as f32;

            let texture_width = (self.image_width * self.pixel_size) as f32;
            let texture_height = (self.image_height * self.pixel_size) as f32;

            let scale_x = screen_width / texture_width;
            let scale_y = screen_height / texture_height;
            let scale = scale_x.min(scale_y); 

            let dest_width = texture_width * scale;
            let dest_height = texture_height * scale;

            let dest_x = (screen_width - dest_width) / 2.0;
            let dest_y = (screen_height - dest_height) / 2.0;

            

            d.draw_texture_ex(
                &texture,
                Vector2::new(dest_x, dest_y),
                0.0,
                scale,
                Color::WHITE,
            );

            d.draw_text(&format!("FPS: {}", fps), 1200, 10, 20, Color::WHITE);
        }
    }

    pub fn swap_buffers_image(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
        scarfy: &Texture2D
    ) {
        let fps = window.get_fps();
        if let Ok(texture) = window.load_texture_from_image(raylib_thread, &self.color_buffer) {
            let mut d = window.begin_drawing(raylib_thread);
            let screen_width = d.get_screen_width() as f32;
            let screen_height = d.get_screen_height() as f32;

            let texture_width = (self.image_width * self.pixel_size) as f32;
            let texture_height = (self.image_height * self.pixel_size) as f32;

            let scale_x = screen_width / texture_width;
            let scale_y = screen_height / texture_height;
            let scale = scale_x.min(scale_y); // mantener aspecto cuadrado

            let dest_width = texture_width * scale;
            let dest_height = texture_height * scale;

            let dest_x = (screen_width - dest_width) / 2.0;
            let dest_y = (screen_height - dest_height) / 2.0;

            

            d.draw_texture_ex(
                &texture,
                Vector2::new(dest_x, dest_y),
                0.0,
                scale,
                Color::WHITE,
            );

            d.draw_texture(
                scarfy,
                0,
                0,
                Color::WHITE
            );
        }
    }

}

