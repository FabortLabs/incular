//
// Nonseparable blends use weighted luminosity and a channel range; a
// per-channel operation cannot preserve the other color's attributes.
fn luminosity(c: vec3<f32>) -> f32 {
  return dot(c, vec3<f32>(.3, .59, .11));
}
fn channel_range(c: vec3<f32>) -> vec2<f32> {
  return vec2<f32>(min(c.r, min(c.g, c.b)), max(c.r, max(c.g, c.b)));
}
fn saturation(c: vec3<f32>) -> f32 {
  let range = channel_range(c);
  return range.y - range.x;
}
fn with_saturation(c: vec3<f32>, sat: f32) -> vec3<f32> {
  let range = channel_range(c);
  if (range.x == range.y) { return vec3<f32>(0.); }
  return (c - range.x) * (sat / (range.y - range.x));
}
fn with_luminosity(c: vec3<f32>, lum: f32) -> vec3<f32> {
  var adjusted = c + (lum - luminosity(c));
  let range = channel_range(adjusted);
  if (range.x < 0.) {
    adjusted = lum + (adjusted - lum) * (lum / (lum - range.x));
  }
  if (range.y > 1.) {
    adjusted = lum + (adjusted - lum) * ((1. - lum) / (range.y - lum));
  }
  return adjusted;
}

fn artistic(mode: u32, s: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
  var out = s;
  if (mode == 11u) { out = s * d; }
  else if (mode == 12u) { out = s + d - s * d; }
  else if (mode == 13u) { out = select(2. * s * d, 1. - 2. * (1. - s) * (1. - d), d > vec3<f32>(.5)); }
  else if (mode == 14u) { out = min(s, d); }
  else if (mode == 15u) { out = max(s, d); }
  else if (mode == 16u) { out = select(min(d / max(1. - s, vec3<f32>(.000001)), vec3<f32>(1.)), vec3<f32>(1.), s >= vec3<f32>(1.)); }
  else if (mode == 17u) { out = select(max(1. - (1. - d) / max(s, vec3<f32>(.000001)), vec3<f32>(0.)), vec3<f32>(0.), s <= vec3<f32>(0.)); }
  else if (mode == 18u) { out = select(2. * s * d, 1. - 2. * (1. - s) * (1. - d), s > vec3<f32>(.5)); }
  else if (mode == 19u) {
    let low = d - (1. - 2. * s) * d * (1. - d);
    let g = select(sqrt(d), ((16. * d - 12.) * d + 4.) * d, d <= vec3<f32>(.25));
    out = select(low, d + (2. * s - 1.) * (g - d), s > vec3<f32>(.5));
  }
  else if (mode == 20u) { out = abs(d - s); }
  else if (mode == 21u) { out = s + d - 2. * s * d; }
  else if (mode == 23u) { out = with_luminosity(with_saturation(s, saturation(d)), luminosity(d)); }
  else if (mode == 24u) { out = with_luminosity(with_saturation(d, saturation(s)), luminosity(d)); }
  else if (mode == 25u) { out = with_luminosity(s, luminosity(d)); }
  else if (mode == 26u) { out = with_luminosity(d, luminosity(s)); }
  return clamp(out, vec3<f32>(0.), vec3<f32>(1.));
}
