// Kept in lockstep with meridian_core::mule::MULE_PIECES by the shader layout test.
const MULE_PIECES: u32 = 34u;
struct Piece { range:vec4<u32>, low:vec4<f32>, high:vec4<f32> }
struct Triangle { a:vec4<f32>, b:vec4<f32>, c:vec4<f32>, normal:vec4<f32> }
struct Inputs {
    projection:vec4<f32>, origin:vec4<f32>, x:vec4<f32>, y:vec4<f32>, z:vec4<f32>,
    camera:vec4<f32>, banks:vec4<f32>, low:vec4<f32>, high:vec4<f32>, pieces:array<Piece,MULE_PIECES>,
}
@group(0) @binding(0) var<uniform> input:Inputs;
@group(0) @binding(1) var<storage,read> triangles:array<Triangle>;
@group(0) @binding(2) var diffuse:texture_2d_array<f32>;
@group(0) @binding(3) var repeat_sampler:sampler;
fn project(p:vec3<f32>)->vec4<f32> {
    let near=input.projection.z;let far=input.projection.w;
    return vec4<f32>(p.x*input.projection.y/input.projection.x,p.y*input.projection.y,
        p.z*far/(far-near)-near*far/(far-near),p.z);
}
fn to_view(p:vec3<f32>)->vec3<f32> {return input.origin.xyz+input.x.xyz*p.x+input.y.xyz*p.y+input.z.xyz*p.z;}
fn from_view(v:vec3<f32>)->vec3<f32> {
    // Invert the ACTUAL view basis, including f32 quaternion residual. Do not assume
    // the camera's transpose is an exact inverse or alter the World orientation.
    let x=input.x.xyz;let y=input.y.xyz;let z=input.z.xyz;
    let determinant=dot(x,cross(y,z));
    return vec3<f32>(dot(cross(y,z),v),dot(cross(z,x),v),dot(cross(x,y),v))/determinant;
}
fn linear(c:vec3<f32>)->vec3<f32> {return select(pow((c+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),c/12.92,c<=vec3<f32>(0.04045));}
fn output_color(c:vec3<f32>)->vec3<f32> {
    let encoded=select(1.055*pow(max(c,vec3<f32>(0.0)),vec3<f32>(1.0/2.4))-vec3<f32>(0.055),12.92*c,c<=vec3<f32>(0.0031308));
    return select(encoded,c,input.origin.w>0.5);
}
struct Solid {
    @builtin(position) position:vec4<f32>, @location(0) color:vec4<f32>, @location(1) uv:vec2<f32>,
    @location(2) @interpolate(flat) layer:u32, @location(3) @interpolate(flat) emissive:u32,
}
@vertex fn solid_vertex(@location(0) p:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) color:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) layer:u32, @location(5) emissive:u32)->Solid {
    var out:Solid;out.position=project(to_view(p));out.color=color;out.uv=uv;out.layer=layer;out.emissive=emissive;return out;
}
@fragment fn solid_fragment(v:Solid)->@location(0) vec4<f32> {
    // Diffuse source maps already contain restrained baked relief. Do not double-light them.
    let texel=textureSample(diffuse,repeat_sampler,v.uv,i32(v.layer));
    return vec4<f32>(output_color(texel.rgb*linear(v.color.rgb)),1.0);
}
struct Screen { @builtin(position) position:vec4<f32>, @location(0) ray:vec3<f32> }
@vertex fn union_vertex(@builtin(vertex_index) index:u32)->Screen {
    let p=array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0))[index];
    var out:Screen;out.position=vec4<f32>(p,0.0,1.0);out.ray=vec3<f32>(p.x*input.projection.x/input.projection.y,p.y/input.projection.y,1.0);return out;
}
struct Hit { t:f32, normal:vec3<f32> }
fn choose(old:Hit,t:f32,n:vec3<f32>,d:vec3<f32>,begin:f32)->Hit {
    if t>=begin && t<old.t && dot(n,d)< -0.000001 && all(abs(n)<vec3<f32>(2.0)) {return Hit(t,n);}
    return old;
}
fn sphere(o:vec3<f32>,d:vec3<f32>,center:vec3<f32>,r:f32,begin:f32,old:Hit)->Hit {
    let v=o-center;let b=dot(v,d);let perpendicular=v-d*b;let discriminant=r*r-dot(perpendicular,perpendicular);
    if discriminant<0.0 {return old;}
    let t=-b-sqrt(discriminant);let n=(o+d*t-center)/r;
    return choose(old,t,n,d,begin);
}
fn cylinder(o:vec3<f32>,d:vec3<f32>,a:vec3<f32>,b:vec3<f32>,r:f32,begin:f32,old:Hit)->Hit {
    let edge=b-a;let length2=dot(edge,edge);
    if length2<0.0000000001 {return old;}
    let v=o-a;let dv=dot(d,edge)/length2;let ov=dot(v,edge)/length2;
    let pd=d-edge*dv;let pv=v-edge*ov;
    let aa=dot(pd,pd);let bb=dot(pd,pv);
    if aa<0.00000001 {return old;}
    let center_t=-bb/aa;let perpendicular=pv+pd*center_t;
    let discriminant=r*r-dot(perpendicular,perpendicular);
    if discriminant<0.0 {return old;}
    let t=center_t-sqrt(discriminant/aa);let height=ov+t*dv;
    if height<0.0 || height>1.0 {return old;}
    let n=(v+t*d-edge*height)/r;
    return choose(old,t,n,d,begin);
}
fn in_triangle(p:vec3<f32>,a:vec3<f32>,b:vec3<f32>,c:vec3<f32>,n:vec3<f32>)->bool {
    // A tiny barycentric seam tolerance is bounded independently of the 2cm radius.
    return dot(cross(b-a,p-a),n)>= -0.00001*length(b-a) && dot(cross(c-b,p-b),n)>= -0.00001*length(c-b) && dot(cross(a-c,p-c),n)>= -0.00001*length(a-c);
}
fn rounded_triangle(o:vec3<f32>,d:vec3<f32>,face:Triangle,r:f32,begin:f32,old:Hit)->Hit {
    let a=face.a.xyz;let b=face.b.xyz;let c=face.c.xyz;
    let n=face.normal.xyz;let closing=dot(n,d);
    var result=old;
    if closing< -0.000001 {
        let t=(dot(n,a-o)+r)/closing;
        let p=o+d*t-n*r;
        if in_triangle(p,a,b,c,n) {result=choose(result,t,n,d,begin);}
    }
    result=cylinder(o,d,a,b,r,begin,result);result=cylinder(o,d,b,c,r,begin,result);result=cylinder(o,d,c,a,r,begin,result);
    result=sphere(o,d,a,r,begin,result);result=sphere(o,d,b,r,begin,result);result=sphere(o,d,c,r,begin,result);
    return result;
}
fn segment_distance2(p:vec3<f32>,a:vec3<f32>,b:vec3<f32>)->f32 {
    let e=b-a;let q=p-a-e*clamp(dot(p-a,e)/max(dot(e,e),0.0000000001),0.0,1.0);return dot(q,q);
}
fn triangle_distance2(p:vec3<f32>,face:Triangle)->f32 {
    let a=face.a.xyz;let b=face.b.xyz;let c=face.c.xyz;let n=face.normal.xyz;let h=dot(p-a,n);
    if in_triangle(p-n*h,a,b,c,n) {return h*h;}
    return min(segment_distance2(p,a,b),min(segment_distance2(p,b,c),segment_distance2(p,c,a)));
}
fn inside_piece(p:vec3<f32>,piece:Piece,strict:bool)->bool {
    let r=piece.low.w-select(0.0,0.0002,strict);
    if any(p<piece.low.xyz-vec3<f32>(r)) || any(p>piece.high.xyz+vec3<f32>(r)) {return false;}
    var inside=true;var distance2=1e30;
    for(var i=0u;i<piece.range.y;i++) {
        let face=triangles[piece.range.x+i];let n=face.normal.xyz;
        if dot(n,p-face.a.xyz)>0.0 {inside=false;}
        distance2=min(distance2,triangle_distance2(p,face));
    }
    return inside || distance2<r*r;
}
fn box_interval(o:vec3<f32>,d:vec3<f32>,low:vec3<f32>,high:vec3<f32>,limit:f32)->vec2<f32> {
    var near=0.0;var far=limit;
    for(var axis=0u;axis<3u;axis++) {
        if abs(d[axis])<0.0000001 {if o[axis]<low[axis] || o[axis]>high[axis] {return vec2<f32>(1.0,-1.0);}}
        else {let a=(low[axis]-o[axis])/d[axis];let b=(high[axis]-o[axis])/d[axis];near=max(near,min(a,b));far=min(far,max(a,b));}
    }
    return vec2<f32>(near,far);
}
struct Shimmer { @location(0) color:vec4<f32>, @builtin(frag_depth) depth:f32 }
@fragment fn union_fragment(screen:Screen)->Shimmer {
    if max(input.banks.x,input.banks.y)<=0.0 {discard;}
    let view_ray=normalize(screen.ray);let raw_direction=from_view(view_ray);
    let stretch=length(raw_direction);let d=raw_direction/stretch;let o=input.camera.xyz;
    let interval=box_interval(o,d,input.low.xyz,input.high.xyz,64.0);
    if interval.x>interval.y {discard;}
    let begin=input.projection.z/view_ray.z*stretch;
    // Camera/near-plane inside any member cannot show that member's internal exit surface.
    for(var piece=0u;piece<MULE_PIECES;piece++) {if inside_piece(o+d*begin,input.pieces[piece],false) {discard;}}
    var hit=Hit(64.0,vec3<f32>(0.0));
    for(var piece=0u;piece<MULE_PIECES;piece++) {
        let item=input.pieces[piece];let r=item.low.w;
        let clipped=box_interval(o,d,item.low.xyz-vec3<f32>(r+0.001),item.high.xyz+vec3<f32>(r+0.001),hit.t);
        if clipped.x>clipped.y {continue;}
        for(var i=0u;i<item.range.y;i++) {hit=rounded_triangle(o,d,triangles[item.range.x+i],r,begin,hit);}
    }
    if hit.t>=64.0 {discard;}
    let local=o+d*hit.t;
    // No concealed component surface contributes. There is no second hit/fallback layer.
    for(var piece=0u;piece<MULE_PIECES;piece++) {if inside_piece(local,input.pieces[piece],true) {discard;}}
    let normal=normalize(hit.normal);let grazing=pow(1.0-clamp(abs(dot(normal,d)),0.0,1.0),4.0);
    let bank=select(input.banks.y,input.banks.x,local.z>=0.0);
    let fade=clamp((64.0-hit.t)/32.0,0.0,1.0);
    let wave=0.95+0.05*sin(dot(local,vec3<f32>(0.31,0.47,0.23))+input.camera.w*0.7);
    let alpha=clamp(min(0.018,input.banks.z)*bank*fade*fade*grazing*wave,0.0,0.018);
    if alpha<=0.0 {discard;}
    let projected=project(to_view(local));
    if projected.w<=input.projection.z {discard;}
    var result:Shimmer;result.color=vec4<f32>(output_color(vec3<f32>(0.12,0.40,0.46))*alpha,alpha);result.depth=projected.z/projected.w;return result;
}
