// Bright summer sky: saturated blue, warm sun glow, big puffy sunlit cumulus.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view, globals}
struct HoliSky {
    sun_dir: vec3<f32>,
    gain: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: HoliSky;

// Hash without Sine, (c) 2014 David Hoskins, MIT License (https://www.shadertoy.com/view/4djSRW)
fn hash(p: vec2<f32>) -> f32 {
    // arithmetic hash: sin()-based hashes turn blocky at large coordinates
    var p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
               mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 6; i++) {
        s += a * noise(p);
        p = p * 2.03 + vec2(1.7, 9.2);
        a *= 0.5;
    }
    return s;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let rd = normalize(in.world_position.xyz - view.world_position);
    let sun_dir = normalize(material.sun_dir);
    let h = max(rd.y, 0.0);
    var col = mix(vec3(0.62, 0.80, 1.0), vec3(0.05, 0.30, 0.95), pow(h, 0.45)) * 1.15;
    if (rd.y < 0.0) {
        col = vec3(0.55, 0.72, 0.45);
    }
    let mu = max(dot(rd, sun_dir), 0.0);
    col += vec3(1.0, 0.85, 0.6) * (pow(mu, 8.0) * 0.45 + pow(mu, 64.0) * 0.8 + pow(mu, 3000.0) * 40.0);

    // cumulus on a high plane: fbm density, lit from the sun side, grey bellies
    if (rd.y > 0.01) {
        let uv = rd.xz / (rd.y + 0.08) * 0.9 + vec2(globals.time * 0.006, 0.0);
        let d = fbm(uv);
        let cover = smoothstep(0.50, 0.72, d);
        let toward_sun = normalize(sun_dir.xz + vec2(1e-4)) * 0.08;
        let shade = clamp((d - fbm(uv + toward_sun)) * 5.0 + 0.6, 0.0, 1.0);
        let cloud = mix(vec3(0.72, 0.76, 0.85), vec3(1.05, 1.02, 0.97), shade)
                  + vec3(1.0, 0.85, 0.6) * pow(mu, 6.0) * 0.4;
        col = mix(col, cloud, cover * smoothstep(0.01, 0.12, rd.y) * 0.95);
    }
    return vec4(col * material.gain, 1.0);
}
