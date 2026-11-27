struct Camera {
    view_proj: mat4x4<f32>,
};

struct Draw {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> draw: Draw;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * draw.model * vec4<f32>(position, 1.0);
    // Models are rigid (pose * origin), so the model matrix transforms normals too.
    out.normal = (draw.model * vec4<f32>(normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(0.4, 0.3, 1.0));
    let diffuse = abs(dot(normalize(in.normal), light));
    // shortcut: translucent URDF colours are drawn opaque; add a depth-sorted blended pass if a robot needs them.
    return vec4<f32>(draw.color.rgb * (0.35 + 0.65 * diffuse), 1.0);
}
