struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) colour:vec3<f32> }
@group(0) @binding(0) var glyphs:texture_2d<f32>;
@group(0) @binding(1) var glyph_sampler:sampler;
@vertex fn vertex_main(@location(0) position:vec2<f32>,@location(1) uv:vec2<f32>,@location(2) colour:vec3<f32>)->Output {
    var out:Output;out.position=vec4(position,0.,1.);out.uv=uv;out.colour=colour;return out;
}
@fragment fn fragment_main(in:Output)->@location(0) vec4<f32> {
    if textureSample(glyphs,glyph_sampler,in.uv).r<0.5 {discard;}
    return vec4(in.colour,1.);
}
