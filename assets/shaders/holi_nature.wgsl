// Procedural nature surfaces on top of the standard PBR pipeline.
//   kind 0: meadow (mottled greens, sunlit patches, daisy + buttercup speckles)
//   kind 1: foliage (leaf clumps: bumpy normal, crevice occlusion, colour variation)
//   kind 2: rock (grey speckled stone, bumpy)
//   kind 3: flowering bush (foliage + white blossoms)
// Everything is in world space, so any mesh (plane, hill, sphere cluster) can use it.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct Nature {
    kind: f32,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    freq: f32,
    bump: f32,
    flowers: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> nature: Nature;

// Hash without Sine, (c) 2014 David Hoskins, MIT License (https://www.shadertoy.com/view/4djSRW)
fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

fn vnoise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(mix(hash3(i), hash3(i + vec3(1.0, 0.0, 0.0)), u.x),
                mix(hash3(i + vec3(0.0, 1.0, 0.0)), hash3(i + vec3(1.0, 1.0, 0.0)), u.x), u.y);
    let b = mix(mix(hash3(i + vec3(0.0, 0.0, 1.0)), hash3(i + vec3(1.0, 0.0, 1.0)), u.x),
                mix(hash3(i + vec3(0.0, 1.0, 1.0)), hash3(i + vec3(1.0, 1.0, 1.0)), u.x), u.y);
    return mix(a, b, u.z);
}

fn fbm(p_in: vec3<f32>) -> f32 {
    var p = p_in;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i++) {
        s += a * vnoise(p);
        p = p * 2.07 + vec3(3.1, 1.7, 4.3);
        a *= 0.5;
    }
    return s / 0.9375;
}

// Speckles on a jittered grid: returns (mask, which) for flowers of the given cell size.
fn speckle(p: vec2<f32>, cell: f32, chance: f32) -> vec2<f32> {
    let g = p / cell;
    let id = floor(g);
    let h = hash3(vec3(id, 7.0));
    if (h > chance) {
        return vec2(0.0);
    }
    let c = vec2(hash3(vec3(id, 1.0)), hash3(vec3(id, 2.0))) * 0.6 + 0.2;
    let d = length(fract(g) - c);
    return vec2(1.0 - smoothstep(0.10, 0.16, d), hash3(vec3(id, 3.0)));
}

// Cheap bump: gradient of two value-noise octaves by forward differences (6 lookups).
fn bump(p: vec3<f32>, freq: f32) -> vec3<f32> {
    let q = p * freq;
    let e = 0.3;
    let n = vnoise(q) + 0.5 * vnoise(q * 2.3);
    let nx = vnoise(q + vec3(e, 0.0, 0.0)) + 0.5 * vnoise((q + vec3(e, 0.0, 0.0)) * 2.3);
    let ny = vnoise(q + vec3(0.0, e, 0.0)) + 0.5 * vnoise((q + vec3(0.0, e, 0.0)) * 2.3);
    let nz = vnoise(q + vec3(0.0, 0.0, e)) + 0.5 * vnoise((q + vec3(0.0, 0.0, e)) * 2.3);
    return vec3(nx - n, ny - n, nz - n) / e;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let m = nature;
    var pbr = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xyz;
    let kind = i32(m.kind + 0.5);
    let a = m.color_a.rgb;
    let b = m.color_b.rgb;

    if (kind == 0) {
        let big = fbm(vec3(p.xz * 0.18, 0.0));
        let mid = fbm(vec3(p.xz * 1.3, 5.0));
        let fine = vnoise(vec3(p.xz * 14.0, 9.0));
        var col = mix(a, b, smoothstep(0.3, 0.75, big)) * (0.78 + 0.3 * mid) * (0.88 + 0.2 * fine);
        // flowers: daisies and buttercups, fading out with distance so they don't shimmer
        let fade = 1.0 - smoothstep(18.0, 40.0, length(p.xz));
        let daisy = speckle(p.xz, 0.11, 0.05 * m.flowers);
        let butter = speckle(p.xz + 0.37, 0.09, 0.05 * m.flowers);
        col = mix(col, vec3(0.95, 0.95, 0.9), daisy.x * fade);
        col = mix(col, vec3(1.0, 0.82, 0.12), butter.x * fade);
        pbr.material.base_color = vec4(col, 1.0);
        pbr.material.perceptual_roughness = 0.92;
        pbr.N = normalize(pbr.N + vec3(mid - 0.5, 0.0, fine - 0.5) * 0.25);
    } else if (kind == 1 || kind == 3) {
        let clumps = fbm(p * m.freq);
        let fine = vnoise(p * m.freq * 5.0);
        var col = mix(a, b, smoothstep(0.35, 0.72, clumps)) * (0.8 + 0.3 * fine);
        if (kind == 3) {
            let s = speckle(vec2(p.x + p.y * 0.7, p.z - p.y * 0.4), 0.14, 0.18);
            col = mix(col, vec3(0.97, 0.96, 0.92), s.x);
        }
        pbr.material.base_color = vec4(col, 1.0);
        pbr.material.perceptual_roughness = 0.75;
        pbr.N = normalize(pbr.N + bump(p, m.freq) * m.bump);
        // leafy crevices are dark
        pbr.diffuse_occlusion = vec3(mix(0.6, 1.0, smoothstep(0.25, 0.7, clumps)));
    } else if (kind == 4) {
        // bark: vertical grain (noise stretched along y) with darker furrows
        let grain = fbm(vec3(p.x * m.freq * 3.0, p.y * m.freq * 0.35, p.z * m.freq * 3.0));
        let furrow = smoothstep(0.35, 0.6, grain);
        let col = mix(a, b, furrow) * (0.85 + 0.2 * vnoise(p * m.freq * 2.0));
        pbr.material.base_color = vec4(col, 1.0);
        pbr.material.perceptual_roughness = 0.9;
        pbr.N = normalize(pbr.N + bump(vec3(p.x * 3.0, p.y * 0.35, p.z * 3.0), m.freq) * m.bump);
        pbr.diffuse_occlusion = vec3(mix(0.6, 1.0, furrow));
    } else {
        let n = fbm(p * m.freq);
        let fine = vnoise(p * m.freq * 6.0);
        let col = mix(a, b, n) * (0.85 + 0.25 * fine);
        pbr.material.base_color = vec4(col, 1.0);
        pbr.material.perceptual_roughness = 0.8;
        pbr.N = normalize(pbr.N + bump(p, m.freq) * m.bump);
        pbr.diffuse_occlusion = vec3(mix(0.55, 1.0, n));
    }

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
