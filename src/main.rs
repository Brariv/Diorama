#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::thread;
use nalgebra_glm::Vec3;
use std::time::{Duration, Instant};
use raylib::ffi::TextFormat;
use raylib::prelude::*;
use raylib::prelude::RaylibDraw;
use std::f32::consts::PI;
use std::io::BufReader;
use rayon::prelude::*;

mod framebuffers;
mod ray_intersect;
mod texture;
mod cube;
mod lights;
mod material;
mod camera;

const ORIGIN_BIAS: f32 = 1e-4;

use framebuffers::FrameBuffer;
use texture::TextureManager;
use ray_intersect::{Intersect, RayIntersect};
use cube::Cube;
use camera::Camera;
use lights::Light;
use material::{Material, vector3_to_color};

const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);

fn procedural_sky(dir: Vector3) -> Vector3 {
    let d = dir.normalized();
    let t = (d.y + 1.0) * 0.5; // map y [-1,1] → [0,1]

    let grass = Vector3::new(0.7, 0.8, 0.7); // winter grass green (desaturated, pale)
    let white = Vector3::new(1.0, 1.0, 1.0); // horizon haze
    let blue = Vector3::new(0.3, 0.5, 1.0);  // sky blue

    if t < 0.54 {
        // Bottom → fade grass to white
        let k = t / 0.55;
        grass * (1.0 - k) + white * k
    } else if t < 0.55 {
        // Around horizon → mostly white
        white
    } else if t < 0.8 {
        // Fade white to blue
        let k = (t - 0.55) / (0.25);
        white * (1.0 - k) + blue * k
    } else {
        // Upper sky → solid blue
        blue
    }
}

fn offset_origin(intersect: &Intersect, direction: &Vector3) -> Vector3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vector3, normal: &Vector3) -> Vector3 {
    *incident - *normal * 2.0 * incident.dot(*normal)
}

fn refract(incident: &Vector3, normal: &Vector3, refractive_index: f32) -> Option<Vector3> {
    // Implementation of Snell's Law for refraction.
    // It calculates the direction of a ray as it passes from one medium to another.

    // `cosi` is the cosine of the angle between the incident ray and the normal.
    // We clamp it to the [-1, 1] range to avoid floating point errors.
    let mut cosi = incident.dot(*normal).max(-1.0).min(1.0);

    // `etai` is the refractive index of the medium the ray is currently in.
    // `etat` is the refractive index of the medium the ray is entering.
    // `n` is the normal vector, which may be flipped depending on the ray's direction.
    let mut etai = 1.0; // Assume we are in Air (or vacuum) initially
    let mut etat = refractive_index;
    let mut n = *normal;

    if cosi > 0.0 {
        // The ray is inside the medium (e.g., glass) and going out into the air.
        // We need to swap the refractive indices.
        std::mem::swap(&mut etai, &mut etat);
        // We also flip the normal so it points away from the medium.
        n = -n;
    } else {
        // The ray is outside the medium and going in.
        // We need a positive cosine for the calculation, so we negate it.
        cosi = -cosi;
    }

    // `eta` is the ratio of the refractive indices (n1 / n2).
    let eta = etai / etat;
    // `k` is a term derived from Snell's law that helps determine if total internal reflection occurs.
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);

    if k < 0.0 {
        // If k is negative, it means total internal reflection has occurred.
        // There is no refracted ray, so we return None.
        None
    } else {
        // If k is non-negative, we can calculate the direction of the refracted ray.
        Some(*incident * eta + n * (eta * cosi - k.sqrt()))
    }
}

