//! Static powder volume.
//!
//! Builds the two 3D textures the raymarch shader reads:
//!   colour: RGBA8 = powder colour, sqrt(density / 5)
//!   light:  RG8   = sun transmittance (powder above each voxel), local occlusion
//! Plumes are chains of soft puffs rising from each cannon.

use bevy::math::Vec3;

use crate::geometry::Rng;

pub const NX: usize = 160;
pub const NY: usize = 128;
pub const NZ: usize = 160;

pub struct Volume {
    pub color: Vec<u8>,
    pub light: Vec<u8>,
}

fn at(x: usize, y: usize, z: usize) -> usize {
    (z * NY + y) * NX + x
}

/// Add one soft sphere of powder (cell units).
fn puff(dens: &mut [f32], dye: &mut [[f32; 3]], c: Vec3, r: f32, amount: f32, col: [f32; 3]) {
    let lo = (c - Vec3::splat(r)).max(Vec3::ZERO);
    let hi = (c + Vec3::splat(r)).min(Vec3::new(NX as f32 - 1.0, NY as f32 - 1.0, NZ as f32 - 1.0));
    for z in lo.z as usize..=hi.z as usize {
        for y in lo.y as usize..=hi.y as usize {
            for x in lo.x as usize..=hi.x as usize {
                let d2 = Vec3::new(x as f32, y as f32, z as f32).distance_squared(c) / (r * r);
                if d2 < 1.0 {
                    let w = (1.0 - d2) * (1.0 - d2) * amount;
                    let i = at(x, y, z);
                    dens[i] += w;
                    for k in 0..3 {
                        dye[i][k] += w * col[k];
                    }
                }
            }
        }
    }
}

/// Plumes rising from cannons at `muzzles` (cell coords) along `aims`, in `colors`.
pub fn build(muzzles: &[Vec3], aims: &[Vec3], colors: &[[f32; 3]], seed: u64) -> Volume {
    let n = NX * NY * NZ;
    let mut dens = vec![0.0f32; n];
    let mut dye = vec![[0.0f32; 3]; n];
    let mut g = Rng::new(seed);
    for ((&m, &aim), &col) in muzzles.iter().zip(aims).zip(colors) {
        let height = g.range(45.0, 75.0);
        let mut p = m;
        let mut dir = aim.normalize();
        let steps = 16;
        for s in 0..steps {
            let t = s as f32 / steps as f32;
            dir = (dir + g.normal3() * 0.12 + Vec3::new(0.0, 0.15, 0.0)).normalize();
            p += dir * height / steps as f32;
            let r = 5.0 + 11.0 * t.sqrt();
            // each puff is a cluster of sub-puffs, so the plume is lumpy, not a tube
            for _ in 0..5 {
                let off = g.normal3() * r * 0.35;
                puff(&mut dens, &mut dye, p + off, r * g.range(0.55, 0.85), 1.1 * (1.0 - 0.4 * t), col);
            }
        }
    }

    // sun almost overhead: transmittance = exp(-k * powder above)
    let mut sun = vec![0.0f32; n];
    for z in 0..NZ {
        for x in 0..NX {
            let mut above = 0.0;
            for y in (0..NY).rev() {
                let i = at(x, y, z);
                sun[i] = (-0.35 * (above + dens[i] * 0.5)).exp();
                above += dens[i];
            }
        }
    }
    // local occlusion: 9-wide separable box blur of density
    let blur = box_blur(&dens, 4);

    let mut color = vec![0u8; n * 4];
    let mut light = vec![0u8; n * 2];
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for i in 0..n {
        let inv = 1.0 / (dens[i] + 1e-4);
        color[i * 4] = q(dye[i][0] * inv);
        color[i * 4 + 1] = q(dye[i][1] * inv);
        color[i * 4 + 2] = q(dye[i][2] * inv);
        color[i * 4 + 3] = q((dens[i] / 5.0).sqrt());
        light[i * 2] = q(sun[i]);
        light[i * 2 + 1] = q((-0.9 * blur[i]).exp());
    }
    Volume { color, light }
}

fn box_blur(src: &[f32], r: usize) -> Vec<f32> {
    let mut a = src.to_vec();
    let mut b = vec![0.0f32; src.len()];
    let dims = [(NX, 1usize), (NY, NX), (NZ, NX * NY)];
    for &(len, stride) in &dims {
        for i in 0..src.len() {
            let coord = (i / stride) % len;
            let lo = coord.saturating_sub(r);
            let hi = (coord + r).min(len - 1);
            let base = i - coord * stride;
            let mut s = 0.0;
            for c in lo..=hi {
                s += a[base + c * stride];
            }
            b[i] = s / (2 * r + 1) as f32;
        }
        std::mem::swap(&mut a, &mut b);
    }
    a
}
