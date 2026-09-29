//! Procedural geometry: grass tufts + flowers, broadleaf trees with leaf cards,
//! merged ellipsoids, leaf texture.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::math::Vec3;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Small deterministic RNG (xorshift64*), enough for scattering props.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f()
    }
    pub fn int(&mut self, lo: u32, hi_exclusive: u32) -> u32 {
        lo + (self.next_u64() % u64::from(hi_exclusive - lo)) as u32
    }
    /// Standard normal (Box-Muller).
    pub fn normal(&mut self) -> f32 {
        let u = self.f().max(1e-7);
        let v = self.f();
        (-2.0 * u.ln()).sqrt() * (TAU * v).cos()
    }
    pub fn normal3(&mut self) -> Vec3 {
        Vec3::new(self.normal(), self.normal(), self.normal())
    }
    pub fn unit(&mut self) -> Vec3 {
        self.normal3().normalize_or(Vec3::Y)
    }
}

fn mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Option<Vec<[f32; 2]>>,
    colors: Option<Vec<[f32; 4]>>,
    indices: Vec<u32>,
) -> Mesh {
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    if let Some(uv) = uvs {
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    if let Some(c) = colors {
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
    }
    m.insert_indices(Indices::U32(indices));
    m
}

/// ~700k curved blades growing in tufts, plus daisies and buttercups, one mesh.
/// uv.y = height along the blade, read by the wind vertex shader.
pub fn grass_mesh(seed: u64) -> Mesh {
    let mut g = Rng::new(seed);
    let tufts = |k: usize, r0: f32, r1: f32, g: &mut Rng| -> Vec<(f32, f32)> {
        (0..k)
            .map(|_| {
                let r = (g.range(r0 * r0, r1 * r1)).sqrt();
                let th = g.f() * TAU;
                (r * th.cos(), r * th.sin())
            })
            .collect()
    };
    let inner = tufts(46_000, 2.35, 14.2, &mut g);
    let outer = tufts(22_000, 14.2, 30.0, &mut g);

    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut uv = Vec::new();
    let mut col = Vec::new();
    let mut idx = Vec::new();

    for (group, per) in [(&inner, 12), (&outer, 8)] {
        for &(tx, tz) in group.iter() {
            let t_h = 0.16 + 0.22 * g.f().powf(1.3);
            let t_col = g.f();
            let patch = 0.5 + 0.5 * (tx * 0.45 + 1.3).sin() * (tz * 0.37 - 0.4).sin();
            let t_lean = (g.normal() * 0.35, g.normal() * 0.35);
            for _ in 0..per {
                let mut bx = tx + g.normal() * 0.045;
                let mut bz = tz + g.normal() * 0.045;
                let d = (bx * bx + bz * bz).sqrt();
                if d < 2.3 {
                    bx *= 2.3 / d.max(1e-3);
                    bz *= 2.3 / d.max(1e-3);
                }
                let h = t_h * (0.55 + 0.6 * g.f());
                let (ox, oz) = (bx - tx, bz - tz);
                let on = (ox * ox + oz * oz).sqrt().max(1e-3);
                let lx = (t_lean.0 * 0.6 + ox / on * 0.5 + g.normal() * 0.2) * h;
                let lz = (t_lean.1 * 0.6 + oz / on * 0.5 + g.normal() * 0.2) * h;
                let ang = lz.atan2(lx) + PI / 2.0 + g.normal() * 0.4;
                let w = 0.010 + 0.008 * g.f();
                let (wx, wz) = (ang.cos() * w, ang.sin() * w);
                let lean_len = (lx * lx + lz * lz).sqrt();
                let at = |f: f32, width: f32| -> ([f32; 3], [f32; 3]) {
                    let x = bx + lx * f * f;
                    let z = bz + lz * f * f;
                    let y = h * f * (1.0 - 0.25 * f * f * lean_len / h.max(1e-3));
                    (
                        [x - wx * width, y, z - wz * width],
                        [x + wx * width, y, z + wz * width],
                    )
                };
                let (b0, b1) = at(0.0, 1.0);
                let (m0, m1) = at(0.5, 0.8);
                let (tip, _) = at(1.0, 0.0);
                let base = pos.len() as u32;
                pos.extend_from_slice(&[b0, b1, m0, m1, tip]);
                uv.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [0.0, 0.5], [1.0, 0.5], [0.5, 1.0]]);
                idx.extend_from_slice(&[0, 1, 2, 1, 3, 2, 2, 3, 4].map(|i| base + i));
                // colour: dark roots, sunny tips; tufts and big patches vary
                let c = t_col * 0.6 + patch * 0.4;
                let k = 0.8 + 0.35 * g.f();
                let tip_c = [
                    (0.13 + 0.2 * c) * k,
                    (0.36 + 0.14 * c) * k,
                    (0.035 + 0.03 * (1.0 - c)) * k,
                    1.0,
                ];
                let mid_c = [tip_c[0] * 0.55, tip_c[1] * 0.55, tip_c[2] * 0.55, 1.0];
                let root_c = [0.025, 0.08, 0.012, 1.0];
                col.extend_from_slice(&[root_c, root_c, mid_c, mid_c, tip_c]);
                let n = Vec3::new(lx / h.max(1e-3) * 0.3, 1.0, lz / h.max(1e-3) * 0.3).normalize();
                for _ in 0..5 {
                    nrm.push(n.to_array());
                }
            }
        }
    }

    // flowers: 8-petal daisies and 5-petal-ish buttercups (same fan) above the grass
    for _ in 0..7000 {
        let r = (g.range(2.4 * 2.4, 14.0 * 14.0)).sqrt();
        let th = g.f() * TAU;
        let (fx, fz) = (r * th.cos(), r * th.sin());
        let fy = 0.12 + 0.2 * g.f();
        let daisy = g.f() < 0.55;
        let size = if daisy { 0.034 } else { 0.024 } * (0.8 + 0.4 * g.f());
        let rot = g.f() * TAU;
        let (tx, tz) = (g.normal() * 0.25, g.normal() * 0.25);
        let petal = if daisy { [0.8, 0.8, 0.76, 1.0] } else { [0.95, 0.7, 0.04, 1.0] };
        let centre = if daisy { [1.0, 0.72, 0.05, 1.0] } else { [0.95, 0.6, 0.03, 1.0] };
        let base = pos.len() as u32;
        pos.push([fx, fy + 0.006, fz]);
        col.push(centre);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU + rot;
            let (ox, oz) = (a.cos() * size, a.sin() * size);
            pos.push([fx + ox, fy + ox * tx + oz * tz, fz + oz]);
            col.push(petal);
        }
        for _ in 0..9 {
            nrm.push([0.0, 1.0, 0.0]);
            uv.push([0.5, 0.9]); // flowers sway like blade tips
        }
        for k in 1..=8u32 {
            idx.extend_from_slice(&[base, base + k % 8 + 1, base + k]);
        }
    }
    mesh(pos, nrm, Some(uv), Some(col), idx)
}