fn cast_shadow(
    intersect: &Intersect,
    light: &Light,
    objects: &[Cube],
) -> f32 {
    let light_dir = (light.position - intersect.point).normalized();
    let light_distance = (light.position - intersect.point).length();

    let shadow_ray_origin = offset_origin(intersect, &light_dir);

    for object in objects {
        let shadow_intersect = object.ray_intersect(&shadow_ray_origin, &light_dir);
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            return 1.0; // Hit something, full shadow
        }
    }

    0.0 // No shadow
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[Cube],
    light: &Light,
    texture_manager: &TextureManager,
    depth: u32,
) -> Vector3 {
    if depth > 3 {
        return procedural_sky(*ray_direction);
        // return SKYBOX_COLOR;
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = object.ray_intersect(ray_origin, ray_direction);
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return procedural_sky(*ray_direction);
        // return SKYBOX_COLOR;
    }

    let light_dir = (light.position - intersect.point).normalized();
    let view_dir = (*ray_origin - intersect.point).normalized();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalized();

    let shadow_intensity = cast_shadow(&intersect, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    let diffuse_color = if let Some(texture_path) = &intersect.material.texture_id {
        let texture = texture_manager.get_texture(texture_path).unwrap();
        let width = texture.width() as u32;
        let height = texture.height() as u32;

        // Compute (u, v) based on which face was hit so each face gets the full texture.
        let (u, v) = match intersect.face {
            Some(face) => {
                let local = intersect.local_pos; // local_pos should be the hit point in cube-local space [-0.5, 0.5]
                match face {
                    0 => ((local.z + 0.5), (0.5 - local.y)), // +X face
                    1 => ((0.5 - local.z), (0.5 - local.y)), // -X face
                    2 => ((local.x + 0.5), (local.z + 0.5)), // +Y face
                    3 => ((local.x + 0.5), (0.5 - local.z)), // -Y face
                    4 => ((local.x + 0.5), (0.5 - local.y)), // +Z face (front)
                    5 => ((0.5 - local.x), (0.5 - local.y)), // -Z face (back)
                    _ => (intersect.u, intersect.v),
                }
            }
            None => (intersect.u, intersect.v),
        };
        let u = u.max(0.0).min(1.0);
        let v = v.max(0.0).min(1.0);

        let tx = (u * (width as f32 - 1.0)).round() as u32;
        let ty = (v * (height as f32 - 1.0)).round() as u32;

        texture_manager.get_pixel_color(texture_path, tx, ty)
    } else {
        intersect.material.diffuse
    };

    let diffuse_intensity = intersect.normal.dot(light_dir).max(0.0) * light_intensity;
    let diffuse = diffuse_color * diffuse_intensity;

    let specular_intensity = view_dir.dot(reflect_dir).max(0.0).powf(intersect.material.specular) * light_intensity;
    let light_color_v3 = Vector3::new(light.color.r as f32 / 255.0, light.color.g as f32 / 255.0, light.color.b as f32 / 255.0);
    let specular = light_color_v3 * specular_intensity;

    let albedo = intersect.material.albedo;
    let phong_color = diffuse * albedo[0] + specular * albedo[1];

    // Reflections
    let reflectivity = intersect.material.albedo[2];
    let reflect_color = if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, depth + 1)
    } else {
        Vector3::zero()
    };

    // Refractions
    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        // Calculate the refracted ray direction. This can fail (return None) in case of total internal reflection.
        if let Some(refract_dir) = refract(ray_direction, &intersect.normal, intersect.material.refractive_index) {
            // If refraction is possible, cast a new ray.
            let refract_origin = offset_origin(&intersect, &refract_dir);
            cast_ray(&refract_origin, &refract_dir, objects, light, texture_manager, depth + 1)
        } else {
            // Total internal reflection occurred. In this case, the light is perfectly reflected.
            // We cast a reflection ray instead of a refraction ray.
            let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, depth + 1)
        }
    } else {
        // If the material is not transparent, the refracted color is black.
        Vector3::zero()
    };

    // Combine the Phong color with the reflected and refracted colors using the material's albedo values.
    phong_color * (1.0 - reflectivity - transparency) + reflect_color * reflectivity + refract_color * transparency
}

