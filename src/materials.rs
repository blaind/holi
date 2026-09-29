//! Custom materials: powder volume, sky dome, procedural nature and wind-swept grass.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialExtension, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

#[derive(ShaderType, Debug, Clone)]
pub struct VolumeParams {
    pub box_min: Vec3,
    pub cell: f32,
    pub box_size: Vec3,
    pub step_scale: f32,
    pub sun_dir: Vec3,
    pub sun_gain: f32,
    pub sky_gain: f32,
    pub density: f32,
    pub detail_amount: f32,
    pub detail_freq: f32,
}

/// Raymarched powder: colour + light 3D textures, plus a tiling noise volume for detail.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HoliVolume {
    #[uniform(0)]
    pub params: VolumeParams,
    #[texture(1, dimension = "3d")]
    #[sampler(2)]
    pub vol: Handle<Image>,
    #[texture(3, dimension = "3d")]
    #[sampler(4)]
    pub lvol: Handle<Image>,
    #[texture(5, dimension = "3d")]
    #[sampler(6)]
    pub detail: Handle<Image>,
}

impl Material for HoliVolume {
    fn fragment_shader() -> ShaderRef {
        "shaders/holi.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }
}

#[derive(ShaderType, Debug, Clone)]
pub struct SkyParams {
    pub sun_dir: Vec3,
    pub gain: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HoliSky {
    #[uniform(0)]
    pub params: SkyParams,
}

impl Material for HoliSky {
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_sky.wgsl".into()
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // we look at the dome from inside
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

#[derive(ShaderType, Debug, Clone, Default)]
pub struct NatureParams {
    pub kind: f32,
    pub color_a: Vec4,
    pub color_b: Vec4,
    pub freq: f32,
    pub bump: f32,
    pub flowers: f32,
}

/// Meadow (0), foliage (1), rock (2), flowering bush (3), bark (4).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Nature {
    #[uniform(100)]
    pub params: NatureParams,
}

impl MaterialExtension for Nature {
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_nature.wgsl".into()
    }
}

pub type NatureMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, Nature>;

pub fn nature(kind: f32, a: [f32; 3], b: [f32; 3], freq: f32, bump: f32, flowers: f32) -> NatureMaterial {
    NatureMaterial {
        base: StandardMaterial::default(),
        extension: Nature {
            params: NatureParams {
                kind,
                color_a: Vec4::new(a[0], a[1], a[2], 1.0),
                color_b: Vec4::new(b[0], b[1], b[2], 1.0),
                freq,
                bump,
                flowers,
            },
        },
    }
}

#[derive(ShaderType, Debug, Clone)]
pub struct GrassParams {
    pub now: f32,
    /// xyz direction, w strength
    pub wind: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Grass {
    #[uniform(100)]
    pub params: GrassParams,
}

impl MaterialExtension for Grass {
    fn vertex_shader() -> ShaderRef {
        "shaders/holi_grass.wgsl".into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        "shaders/holi_grass.wgsl".into()
    }
}

pub type GrassMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, Grass>;
