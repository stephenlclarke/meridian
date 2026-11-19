// Appended only for explicit authored-package passes, never the original opaque pipeline.
struct AuthoredSolid {
    @builtin(position) position:vec4<f32>, @location(0) color:vec4<f32>, @location(1) uv:vec2<f32>,
    @location(2) @interpolate(flat) layer:u32, @location(3) @interpolate(flat) emissive:u32,
    @location(4) normal:vec3<f32>,
}
@vertex fn authored_vertex(@location(0) p:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) color:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) layer:u32, @location(5) emissive:u32)->AuthoredSolid {
    var out:AuthoredSolid;out.position=project(to_view(p));out.color=color;out.uv=uv;out.layer=layer;out.emissive=emissive;
    out.normal=input.x.xyz*normal.x+input.y.xyz*normal.y+input.z.xyz*normal.z;return out;
}
@fragment fn authored_fragment(v:AuthoredSolid)->@location(0) vec4<f32> {
    // Sample in uniform control flow. The explicit no-map sentinel then selects white.
    let mapped=textureSample(diffuse,repeat_sampler,v.uv,i32(min(v.layer,2u)));
    let untextured=v.layer==0xffffffffu;
    let texel=select(mapped,vec4<f32>(1.0),untextured);
    // Untextured non-emissive surfaces receive restrained directional relief; original baked maps do not.
    let shade=0.60+0.40*max(0.0,dot(normalize(v.normal),normalize(vec3<f32>(-0.3,0.5,-1.0))));
    let rgb=texel.rgb*linear(v.color.rgb)*select(1.0,shade,untextured && v.emissive==0u);
    let alpha=texel.a*v.color.a;
    if alpha<=0.0 {discard;}
    return vec4<f32>(output_color(rgb),alpha);
}
