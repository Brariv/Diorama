use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use raylib::prelude::Vector3;

pub struct Cube {
    pub center: Vector3,
    pub size: f32,
    pub material: Material,
}

impl Cube {
    fn get_uv(&self, point: &Vector3) -> (f32, f32) {
        let half_size = self.size / 2.0;
        let local_point = *point - self.center + Vector3::new(half_size, half_size, half_size);

        let u = (local_point.x / self.size).fract();
        let v = (local_point.y / self.size).fract();

        (u, v)
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect {
        let half_size = self.size / 2.0;
        let min_bound = self.center - Vector3::new(half_size, half_size, half_size);
        let max_bound = self.center + Vector3::new(half_size, half_size, half_size);

        let mut tmin = (min_bound.x - ray_origin.x) / ray_direction.x;
        let mut tmax = (max_bound.x - ray_origin.x) / ray_direction.x;

        if tmin > tmax {
            std::mem::swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (min_bound.y - ray_origin.y) / ray_direction.y;
        let mut tymax = (max_bound.y - ray_origin.y) / ray_direction.y;

        if tymin > tymax {
            std::mem::swap(&mut tymin, &mut tymax);
        }

        if (tmin > tymax) || (tymin > tmax) {
            return Intersect::empty();
        }

        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }

        let mut tzmin = (min_bound.z - ray_origin.z) / ray_direction.z;
        let mut tzmax = (max_bound.z - ray_origin.z) / ray_direction.z;

        if tzmin > tzmax {
            std::mem::swap(&mut tzmin, &mut tzmax);
        }

        if (tmin > tzmax) || (tzmin > tmax) {
            return Intersect::empty();
        }

        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }

        if tmin < 0.0 && tmax < 0.0 {
            return Intersect::empty();
        }

        let distance = if tmin < 0.0 { tmax } else { tmin };
        let intersection_point = *ray_origin + *ray_direction * distance;
        let (u, v) = self.get_uv(&intersection_point);

        let local_pos = intersection_point - (self.center);

        let face = if (intersection_point.x - min_bound.x).abs() < 1e-4 {
            Some(0)
        } else if (intersection_point.x - max_bound.x).abs() < 1e-4 {
            Some(1)
        } else if (intersection_point.y - min_bound.y).abs() < 1e-4 {
            Some(2)
        } else if (intersection_point.y - max_bound.y).abs() < 1e-4 {
            Some(3)
        } else if (intersection_point.z - min_bound.z).abs() < 1e-4 {
            Some(4)
        } else if (intersection_point.z - max_bound.z).abs() < 1e-4 {
            Some(5)
        } else {
            None
        };

        // Determine normal
        let mut normal = Vector3::zero();
        let epsilon = 1e-4;

        if (intersection_point.x - min_bound.x).abs() < epsilon {
            normal = Vector3::new(-1.0, 0.0, 0.0);
        } else if (intersection_point.x - max_bound.x).abs() < epsilon {
            normal = Vector3::new(1.0, 0.0, 0.0);
        } else if (intersection_point.y - min_bound.y).abs() < epsilon {
            normal = Vector3::new(0.0, -1.0, 0.0);
        } else if (intersection_point.y - max_bound.y).abs() < epsilon {
            normal = Vector3::new(0.0, 1.0, 0.0);
        } else if (intersection_point.z - min_bound.z).abs() < epsilon {
            normal = Vector3::new(0.0, 0.0, -1.0);
        } else if (intersection_point.z - max_bound.z).abs() < epsilon {
            normal = Vector3::new(0.0, 0.0, 1.0);
        }

        Intersect::new(intersection_point, normal, distance, self.material.clone(), u, v, local_pos, face)
    }
}