/// Many ellipsoids (centre, radii, yaw) merged into one mesh.
pub fn blob_mesh(blobs: &[(Vec3, Vec3, f32)]) -> Mesh {
    let (lat, lon) = (10usize, 16usize);
    let mut unit = Vec::new();
    for i in 0..=lat {
        let t = PI * i as f32 / lat as f32;
        for j in 0..=lon {
            let p = TAU * j as f32 / lon as f32;
            unit.push(Vec3::new(t.sin() * p.cos(), t.cos(), t.sin() * p.sin()));
        }
    }
    let mut tri = Vec::new();
    for i in 0..lat {
        for j in 0..lon {
            let a = (i * (lon + 1) + j) as u32;
            let b = ((i + 1) * (lon + 1) + j) as u32;
            tri.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let (mut pos, mut nrm, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    for (k, &(c, r, yaw)) in blobs.iter().enumerate() {
        let (cy, sy) = (yaw.cos(), yaw.sin());
        let rot = |v: Vec3| Vec3::new(cy * v.x + sy * v.z, v.y, -sy * v.x + cy * v.z);
        for &u in &unit {
            pos.push((rot(u * r) + c).to_array());
            nrm.push(rot(u / r).normalize().to_array());
        }
        let off = (k * unit.len()) as u32;
        idx.extend(tri.iter().map(|i| i + off));
    }
    mesh(pos, nrm, None, None, idx)
}

const SIDES: usize = 7;

fn frame(d: Vec3) -> (Vec3, Vec3) {
    let a = if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = d.cross(a).normalize();
    (u, d.cross(u))
}

#[derive(Default)]
pub struct Forest {
    pub bark_pos: Vec<[f32; 3]>,
    pub bark_nrm: Vec<[f32; 3]>,
    pub bark_idx: Vec<u32>,
    pub leaf_pos: Vec<[f32; 3]>,
    pub leaf_nrm: Vec<[f32; 3]>,
    pub leaf_uv: Vec<[f32; 2]>,
    pub leaf_col: Vec<[f32; 4]>,
}

impl Forest {
    pub fn bark_mesh(&self) -> Mesh {
        mesh(self.bark_pos.clone(), self.bark_nrm.clone(), None, None, self.bark_idx.clone())
    }

    pub fn leaf_mesh(&self) -> Mesh {
        let cards = self.leaf_pos.len() as u32 / 4;
        let idx = (0..cards)
            .flat_map(|c| [0, 1, 2, 0, 2, 3].map(|i| c * 4 + i))
            .collect();
        mesh(
            self.leaf_pos.clone(),
            self.leaf_nrm.clone(),
            Some(self.leaf_uv.clone()),
            Some(self.leaf_col.clone()),
            idx,
        )
    }

    fn tube(&mut self, pts: &[Vec3], radii: &[f32]) {
        let base = self.bark_pos.len() as u32;
        for (i, &p) in pts.iter().enumerate() {
            let d = (pts[(i + 1).min(pts.len() - 1)] - pts[i.saturating_sub(1)]).normalize_or(Vec3::Y);
            let (u, v) = frame(d);
            for s in 0..SIDES {
                let a = s as f32 / SIDES as f32 * TAU;
                let n = u * a.cos() + v * a.sin();
                self.bark_pos.push((p + n * radii[i]).to_array());
                self.bark_nrm.push(n.to_array());
            }
        }
        for r in 0..pts.len() - 1 {
            for s in 0..SIDES {
                let a = (r * SIDES + s) as u32 + base;
                let b = (r * SIDES + (s + 1) % SIDES) as u32 + base;
                let (c, d) = (a + SIDES as u32, b + SIDES as u32);
                self.bark_idx.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn cluster(
        &mut self,
        g: &mut Rng,
        c: Vec3,
        radius: f32,
        crown_c: Vec3,
        crown_r: Vec3,
        cards: usize,
        size: f32,
        tint: Vec3,
    ) {
        for _ in 0..cards {
            let dir = g.unit();
            let r = radius * (0.55 + 0.45 * g.f().sqrt());
            let centre = c + dir * r;
            let face = (dir + g.normal3() * 0.8).normalize_or(Vec3::Y);
            let up = face.cross(g.normal3()).normalize_or(Vec3::X);
            let right = up.cross(face);
            let s = size * (0.75 + 0.5 * g.f()) * 0.5;
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                self.leaf_pos.push((centre + (right * x + up * y) * s).to_array());
            }
            // shading normal: out of the clump, blended with out of the crown ellipsoid
            let out_c = (centre - c).normalize_or(Vec3::Y);
            let rel = (centre - crown_c) / crown_r;
            let out_t = (rel / crown_r).normalize_or(Vec3::Y);
            let n = (out_c * 0.3 + out_t * 0.7).normalize_or(Vec3::Y);
            // occlusion: dark deep inside the crown and underneath, sunny on top
            let shell = rel.length().clamp(0.0, 1.0);
            let local = ((r / radius - 0.55) / 0.45).clamp(0.0, 1.0);
            let under = (-rel.y).clamp(0.0, 1.0);
            let mut light =
                (0.3 + 0.7 * shell * shell) * (0.75 + 0.25 * local) * (1.0 - 0.35 * under);
            light = light * (0.9 + 0.25 * rel.y.clamp(0.0, 1.0)) + g.normal() * 0.05;
            let col = (tint * light).clamp(Vec3::ZERO, Vec3::ONE);
            for uv in [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]] {
                self.leaf_nrm.push(n.to_array());
                self.leaf_uv.push(uv);
                self.leaf_col.push([col.x, col.y, col.z, 1.0]);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn limb(&mut self, g: &mut Rng, tips: &mut Vec<Vec3>, p0: Vec3, d: Vec3, length: f32, r0: f32, depth: u32) {
        let (mut pts, mut radii) = (vec![p0], vec![r0]);
        let (mut p, mut dd) = (p0, d);
        for k in 0..3 {
            dd = (dd + g.normal3() * 0.07 + Vec3::new(0.0, 0.05, 0.0)).normalize();
            p += dd * length / 3.0;
            pts.push(p);
            radii.push(r0 * (1.0 - 0.13 * (k + 1) as f32));
        }
        self.tube(&pts, &radii);
        tips.push(p);
        if depth == 0 {
            return;
        }
        for _ in 0..2 {
            let spin = g.range(0.0, TAU);
            let (u, v) = frame(dd);
            let spread = g.range(0.35, 0.65);
            let mut nd = dd * spread.cos() + (u * spin.cos() + v * spin.sin()) * spread.sin();
            nd.y = nd.y.max(0.25);
            let r_end = *radii.last().unwrap();
            let len = length * g.range(0.6, 0.75);
            self.limb(g, tips, p, nd.normalize(), len, r_end * 0.72, depth - 1);
        }
    }

    /// A broadleaf tree: straight flared trunk, 3-4 upward limbs, and a rounded crown
    /// packed with leaf clumps (darker inside and underneath, sunny on top).
    pub fn grow(&mut self, g: &mut Rng, base: Vec3, scale: f32, tint: Vec3, leaf_density: f32) {
        let mut tips = Vec::new();
        let trunk_h = scale * g.range(1.15, 1.45);
        let top = base + Vec3::new(g.normal() * 0.04, 1.0, g.normal() * 0.04) * trunk_h;
        let r0 = 0.2 * scale;
        self.tube(
            &[
                base - Vec3::new(0.0, 0.05, 0.0),
                base + Vec3::new(0.0, 0.12 * scale, 0.0),
                base + (top - base) * 0.5,
                top,
            ],
            &[r0 * 1.7, r0 * 1.15, r0 * 0.95, r0 * 0.8],
        );
        let limbs = g.int(3, 5);
        let spin0 = g.range(0.0, TAU);
        for i in 0..limbs {
            let a = spin0 + i as f32 / limbs as f32 * TAU + g.normal() * 0.3;
            let spread = g.range(0.5, 0.8);
            let d = Vec3::new(a.cos() * spread.sin(), spread.cos(), a.sin() * spread.sin());
            let len = scale * g.range(0.8, 1.0);
            self.limb(g, &mut tips, top, d, len, r0 * 0.62, 2);
        }
        let centre = top + Vec3::new(0.0, 0.95 * scale, 0.0);
        let rad = Vec3::new(1.45, 1.2, 1.45) * scale
            * Vec3::new(g.range(0.9, 1.1), g.range(0.9, 1.1), g.range(0.9, 1.1));
        let clumps = (34.0 * leaf_density) as usize + 6;
        for _ in 0..clumps {
            let dir = g.unit();
            let shell = 0.55 + 0.45 * g.f().powf(0.35);
            let near = tips[g.int(0, tips.len() as u32) as usize];
            let c = (centre + dir * rad * shell) * 0.75 + near * 0.25;
            let cards = (95.0 * leaf_density) as usize;
            let radius = 0.42 * scale * g.range(0.8, 1.15);
            let t = tint * g.range(0.88, 1.12);
            self.cluster(g, c, radius, centre, rad, cards, 0.13 * scale, t);
        }
    }
}

/// spots: (base position, scale, tint index).
pub fn forest(spots: &[(Vec3, f32, usize)], seed: u64) -> Forest {
    let tints = [
        Vec3::new(0.26, 0.52, 0.08),
        Vec3::new(0.32, 0.58, 0.10),
        Vec3::new(0.22, 0.46, 0.09),
    ];
    let mut g = Rng::new(seed);
    let mut f = Forest::default();
    for &(base, s, k) in spots {
        let density = if (base.x * base.x + base.z * base.z).sqrt() < 30.0 { 1.0 } else { 0.45 };
        f.grow(&mut g, base, s, tints[k % tints.len()], density);
    }
    f
}

/// RGBA8 card with ~12 small pointed leaves; alpha is the leaf mask.
pub fn leaf_texture(n: usize, seed: u64) -> Vec<u8> {
    let mut g = Rng::new(seed);
    let mut rgba = vec![0u8; n * n * 4];
    for _ in 0..12 {
        let (cx, cy) = (g.range(0.2, 0.8), g.range(0.2, 0.8));
        let a = g.range(0.0, TAU);
        let (ln, wd) = (g.range(0.15, 0.21), g.range(0.08, 0.11));
        let jitter = g.range(-0.08, 0.08);
        for py in 0..n {
            for px in 0..n {
                let x = px as f32 / (n - 1) as f32 - cx;
                let y = py as f32 / (n - 1) as f32 - cy;
                let u = x * a.cos() + y * a.sin();
                let v = -x * a.sin() + y * a.cos();
                let t = (u / ln).clamp(-1.0, 1.0);
                let half = wd * (1.0 - t * t).max(0.0).powf(0.8) * (1.0 - 0.35 * t);
                if u.abs() < ln && v.abs() < half {
                    let midrib = if v.abs() < 0.005 { 0.08 } else { 0.0 };
                    let shade = (0.8 + 0.1 * (v / (half + 1e-6)) - midrib + jitter).clamp(0.0, 1.0);
                    let i = (py * n + px) * 4;
                    let s = (shade * 255.0) as u8;
                    rgba[i..i + 4].copy_from_slice(&[s, s, s, 255]);
                }
            }
        }
    }
    rgba
}