pub fn render(framebuffer: &mut FrameBuffer, objects: &[Cube], camera: &Camera, light: &Light, texture_manager: &TextureManager) {
    let width = framebuffer.image_width as u32;
    let height = framebuffer.image_height as u32;
    let aspect_ratio = width as f32 / height as f32;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    let mut scratch: Vec<Color> = vec![Color::new(0,0,0,255); width as usize * height as usize];

    scratch
        .par_iter_mut()
        .enumerate()
        .for_each(|(idx, pixel)| {
            let x = (idx as u32) % width;
            let y = (idx as u32) / width;

            let screen_x = (2.0 * x as f32) / width as f32 - 1.0;
            let screen_y = -(2.0 * y as f32) / height as f32 + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            
            let rotated_direction = camera.basis_change(&ray_direction);

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            *pixel = pixel_color;
        });

        for y in 0..height {
            let row = &scratch[(y * width) as usize .. ((y + 1) * width) as usize];
            for x in 0..width {
                framebuffer.set_pixel(x as i32, y as i32, row[x as usize]);
            }
        }
        
    

    // for y in 0..framebuffer.image_height {
    //     for x in 0..framebuffer.image_width {
    //         let screen_x = (2.0 * x as f32) / width - 1.0;
    //         let screen_y = -(2.0 * y as f32) / height + 1.0;

    //         let screen_x = screen_x * aspect_ratio * perspective_scale;
    //         let screen_y = screen_y * perspective_scale;

    //         let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            
    //         let rotated_direction = camera.basis_change(&ray_direction);

    //         let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
    //         let pixel_color = vector3_to_color(pixel_color_v3);

    //         framebuffer.set_pixel(x, y, pixel_color);
    //     }
    // }
}

