use raylib::prelude::{Color, Vector3};

#[derive(Debug, Clone)]
pub struct Material {
    pub diffuse: Vector3,
    pub albedo: [f32; 4],
    pub specular: f32,
    pub refractive_index: f32,
    pub texture_id: Option<String>,
}

impl Material {
    pub fn new(diffuse: Vector3, specular: f32, albedo: [f32; 4], refractive_index: f32, texture_id: Option<String>) -> Self {
        Material {
            diffuse,
            albedo,
            specular,
            refractive_index,
            texture_id,
        }
    }

    pub fn black() -> Self {
        Material {
            diffuse: Vector3::zero(),
            albedo: [0.0, 0.0, 0.0, 0.0],
            specular: 0.0,
            refractive_index: 0.0,
            texture_id: None,
        }
    }
}

pub fn vector3_to_color(v: Vector3) -> Color {
    Color::new(
        (v.x * 255.0).min(255.0) as u8,
        (v.y * 255.0).min(255.0) as u8,
        (v.z * 255.0).min(255.0) as u8,
        255,
    )
}