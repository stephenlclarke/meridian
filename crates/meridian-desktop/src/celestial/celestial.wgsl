// Adapted from frozen L337 PlanetGPUShader.swift and CelestialRendering.swift.
// All physical bodies and terrain data are provided by the simulation/presentation snapshot.
struct Uniforms {
    viewport: vec4<f32>, geometry: vec4<f32>, depth: vec4<f32>, light: vec4<f32>,
    ocean: vec4<f32>, land: vec4<f32>, highland: vec4<f32>, vegetation: vec4<f32>, cloud: vec4<f32>,
    cloud_levels: vec4<f32>, atmosphere: vec4<f32>, night_levels: vec4<f32>, night: vec4<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> terrain: array<vec4<f32>, 32768>;
struct VertexOut { @builtin(position) position: vec4<f32>, @location(0) normal_xy: vec2<f32> }
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let corners = array<vec2<f32>,6>(vec2(-1.,-1.),vec2(1.,-1.),vec2(-1.,1.),vec2(-1.,1.),vec2(1.,-1.),vec2(1.,1.));
    let xy = corners[index] * u.depth.w;
    let pixel = u.viewport.zw + vec2(xy.x,-xy.y) * u.geometry.x;
    var o:VertexOut;
    o.position=vec4(2.*pixel.x/u.viewport.x-1.,1.-2.*pixel.y/u.viewport.y,0.,1.);
    o.normal_xy=xy;return o;
}
fn map_sample(longitude:f32,latitude:f32)->vec4<f32> {
    let pi=3.141592653589793;
    let x=fract(longitude/(2.*pi)+0.5)*256.;
    let y=clamp((0.5-latitude/pi)*127.,0.,127.);
    let ix=u32(x);let iy=u32(y);let nx=(ix+1u)%256u;let ny=min(iy+1u,127u);
    return mix(mix(terrain[iy*256u+ix],terrain[iy*256u+nx],fract(x)),mix(terrain[ny*256u+ix],terrain[ny*256u+nx],fract(x)),fract(y));
}
// Reference palette math produces sRGB image colours. Convert once for an sRGB render target.
fn output_colour(premultiplied:vec3<f32>,alpha:f32)->vec4<f32> {
    if u.night.w<0.5 || alpha<=0. {return vec4(premultiplied,alpha);}
    let c=clamp(premultiplied/alpha,vec3(0.),vec3(1.));
    let linear=select(c/12.92,pow((c+0.055)/1.055,vec3(2.4)),c>vec3(0.04045));
    return vec4(linear*alpha,alpha);
}
struct FragmentOut { @location(0) colour:vec4<f32>, @builtin(frag_depth) depth:f32 }
fn depth_at(normal_z:f32)->f32 {
    let z=max(u.depth.x,u.geometry.z-normal_z*u.geometry.w);
    return clamp(u.depth.y/(u.depth.y-u.depth.x)-u.depth.x*u.depth.y/((u.depth.y-u.depth.x)*z),0.,1.);
}
@fragment fn surface_main(in:VertexOut)->FragmentOut {
    let xy=in.normal_xy;let r=dot(xy,xy);
    if r>1. {discard;}
    let nz=sqrt(max(0.,1.-r));
    var colour:vec3<f32>;
    if u.depth.z>0.5 {
        colour=select(vec3(1.,0.8,0.133333333),vec3(1.,1.,0.),r<0.86*0.86);
    } else {
        let latitude=asin(clamp(xy.y,-1.,1.));
        let longitude=atan2(xy.x,nz)+u.geometry.y;
        let values=map_sample(longitude,latitude);
        let cloud_value=map_sample(longitude+u.geometry.y*0.08,latitude).y;
        let coast=smoothstep(u.land.w-0.018,u.land.w+0.018,values.x);
        let high=smoothstep(u.land.w+0.04,0.8,values.x);
        var land=mix(u.land.xyz,u.highland.xyz,high*u.highland.w);
        let vegetation=smoothstep(u.land.w+0.015,u.land.w+0.13,values.x)*(1.-high)*u.vegetation.w;
        land=mix(land,u.vegetation.xyz,vegetation);
        var surface=mix(u.ocean.xyz,land,coast);
        surface=mix(surface,vec3(0.81,0.89,0.93),smoothstep(u.ocean.w-0.06,u.ocean.w+0.02,abs(xy.y)));
        let cloud_opacity=smoothstep(u.cloud_levels.x,u.cloud_levels.y,cloud_value)*u.cloud.w;
        surface=mix(surface,u.cloud.xyz,cloud_opacity);
        surface=mix(surface,u.atmosphere.xyz,u.cloud_levels.z);
        let sunlight=dot(vec3(xy,nz),u.light.xyz);
        let illumination=0.006+0.994*smoothstep(-0.08,0.8,sunlight)*u.light.w;
        let atmosphere=pow(1.-nz,3.)*smoothstep(-0.12,0.5,sunlight)*u.cloud_levels.w*u.light.w;
        let darkness=1.-smoothstep(-0.12,0.08,sunlight)*u.light.w;
        let emission=values.z*(1.-cloud_opacity*0.85)+values.w*u.night_levels.y*(0.15+cloud_opacity*0.85);
        colour=surface*illumination+u.atmosphere.xyz*atmosphere+u.night.xyz*(emission*u.night_levels.x*darkness);
    }
    var out:FragmentOut;out.colour=output_colour(clamp(colour,vec3(0.),vec3(1.)),1.);out.depth=depth_at(nz);return out;
}
@fragment fn halo_main(in:VertexOut)->FragmentOut {
    let xy=in.normal_xy;let r=length(xy);
    if r<=1. {discard;}
    var colour:vec3<f32>;var alpha:f32;
    if u.depth.z>0.5 {
        if r>1.15 {discard;}
        let orange=vec3(1.,0.666666667,0.266666667);
        let gold=vec3(1.,0.8,0.133333333);
        alpha=0.15;colour=orange*alpha;
        if r<=1.06 {colour=gold*0.3+colour*0.7;alpha=0.3+alpha*0.7;}
    } else {
        if r>1.+u.atmosphere.w {discard;}
        let haze=max(0.,1.-(r-1.)/u.atmosphere.w);
        alpha=haze*haze*u.night_levels.z;
        let sunlight=dot(xy,u.light.xy);
        let rim_light=0.04+0.96*smoothstep(-0.12,0.30,sunlight)*u.light.w;
        colour=u.atmosphere.xyz*alpha*rim_light;
    }
    var out:FragmentOut;out.colour=output_colour(colour,alpha);out.depth=depth_at(0.);return out;
}
