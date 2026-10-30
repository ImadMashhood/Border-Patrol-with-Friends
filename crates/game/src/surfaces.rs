//! Small deterministic material maps: grit, weathering and fissures without asset downloads.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
pub fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut n = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed;
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n ^ (n >> 16)) as f32 / u32::MAX as f32
}
fn noise(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let u = x.fract();
    let v = y.fract();
    let u = u * u * (3. - 2. * u);
    let v = v * v * (3. - 2. * v);
    let h = |dx: i32, dy: i32| {
        hash(
            (ix + dx).rem_euclid(period),
            (iy + dy).rem_euclid(period),
            seed,
        )
    };
    (h(0, 0) * (1. - u) + h(1, 0) * u) * (1. - v) + (h(0, 1) * (1. - u) + h(1, 1) * u) * v
}
pub fn maps(images: &mut Assets<Image>, kind: usize) -> (Handle<Image>, Handle<Image>) {
    const N: usize = 256;
    let mut heights = vec![0.; N * N];
    let mut data = Vec::with_capacity(N * N * 4);
    for y in 0..N {
        for x in 0..N {
            let u = x as f32 / N as f32;
            let v = y as f32 / N as f32;
            let broad = noise(u * 8., v * 8., 8, 13 + kind as u32);
            let medium = noise(u * 32., v * 32., 32, 83);
            let grain = hash(x as i32, y as i32, 74 + kind as u32);
            let mut value = 0.72 + broad * 0.17 + medium * 0.07 + (grain - 0.5) * 0.16;
            // Warped branching fissures; a low-frequency layer also produces mineral/rust staining.
            let crack = (noise(u * 16., v * 16., 16, 192) + 0.13 * noise(u * 64., v * 64., 64, 9)
                - 0.54)
                .abs();
            if kind == 1 && crack < 0.018 {
                value *= 0.38;
            }
            if kind == 2 && crack < 0.008 {
                value *= 0.65;
            }
            if kind == 3 {
                value *= 0.8 + 0.2 * (v * 45. + broad * 2.).sin().abs();
            }
            heights[y * N + x] = value;
            let red = (value * 255.).clamp(0., 255.) as u8;
            data.extend_from_slice(&[red, red, red, 255]);
        }
    }
    let mut normals = Vec::with_capacity(N * N * 4);
    for y in 0..N {
        for x in 0..N {
            let sample = |dx: usize, dy: usize| heights[((y + dy) % N) * N + (x + dx) % N];
            let strength = if kind == 0 { 1.8 } else { 1.1 };
            let n = Vec3::new(
                (sample(N - 1, 0) - sample(1, 0)) * strength,
                (sample(0, N - 1) - sample(0, 1)) * strength,
                1.,
            )
            .normalize();
            normals.extend_from_slice(&[
                ((n.x * 0.5 + 0.5) * 255.) as u8,
                ((n.y * 0.5 + 0.5) * 255.) as u8,
                ((n.z * 0.5 + 0.5) * 255.) as u8,
                255,
            ]);
        }
    }
    let mut image = |bytes: &[u8], format| {
        let mut im = Image::new_fill(
            Extent3d {
                width: N as u32,
                height: N as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytes,
            format,
            RenderAssetUsages::default(),
        );
        im.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..ImageSamplerDescriptor::linear()
        });
        images.add(im)
    };
    (
        image(&data, TextureFormat::Rgba8UnormSrgb),
        image(&normals, TextureFormat::Rgba8Unorm),
    )
}

/// Two columns by four staggered courses, representing an 80 cm square of concrete masonry.
pub fn masonry(images: &mut Assets<Image>) -> (Handle<Image>, Handle<Image>) {
    const N: usize = 512;
    let mut color = Vec::with_capacity(N * N * 4);
    let mut heights = vec![0.; N * N];
    for y in 0..N {
        for x in 0..N {
            let row = y / 128;
            let shifted = x + if row % 2 == 0 { 0 } else { 128 };
            let bx = shifted % 256;
            let by = y % 128;
            let edge = bx.min(255 - bx).min(by.min(127 - by));
            let joint = edge < 4;
            let bevel = ((edge as f32 - 3.) / 6.).clamp(0., 1.);
            let variation = (hash((shifted / 256) as i32, row as i32, 51) - 0.5) * 9.;
            let pore = (hash(x as i32, y as i32, 88) - 0.5) * 8.;
            let value = if joint {
                133. + pore * 0.2
            } else {
                164. + variation + pore + bevel * 11.
            };
            color.extend_from_slice(&[value as u8, (value + 2.) as u8, (value - 2.) as u8, 255]);
            heights[y * N + x] = if joint {
                0.3
            } else {
                0.7 + bevel * 0.2 + pore * 0.001
            };
        }
    }
    let mut normals = Vec::with_capacity(N * N * 4);
    for y in 0..N {
        for x in 0..N {
            let sample = |dx: usize, dy: usize| heights[((y + dy) % N) * N + (x + dx) % N];
            let n = Vec3::new(
                (sample(N - 1, 0) - sample(1, 0)) * 1.4,
                (sample(0, N - 1) - sample(0, 1)) * 1.4,
                1.,
            )
            .normalize();
            normals.extend_from_slice(&[
                ((n.x * 0.5 + 0.5) * 255.) as u8,
                ((n.y * 0.5 + 0.5) * 255.) as u8,
                ((n.z * 0.5 + 0.5) * 255.) as u8,
                255,
            ]);
        }
    }
    let mut add = |bytes: &[u8], format| {
        let mut image = Image::new_fill(
            Extent3d {
                width: N as u32,
                height: N as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytes,
            format,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..ImageSamplerDescriptor::linear()
        });
        images.add(image)
    };
    (
        add(&color, TextureFormat::Rgba8UnormSrgb),
        add(&normals, TextureFormat::Rgba8Unorm),
    )
}
