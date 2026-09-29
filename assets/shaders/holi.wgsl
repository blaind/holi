// Coloured powder clouds: raymarch the powder voxels.
//   color: rgb = powder colour, a = sqrt(density / 5)
//   light: r = sunlight transmittance (self-shadowing), g = local occlusion
// The march stops at opaque geometry via the depth prepass.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view, globals}
#import bevy_pbr::prepass_utils
struct HoliVolume {
    box_min: vec3<f32>,
    cell: f32,
    box_size: vec3<f32>,
    step_scale: f32,
    sun_dir: vec3<f32>,
    sun_gain: f32,
    sky_gain: f32,
    density: f32,
    detail_amount: f32,
    detail_freq: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: HoliVolume;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var vol: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var vol_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var lvol: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var lvol_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var detail_tex: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var detail_sampler: sampler;

// Hash without Sine, (c) 2014 David Hoskins, MIT License (https://www.shadertoy.com/view/4djSRW)
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn box_hit(ro: vec3<f32>, rd: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec2<f32> {
    let inv = 1.0 / rd;
    let t0 = (bmin - ro) * inv;
    let t1 = (bmax - ro) * inv;
    let tmin = min(t0, t1);
    let tmax = max(t0, t1);
    return vec2(max(max(tmin.x, tmin.y), tmin.z), min(min(tmax.x, tmax.y), tmax.z));
}

// Small-scale billows the grid can't resolve: 3 octaves of value noise from a tiling
// random volume, slowly churning over time.
fn detail(p: vec3<f32>, freq: f32) -> f32 {
    let t = globals.time;
    let q = p * freq + vec3(0.0, -t * 0.05, t * 0.02);
    let a = textureSampleLevel(detail_tex, detail_sampler, q, 0.0).r;
    let b = textureSampleLevel(detail_tex, detail_sampler, q * 2.03 + vec3(0.31, t * 0.03, 0.17), 0.0).g;
    let c = textureSampleLevel(detail_tex, detail_sampler, q * 4.11 + vec3(0.73, 0.11, t * 0.05), 0.0).b;
    return (a * 0.62 + b * 0.28 + c * 0.10);
}

fn scene_t(pos: vec4<f32>, ro: vec3<f32>, rd: vec3<f32>) -> f32 {
#ifdef DEPTH_PREPASS
    let depth = prepass_utils::prepass_depth(pos, 0u);
    if (depth <= 0.0) {
        return 1e9;
    }
    let uv = (pos.xy - view.viewport.xy) / view.viewport.zw;
    let ndc = vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let w = view.world_from_clip * ndc;
    return dot(w.xyz / w.w - ro, rd);
#else
    return 1e9;
#endif
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let m = material;
    let ro = view.world_position;
    let rd = normalize(in.world_position.xyz - ro);
    let bmin = m.box_min;
    let hit = box_hit(ro, rd, bmin, bmin + m.box_size);
    let t_start = max(hit.x, 0.0);
    let t_end = min(hit.y, scene_t(in.position, ro, rd));
    if (t_end <= t_start) {
        discard;
    }

    let dt = m.cell * m.step_scale;
    let n = min(i32((t_end - t_start) / dt) + 1, 600);
    var t = t_start + dt * hash12(in.position.xy + fract(globals.time * 7.13) * 131.0);

    // forward-scattering sun glow when looking toward the sun
    let mu = dot(rd, normalize(m.sun_dir));
    let phase = 0.75 + 0.9 * pow(max(mu, 0.0), 6.0);
    let sun = vec3(1.0, 0.95, 0.85) * m.sun_gain * phase;
    let sky = vec3(0.50, 0.68, 1.0) * m.sky_gain;
    let bounce = vec3(0.30, 0.45, 0.16) * m.sky_gain;

    var T = 1.0;
    var col = vec3(0.0);
    for (var i = 0; i < n; i++) {
        if (t > t_end) {
            break;
        }
        let p = ro + rd * t;
        let uvw = (p - bmin) / m.box_size;
        let c = textureSampleLevel(vol, vol_sampler, uvw, 0.0);
        if (c.a > 0.02) {
            let l = textureSampleLevel(lvol, lvol_sampler, uvw, 0.0);
            let base = c.a * c.a * 5.0;
            // carve the detail mostly at the soft rims, keep cores solid
            let rim = 1.0 - smoothstep(0.4, 4.0, base);
            let n = detail(p, m.detail_freq);
            // erosion: low noise eats into the rims, high noise puffs them out
            let lump = smoothstep(0.32, 0.68, n);
            let dens = max(base * (0.35 + 1.3 * lump) - m.detail_amount * rim * (1.0 - lump) * 0.6, 0.0)
                     * m.detail_amount + base * (1.0 - m.detail_amount);
            // vivid powder: push colour saturation a little
            let luma = dot(c.rgb, vec3(0.299, 0.587, 0.114));
            let albedo = clamp(mix(vec3(luma), c.rgb, 1.25), vec3(0.0), vec3(1.0));
            let height = uvw.y;
            let light = sun * l.r + sky * (0.35 + 0.65 * l.g) + bounce * l.g * (1.0 - height);
            let a = exp(-dens * m.density * dt);
            col += T * albedo * light * (1.0 - a);
            T *= a;
            if (T < 0.004) {
                break;
            }
        }
        t += dt;
    }
    return vec4(col, 1.0 - T);
}