fn main() {
    let framebuffer_width = 1300;
    let framebuffer_height = 900;
    let window_width = 1300;
    let window_height = 900;
    let framebuffer_color = Color::BLACK;

    let (mut window, mut raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("MC Diorama")
        .log_level(TraceLogLevel::LOG_ALL)
        .build();

    let mut framebuffer = FrameBuffer::new(
        framebuffer_width,
        framebuffer_height,
        framebuffer_color,
        1 
    );

    let mut texture_manager = TextureManager::new();
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/wool_colored_black.png",
    );
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/wool_colored_orange.png",
    );
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/wool_colored_white.png",
    );
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/wool_colored_pink.png",
    );
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/hardened_clay_stained_orange.png",
    );
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/obsidian.png",
    );
    texture_manager.load_texture(
        &mut window,
        &raylib_thread,
        "assets/gold_block.png",
    );

    

    
    let black_wool = Material::new(
        Vector3::new(0.3, 0.2, 0.1),
        5.0,
        [0.95, 0.05, 0.0, 0.0],
        0.0,
        Some("assets/wool_colored_black.png".to_string()),
    );

    let orange_wool = Material::new(
        Vector3::new(0.8, 0.8, 0.7),
        5.0,
        [0.98, 0.02, 0.0, 0.0],
        0.0,
        Some("assets/wool_colored_orange.png".to_string()),
    );

    let white_wool = Material::new(
        Vector3::new(0.8, 0.8, 0.7),
        5.0,
        [0.98, 0.02, 0.0, 0.0],
        0.0,
        Some("assets/wool_colored_white.png".to_string()),
    );

    let pink_wool = Material::new(
        Vector3::new(0.9, 0.7, 0.8),
        5.0,
        [0.98, 0.02, 0.0, 0.0],
        0.0,
        Some("assets/wool_colored_pink.png".to_string()),
    );

    let orange_clay = Material::new(
        Vector3::new(0.7, 0.3, 0.2),
        12.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some("assets/hardened_clay_stained_orange.png".to_string()),
    );


    let obsidian = Material::new(
        Vector3::new(0.4, 0.4, 0.3),
        50.0,
        [0.6, 0.3, 0.1, 0.0],
        0.0,
        Some("assets/obsidian.png".to_string()),
    );

    let glass = Material::new(
        Vector3::new(1.0, 0.5, 0.1), // orange color
        125.0,
        [0.0, 0.5, 0.1, 0.8],
        1.5,
        None,
    );

    let gold = Material::new(
        Vector3::new(1.0, 0.84, 0.0),
        65.0,
        [0.2, 0.6, 0.2, 0.0],
        0.0,
        Some("assets/gold_block.png".to_string()),
    );

    let objects = [
        
        Cube { center: Vector3::new(0.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(1.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(2.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(3.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(4.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(6.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(7.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(8.0, -1.0, 1.0), size: 1.0, material: white_wool.clone() },

        Cube { center: Vector3::new(4.0, -1.0, 2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, -1.0, 2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(6.0, -1.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, -1.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, -1.0, 2.0), size: 1.0, material: orange_wool.clone() },

        Cube { center: Vector3::new(-1.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(0.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(1.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(2.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(3.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(4.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },

        Cube { center: Vector3::new(-1.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(0.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(1.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(2.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(3.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(4.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },

        Cube { center: Vector3::new(-1.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(0.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(1.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(2.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(3.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(4.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },

        Cube { center: Vector3::new(5.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(6.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(6.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(6.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(7.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(7.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(7.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(8.0, -1.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(8.0, -1.0, -2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(8.0, -1.0, 0.0), size: 1.0, material: white_wool.clone() },


        Cube { center: Vector3::new(0.0, 0.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(1.0, 0.0, 1.0), size: 1.0, material: black_wool.clone() },
        Cube { center: Vector3::new(2.0, 0.0, 1.0), size: 1.0, material: black_wool.clone() },
        Cube { center: Vector3::new(3.0, 0.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(0.0, 1.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(1.0, 1.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(2.0, 1.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(3.0, 1.0, 1.0), size: 1.0, material: orange_wool.clone() },

        
        Cube { center: Vector3::new(-1.0, 0.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(4.0, 0.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 1.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(4.0, 1.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 2.0, 0.0), size: 1.0, material: black_wool.clone() },
        Cube { center: Vector3::new(0.0, 2.0, 0.0), size: 1.0, material: black_wool.clone() },
        Cube { center: Vector3::new(1.0, 2.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(2.0, 2.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(3.0, 2.0, 0.0), size: 1.0, material: black_wool.clone() },
        Cube { center: Vector3::new(4.0, 2.0, 0.0), size: 1.0, material: black_wool.clone() },
        Cube { center: Vector3::new(-1.0, 3.0, 0.0), size: 1.0, material: orange_clay.clone() },
        Cube { center: Vector3::new(0.0, 3.0, 0.0), size: 1.0, material: orange_clay.clone() },
        Cube { center: Vector3::new(1.0, 3.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(2.0, 3.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(3.0, 3.0, 0.0), size: 1.0, material: orange_clay.clone() },
        Cube { center: Vector3::new(4.0, 3.0, 0.0), size: 1.0, material: orange_clay.clone() },
        Cube { center: Vector3::new(-1.0, 3.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 4.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(0.0, 4.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(1.0, 4.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(2.0, 4.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(3.0, 4.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(4.0, 4.0, 0.0), size: 1.0, material: orange_wool.clone() },

        Cube { center: Vector3::new(-1.0, 5.0, -1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(0.0, 5.0, -1.0), size: 1.0, material: pink_wool.clone() },
        Cube { center: Vector3::new(3.0, 5.0, -1.0), size: 1.0, material: pink_wool.clone() },
        Cube { center: Vector3::new(4.0, 5.0, -1.0), size: 1.0, material: white_wool.clone() },

        Cube { center: Vector3::new(-1.0, 6.0, -1.0), size: 1.0, material: obsidian.clone() },
        Cube { center: Vector3::new(0.0, 6.0, -1.0), size: 1.0, material: obsidian.clone() },
        Cube { center: Vector3::new(3.0, 6.0, -1.0), size: 1.0, material: obsidian.clone() },
        Cube { center: Vector3::new(4.0, 6.0, -1.0), size: 1.0, material: obsidian.clone() },

        Cube { center: Vector3::new(-1.0, 2.0, 1.5), size: 1.0, material: glass.clone() },
        Cube { center: Vector3::new(0.0, 2.0, 1.5), size: 1.0, material: glass.clone() },

        Cube { center: Vector3::new(0.5, 2.0, 1.5), size: 0.5, material: gold.clone() },
        Cube { center: Vector3::new(1.0, 2.0, 1.5), size: 0.5, material: gold.clone() },
        Cube { center: Vector3::new(1.5, 2.0, 1.5), size: 0.5, material: gold.clone() },
        Cube { center: Vector3::new(2.0, 2.0, 1.5), size: 0.5, material: gold.clone() },
        Cube { center: Vector3::new(2.5, 2.0, 1.5), size: 0.5, material: gold.clone() },

        Cube { center: Vector3::new(3.0, 2.0, 1.5), size: 1.0, material: glass.clone() },


        Cube { center: Vector3::new(-1.0, 0.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 1.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 2.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 3.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 4.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 0.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 1.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 2.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 3.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(-1.0, 4.0, -2.0), size: 1.0, material: orange_wool.clone() },

        Cube { center: Vector3::new(0.0, 4.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(0.0, 4.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(1.0, 4.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(1.0, 4.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(2.0, 4.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(2.0, 4.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(3.0, 4.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(3.0, 4.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(4.0, 4.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(4.0, 4.0, -2.0), size: 1.0, material: orange_wool.clone() },

        Cube { center: Vector3::new(5.0, 3.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(5.0, 3.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(6.0, 3.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(6.0, 3.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 3.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 3.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 3.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 3.0, -2.0), size: 1.0, material: orange_wool.clone() },


        Cube { center: Vector3::new(4.0, 2.0, 1.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(4.0, 2.0, 2.0), size: 1.0, material: white_wool.clone() },

        Cube { center: Vector3::new(5.0, 2.0, 0.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, 2.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(5.0, 2.0, 2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(6.0, 2.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(6.0, 2.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(6.0, 2.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 2.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 2.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 2.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 2.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 2.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 2.0, 2.0), size: 1.0, material: orange_wool.clone() },

        Cube { center: Vector3::new(4.0, 1.0, 2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, 1.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(6.0, 1.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 1.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 1.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(4.0, 0.0, 2.0), size: 1.0, material: white_wool.clone() },
        Cube { center: Vector3::new(5.0, 0.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(6.0, 0.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(7.0, 0.0, 2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 0.0, 2.0), size: 1.0, material: orange_wool.clone() },

        Cube { center: Vector3::new(8.0, 1.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 0.0, 1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 1.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 0.0, 0.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 2.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 1.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 0.0, -1.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 2.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 1.0, -2.0), size: 1.0, material: orange_wool.clone() },
        Cube { center: Vector3::new(8.0, 0.0, -2.0), size: 1.0, material: orange_wool.clone() },


    ];

    let mut camera = Camera::new(
        Vector3::new(0.0, 0.0, 15.0),
        Vector3::new(3.0, 2.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let rotation_speed = PI / 100.0;

    let light = Light::new(
        Vector3::new(15.0, 15.0, 15.0),
        Color::new(255, 255, 255, 255),
        1.5,
    );

    

    while !window.window_should_close() {
        framebuffer.clear();

        if window.is_key_pressed(KeyboardKey::KEY_ESCAPE)|| window.is_key_pressed(KeyboardKey::KEY_Q) {
            break;
        }
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            camera.orbit(rotation_speed, 0.0);
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            camera.orbit(-rotation_speed, 0.0);
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            camera.orbit(0.0, -rotation_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            camera.orbit(0.0, rotation_speed);
        }

        if window.is_key_down(KeyboardKey::KEY_O) {
            camera.zoom(0.1);
        }
        if window.is_key_down(KeyboardKey::KEY_P) {
            camera.zoom(-0.1);
        }


        render(&mut framebuffer, &objects, &camera, &light, &texture_manager);


        framebuffer.swap_buffers(&mut window, &raylib_thread);
        //thread::sleep(Duration::from_millis(8));
    }


}
