struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}
@vertex
fn vertex_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> Output {
    var output: Output;
    output.position = vec4<f32>(position, 1.0);
    output.color = color;
    return output;
}
@fragment
fn fragment_main(input: Output) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
