# Paired slice: border hairlines and path paint

Flutter 3.50.0 at `ab2912035e` and Incular framework at `c647cb85`, audit
HEAD `4bddba53`. Flutter used PictureRecorder/toImage; Incular used
WgpuRenderer::render, a hidden Windows surface, Presented frames and actual
RGBA readback. This was not an isolated shader test. Every result below
repeated at DPR 1 and 2, counted once per distinct contract.

| ID | Scenario | Flutter actual | Incular actual |
| --- | --- | --- | --- |
| C11 | Solid, red width-zero rectangular Border over white | White center; edge red coverage 1 physical px | Red center (filled rect); edge probe coverage 3 |
| C12 | Red width-zero stroked rectangular Path over white | White center; edge red coverage 1 physical px | White center; measured edge coverage 0, stroke absent |
| C13 | Opaque red path behind opaque blue (`DstOver`) | Blue center | Red center |
| C14 | Opaque red path tinted green by `SrcIn` color filter | Green center | Red center |
| Control | Ordinary red path over blue | Red center | Red center |

For C11/C12 the sum of red coverage along the top edge (sampled at x=32
logical) is `sum((255-green)/255)` across five physical rows; Flutter
showed green bytes [255,128,127,255,255] in each. The two Incular routes
are independent: the analytic border instance passes zero to WGSL (a
fill), and tessellation drops nonpositive stroke widths. Source:
`crates/incular-wgpu/src/render_plan.rs:878–899`,
`shaders/square_rect_shader.wgsl:23`, `geometry.rs:65–75,168–173`;
Flutter `painting/box_border.dart:358–360`, `painting/borders.dart:224–235`.

C13/C14 use separate scenes so they don't cancel or mask each other.
`Canvas::draw_path` drops blend and color-filter metadata on lowering
(`crates/incular-rendering/src/display_list.rs:625–637`); Flutter passes
full Paint (engine `lib/ui/painting.dart:8076–8081`). Test center pixels
well away from antialiasing. Recorded scope: Windows wgpu and Flutter's
headless renderer; other backends, rounded/rotated/fractional-DPR hairlines,
clipping and all blend/filter combinations remain open.

## Focused reproduction

Use the pinned Flutter scratch package from [paired comparisons](focused-comparisons.md).
Save the following as `test/raster_slice_test.dart` and run
`flutter test --no-pub test/raster_slice_test.dart --reporter expanded`.
The Flutter test passed all assertions.

```dart
import 'dart:ui' as ui;
import 'package:flutter/painting.dart' show Border;
import 'package:flutter_test/flutter_test.dart';

const white=ui.Color(0xffffffff), red=ui.Color(0xffff0000),
    blue=ui.Color(0xff0000ff), green=ui.Color(0xff00ff00);
const box=ui.Rect.fromLTWH(8,8,48,48);
void main(){
  test('neutral raster audit',( ) async{
    for(final dpr in [1,2]){
      for(final kind in ['border','stroke','ordinary','blend','filter']){
        final recorder=ui.PictureRecorder();
        final c=ui.Canvas(recorder)..scale(dpr.toDouble());
        final isBorder=kind=='border'||kind=='stroke';
        c.drawColor(isBorder?white:blue,ui.BlendMode.src);
        final p=ui.Path()..addRect(box);
        if(kind=='border'){
          Border.all(color:red,width:0).paint(c,box);
        } else {
          final paint=ui.Paint()..color=red;
          if(kind=='stroke'){paint.style=ui.PaintingStyle.stroke;paint.strokeWidth=0;}
          if(kind=='blend'){paint.blendMode=ui.BlendMode.dstOver;}
          if(kind=='filter'){paint.colorFilter=const ui.ColorFilter.mode(green,ui.BlendMode.srcIn);}
          c.drawPath(p,paint);
        }
        final picture=recorder.endRecording();
        final image=await picture.toImage(64*dpr,64*dpr);
        final bytes=(await image.toByteData(format:ui.ImageByteFormat.rawRgba))!;
        List<int> pixel(int x,int y)=>List.generate(4,(i)=>bytes.getUint8((y*image.width+x)*4+i));
        final center=pixel(32*dpr,32*dpr);
        final edge=[for(var y=8*dpr-2;y<=8*dpr+2;y++) pixel(32*dpr,y)[1]];
        print('$kind $dpr center=$center edgeGreen=$edge');
        expect(center,
          isBorder?[255,255,255,255]:kind=='blend'?[0,0,255,255]:kind=='filter'?[0,255,0,255]:[255,0,0,255]);
        if(isBorder){
          expect(pixel(2*dpr,2*dpr),[255,255,255,255]);
          expect(edge.map((g)=>(255-g)/255).reduce((a,b)=>a+b),closeTo(1,0.15));
        }
        image.dispose();
        picture.dispose();
      }
    }
  });
}
```

