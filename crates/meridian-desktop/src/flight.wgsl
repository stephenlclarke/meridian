struct View { projection: vec4<f32> }
@group(0) @binding(0) var<uniform> view: View;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vertex_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> VertexOutput {
    let aspect = view.projection.x;
    let focal = view.projection.y;
    let near = view.projection.z;
    let far = view.projection.w;
    var output: VertexOutput;
    // Local +Z is forward; wgpu depth is [0, 1]. Hardware clips the near plane.
    output.position = vec4<f32>(position.x * focal / aspect, position.y * focal,
        position.z * far / (far - near) - near * far / (far - near), position.z);
    output.color = color;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
