
struct Out { @builtin(position) position: vec4<f32>, @location(0) local: vec2<f32>, @location(1) color: vec4<f32>, @location(2) gradient: vec4<f32>, @location(3) options: vec4<f32>, @location(4) gradient_ext: vec4<f32> };
@group(0) @binding(0) var gradient_lut: texture_2d<f32>;
@group(0) @binding(1) var gradient_sampler: sampler;
@vertex fn vs_main(@location(0) local: vec2<f32>, @location(1) affine: vec4<f32>, @location(2) translation: vec4<f32>, @location(3) surface: vec4<f32>, @location(4) color: vec4<f32>, @location(5) gradient: vec4<f32>, @location(6) options: vec4<f32>, @location(7) gradient_ext: vec4<f32>) -> Out {
  var out: Out;
  let physical = vec2<f32>(
    affine.x * local.x + affine.z * local.y + translation.x,
    affine.y * local.x + affine.w * local.y + translation.y,
  );
  out.position = vec4<f32>(physical.x / surface.x * 2. - 1., 1. - physical.y / surface.y * 2., 0., 1.);
  out.color = color;
  out.local = local;
  out.gradient = gradient;
  out.options = options;
  out.gradient_ext = gradient_ext;
  return out;
}
fn lookup(t: f32) -> vec4<f32> { let p=textureSampleLevel(gradient_lut,gradient_sampler,vec2(clamp(t,0.,1.),.5),0.); return select(vec4(0.),vec4(p.rgb/max(p.a,.00001),p.a),p.a>0.); }
fn tile(t: f32, mode: f32) -> f32 { if (mode == 1.) { return fract(t); } if (mode == 2.) { let p = fract(t * .5); return 1. - abs(p * 2. - 1.); } return clamp(t, 0., 1.); }
fn conical(p: vec2<f32>, f: vec2<f32>, fr: f32, c: vec2<f32>, r: f32) -> vec2<f32> { let cd=c-f; let dr=r-fr; let pd=p-f; let a=dot(cd,cd)-dr*dr; let b=dot(pd,cd)+fr*dr; let cc=dot(pd,pd)-fr*fr; if (abs(a) <= 1.1920929e-7) { if (b == 0.) { return vec2(0.,0.); } let t=cc/(2.*b); return vec2(t, select(0.,1.,fr+t*dr>=0.)); } let disc=b*b-a*cc; if (disc < 0.) { return vec2(0.,0.); } let root=sqrt(disc); let hi=max((b+root)/a,(b-root)/a); let lo=min((b+root)/a,(b-root)/a); if (fr+hi*dr >= 0.) { return vec2(hi,1.); } if (fr+lo*dr >= 0.) { return vec2(lo,1.); } return vec2(0.,0.); }
fn gradient_sample(p: vec2<f32>, kind: f32, g: vec4<f32>, ext: vec4<f32>) -> vec4<f32> { var t=0.; var valid=true; if(kind==1.) { let d=g.zw-g.xy; t=dot(p-g.xy,d)/max(dot(d,d),.0001); } else if(kind==2.) { t=length(p-g.xy)/max(g.z,.0001); } else if(kind==3.) { t=fract((atan2(p.y-g.y,p.x-g.x)-g.z)/6.2831853); } else if(kind==4.) { let r=conical(p,ext.yz,ext.w,g.xy,g.z); t=r.x; valid=r.y>0.; } if(kind!=3.) { valid=valid && !(ext.x==3. && (t<0. || t>1.)); t=tile(t,ext.x); } return select(vec4(0.),lookup(t),valid); }
@fragment fn fs_main(i: Out) -> @location(0) vec4<f32> { return select(i.color,gradient_sample(i.local,i.options.x,i.gradient,i.gradient_ext),i.options.x>0.); }