Save the Rust probe only as a temporary target
`crates/incular-desktop/tests/audit_raster_slice_tmp.rs`. Use
`cargo test --locked -p incular-desktop --test audit_raster_slice_tmp raster_behavior_probe -- --exact --nocapture`.
The probe printed all ten samples, then failed with the expected mismatches.
Do not add the known-red temporary target to the normal suite.

```rust
#![cfg(target_os = "windows")]
use incular_config::TransparencyMode;
use incular_core::{Color, Offset, Rect, Size};
use incular_platform::PhysicalSize;
use incular_rendering::{BlendMode, Border, Canvas, ColorFilter, FillRule, Paint, Path, RRect, Stroke};
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use std::sync::Arc;
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

#[test]
#[allow(deprecated)]
fn raster_behavior_probe() {
    let events=EventLoop::builder().with_any_thread(true).build().unwrap();
    let window=events.create_window(Window::default_attributes().with_visible(false)
        .with_inner_size(winit::dpi::PhysicalSize::new(128,128))).unwrap();
    let mut renderer=pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)), PhysicalSize::new(128,128),
        TransparencyMode::Opaque,Color::WHITE)).unwrap();
    let red=Color::rgba(255,0,0,255);
    let green=Color::rgba(0,255,0,255);
    let blue=Color::rgba(0,0,255,255);
    let bounds=Rect::from_origin_size(Offset::ZERO,Size::new(128.,128.));
    let rect=Rect::from_origin_size(Offset::new(8.,8.),Size::new(48.,48.));
    let mut path=Path::builder();
    path.move_to(Offset::new(8.,8.)).line_to(Offset::new(56.,8.)).line_to(Offset::new(56.,56.))
        .line_to(Offset::new(8.,56.)).close();
    let path=Arc::new(path.build());
    let mut mismatches=Vec::new();
    for dpr in [1u32,2] {
        for kind in ["border","stroke","ordinary","blend","filter"] {
            let mut canvas=Canvas::default();
            let is_border=matches!(kind,"border"|"stroke");
            canvas.rect(bounds,if is_border {Color::WHITE} else {blue});
            match kind {
                "border"=>canvas.border(RRect::uniform(rect,0.),Border::new(0.,red)),
                "stroke"=>canvas.stroke_path(path.clone(),red,Stroke{width:0.,..Default::default()}),
                _=>{
                    let mut paint=Paint::new().color(red);
                    if kind=="blend" {paint=paint.blend_mode(BlendMode::DstOver);}
                    if kind=="filter" {paint=paint.color_filter(Some(ColorFilter::blend(green,BlendMode::SrcIn)));}
                    canvas.draw_path(path.clone(),&paint,FillRule::NonZero);
                }
            }
            renderer.request_capture();
            assert!(matches!(renderer.render(&canvas.finish(),f64::from(dpr)).unwrap(),FrameOutcome::Presented(_)));
            let frame=renderer.take_capture().unwrap().unwrap();
            let pixel=|x:u32,y:u32| -> [u8;4] {
                let i=((y*frame.width+x)*4) as usize;
                frame.rgba8[i..i+4].try_into().unwrap()
            };
            let center=pixel(32*dpr,32*dpr);
            let edge:Vec<_>=(8*dpr-2..=8*dpr+2).map(|y|pixel(32*dpr,y)[1]).collect();
            let expected=match kind {"border"|"stroke"=>[255,255,255,255],
                "blend"=>[0,0,255,255],"filter"=>[0,255,0,255],_=>[255,0,0,255]};
            println!("{kind} {dpr} center={center:?} edgeGreen={edge:?}");
            if center!=expected {mismatches.push(format!("{kind}@{dpr}: {center:?} expected {expected:?}"));}
            if is_border {
                assert_eq!(pixel(2*dpr,2*dpr),[255,255,255,255]);
                let coverage=edge.iter().map(|&g|(255-g) as f32/255.).sum::<f32>();
                if (coverage-1.).abs()>0.15 {mismatches.push(format!("{kind}@{dpr} edge coverage {coverage}"));}
            }
        }
    }
    assert!(mismatches.is_empty(),"{mismatches:?}");
}
```
