// Grass sway. uv.y is the height along the blade (0 root, 1 tip): roots stay put, tips
// swing along the breeze with rolling gusts. Shared by the main pass and the prepass
// so the depth of field sees the same blades.
#import bevy_pbr::{mesh_functions, view_transformations::position_world_to_clip}
#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{Vertex, VertexOutput}
#else
#import bevy_pbr::forward_io::{Vertex, VertexOutput}
#endif

struct Grass {
    now: f32,
    wind: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> grass: Grass;

fn sway(p: vec3<f32>, h: f32) -> vec3<f32> {
    let t = grass.now;
    let w = grass.wind;
    // big gust fronts travelling across the meadow + small flutter
    let front = dot(p.xz, normalize(w.xz + vec2(1e-4))) * 0.35 - t * 0.8;
    let gust = 0.55 + 0.45 * sin(front) * sin(front * 0.37 + 1.7);
    let flutter = sin(t * 2.3 + p.x * 3.7 + p.z * 2.9) * 0.12;
    let bend = h * h * w.w * (gust + flutter);
    let side = vec3(w.x, 0.0, w.z) * bend;
    return vec3(side.x, -abs(bend) * 0.25, side.z);
}

@vertex
fn vertex(vin: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vin.instance_index);
    var wp = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vin.position, 1.0));
#ifdef VERTEX_UVS_A
    wp = vec4(wp.xyz + sway(wp.xyz, vin.uv.y), 1.0);
#endif
    out.world_position = wp;
    out.position = position_world_to_clip(wp.xyz);

#ifdef PREPASS_PIPELINE
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vin.normal, vin.instance_index);
#endif
#endif
#ifdef MOTION_VECTOR_PREPASS
    out.previous_world_position = out.world_position;
#endif
#ifdef VERTEX_UVS_A
    out.uv = vin.uv;
#endif
#else
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vin.normal, vin.instance_index);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vin.uv;
#endif
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = vin.uv_b;
#endif
#ifdef VERTEX_COLORS
    out.color = vin.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vin.instance_index;
#endif
    return out;
}
