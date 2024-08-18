struct SceneUBO {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
}

struct ObjectPushConstants {
    model: mat4x4<f32>
}

var<push_constant> object: ObjectPushConstants;

@group(0) @binding(0)
var<uniform> scene: SceneUBO;
//@group(0) @binding(1)
//var<uniform> scene: CameraData;

@group(1) @binding(0)
var colorTexture: texture_2d<f32>;
@group(1) @binding(1)
var colorTextureSampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,

    @location(0) fragColor: vec3<f32>,
    @location(1) fragCoord: vec2<f32>,
}

@vertex
fn vs(in: VertexInput) -> VertexOutput {
    var output = VertexOutput();

    output.position = scene.view_projection * object.model * vec4<f32>(in.position, 1.0);

    output.fragColor = in.color;
    output.fragCoord = in.uv;

    return output;
}

@fragment
fn fs(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(colorTexture, colorTextureSampler, in.fragCoord);
}