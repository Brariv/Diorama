use raylib::prelude::Vector3;
use crate::material::Material;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Intersect {
    pub point: Vector3,
    pub normal: Vector3,
    pub distance: f32,
    pub is_intersecting: bool,
    pub material: Material,
    pub local_pos: Vector3,
    pub face: Option<u32>,
    pub u: f32,
    pub v: f32,
}

impl Intersect {
    pub fn new(point: Vector3, normal: Vector3, distance: f32, material: Material, u: f32, v: f32, local_pos: Vector3, face: Option<u32>) -> Self {
        Intersect {
            point,
            normal,
            distance,
            is_intersecting: true,
            material,
            local_pos,
            face,
            u,
            v,
        }
    }

    pub fn empty() -> Self {
        Intersect {
            point: Vector3::zero(),
            normal: Vector3::zero(),
            distance: 0.0,
            is_intersecting: false,
            material: Material::black(),
            local_pos: Vector3::zero(),
            face: None,
            u: 0.0,
            v: 0.0,
        }
    }
}

pub trait RayIntersect {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect;
}