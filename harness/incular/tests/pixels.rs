//! Pixel parity against Flutter's Windows (Skia) raster.
//!
//! Renders the scenes of the Flutter reference app
//! (`harness/flutter/lib/main.dart`) through the real WGPU renderer at each
//! device pixel ratio, writes `<scene>@<dpr>.png` to `<dir>/incular`, and
//! diffs each one against `<dir>/flutter/<scene>@<dpr>.png`. A per-scene
//! report goes to stderr and amplified diff images to `<dir>/diff`. `<dir>`
//! is `$INCULAR_PARITY_DIR/pixels`, `harness/out/pixels` by default.
//!
//! The scenes must match the Flutter app scene for scene. Runs only when
//! `INCULAR_PARITY=1`; see `harness/README.md`.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Offset, Size};
use incular_image::ImageHandle;
use incular_platform::PhysicalSize;
use incular_rendering::{GradientStop, GradientStops, LinearGradient, Path, Stroke};
use incular_text::{FontFamily, RichText, TextSpan, TextStyle, register_application_font};
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Align, Alignment, BackdropFilter, Border, BorderRadius, BoxDecoration, BoxShadow, BoxShape,
    ClipRRect, ClipRSuperellipse, ClipRect, ColorFilter, ColorFiltered, ColoredBox, Container,
    DecorationImage, Directionality, Image, ImageFiltered, ListView, Opacity, OverlayPortal,
    PathView, PhysicalModel, PhysicalShape, Positioned, RawScrollbar, RawScrollbarStyle,
    ScrollController, SizeTransition, SizedBox, Stack, Text, TextAlign, TextDirection, Widget,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const SIDE: f32 = 160.;
const RATIOS: [f64; 4] = [1.0, 1.25, 1.5, 2.0];

fn rgb(value: u32) -> Color {
    Color::rgba(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
        (value >> 24) as u8,
    )
}

fn at(left: f32, top: f32, width: f32, height: f32, child: impl Into<Widget>) -> Widget {
    Positioned::new(child)
        .left(left)
        .top(top)
        .width(width)
        .height(height)
        .into()
}

fn canvas(background: Color, children: Vec<Widget>) -> Widget {
    Container::new()
        .width(SIDE)
        .height(SIDE)
        .color(background)
        .child(Stack::new(children))
        .into()
}

fn inter(size: f32) -> TextStyle {
    TextStyle::new()
        .family(FontFamily::named("Inter"))
        .size(size)
        .color(Color::WHITE)
}

fn text(content: &str, size: f32) -> Widget {
    Text::new(content)
        .style(inter(size))
        .soft_wrap(false)
        .into()
}

fn colored_text(content: &str, size: f32, color: Color) -> Widget {
    Text::new(content)
        .style(inter(size).color(color))
        .soft_wrap(false)
        .into()
}

fn text_lines(background: Color, color: Color) -> Widget {
    canvas(
        background,
        vec![
            at(
                8.,
                8.,
                152.,
                20.,
                colored_text("Hamburgefonstiv 0123", 14., color),
            ),
            at(
                8.,
                40.,
                152.,
                30.,
                colored_text("Quick brown fox", 16., color),
            ),
            at(8., 80., 152., 40., colored_text("Ag 0123", 24., color)),
        ],
    )
}

fn solid(color: Color) -> Widget {
    Container::new().color(color).into()
}

fn shape_paths() -> Widget {
    let mut triangle = Path::builder();
    triangle
        .move_to(Offset::new(20., 20.))
        .line_to(Offset::new(140., 30.))
        .line_to(Offset::new(40., 140.))
        .close();
    let mut line = Path::builder();
    line.move_to(Offset::new(10., 150.))
        .line_to(Offset::new(150., 100.));
    Stack::new([
        Widget::from(
            PathView::new(triangle.build())
                .fill(Color::WHITE)
                .size(Size::new(SIDE, SIDE)),
        ),
        PathView::new(line.build())
            .stroke(
                Color::WHITE,
                Stroke {
                    width: 2.,
                    ..Stroke::default()
                },
            )
            .size(Size::new(SIDE, SIDE))
            .into(),
    ])
    .into()
}

/// A named scene, matching one in the Flutter reference app.
type Scene = (&'static str, fn() -> Widget);

fn scenes() -> Vec<Scene> {
    vec![
        ("image_default_fit", || {
            let fixture =
                ImageHandle::from_rgba8(40, 20, vec![255; 40 * 20 * 4]).expect("image fixture");
            canvas(
                Color::BLACK,
                vec![at(
                    30.,
                    30.,
                    100.,
                    100.,
                    Image::new(fixture).width(100.).height(100.),
                )],
            )
        }),
        ("decoration_image_default_fit", || {
            let fixture =
                ImageHandle::from_rgba8(40, 20, vec![255; 40 * 20 * 4]).expect("image fixture");
            canvas(
                Color::BLACK,
                vec![at(
                    30.,
                    30.,
                    100.,
                    100.,
                    Container::new()
                        .decoration(BoxDecoration::new().image(DecorationImage::new(fixture))),
                )],
            )
        }),
        // Box-relative gradient on a box away from the origin; the Flutter
        // twin uses Alignment.topLeft -> bottomRight on the same 100x100 box.
        ("linear_gradient_offset_box", || {
            let stops = GradientStops::new(vec![
                GradientStop {
                    offset: 0.,
                    color: rgb(0xFFFF0000),
                },
                GradientStop {
                    offset: 1.,
                    color: rgb(0xFF0000FF),
                },
            ]);
            canvas(
                Color::BLACK,
                vec![at(
                    30.,
                    30.,
                    100.,
                    100.,
                    Container::new().decoration(BoxDecoration::new().gradient(
                        LinearGradient::new(Offset::ZERO, Offset::new(100., 100.), stops),
                    )),
                )],
            )
        }),
        // Static RawScrollbar thumb at rest with thumb visibility on.
        ("raw_scrollbar_thumb", || {
            let controller = ScrollController::new();
            let mut scrollbar = RawScrollbar::new(controller.clone());
            scrollbar.set_style(RawScrollbarStyle {
                thumb_visibility: Some(true),
                thumb_color: Color::WHITE,
                thickness: 6.,
                ..RawScrollbarStyle::default()
            });
            let list = ListView::builder(20, |index| {
                ColoredBox::new(
                    if index % 2 == 0 {
                        rgb(0xFF303030)
                    } else {
                        rgb(0xFF505050)
                    },
                    SizedBox::shrink(),
                )
            })
            .item_extent(40.)
            .controller(controller);
            canvas(
                Color::BLACK,
                vec![at(0., 0., SIDE, SIDE, scrollbar.into_widget(list))],
            )
        }),
        // Line placement per text alignment; the last row wraps so justify
        // has a non-final line.
        ("text_align", || {
            let aligned = |content: &str, align: TextAlign| -> Widget {
                Text::new(content).style(inter(14.)).align(align).into()
            };
            canvas(
                Color::BLACK,
                vec![
                    at(8., 8., 144., 20., aligned("ab", TextAlign::Start)),
                    at(8., 32., 144., 20., aligned("ab", TextAlign::Center)),
                    at(8., 56., 144., 20., aligned("ab", TextAlign::End)),
                    at(
                        8.,
                        80.,
                        144.,
                        72.,
                        aligned("aa bb cc dd ee ff gg hh ii jj kk", TextAlign::Justify),
                    ),
                ],
            )
        }),
        ("circle", || {
            canvas(
                Color::BLACK,
                vec![at(
                    20.,
                    20.,
                    120.,
                    120.,
                    Container::new().decoration(
                        BoxDecoration::new()
                            .color(Color::WHITE)
                            .shape(BoxShape::Circle),
                    ),
                )],
            )
        }),
        ("rrect", || {
            canvas(
                Color::BLACK,
                vec![at(
                    20.,
                    40.,
                    120.,
                    80.,
                    Container::new().decoration(
                        BoxDecoration::new()
                            .color(Color::WHITE)
                            .border_radius(BorderRadius::circular(24.)),
                    ),
                )],
            )
        }),
        ("path", || canvas(Color::BLACK, vec![shape_paths()])),
        ("text", || {
            canvas(
                Color::BLACK,
                vec![
                    at(8., 8., 152., 20., text("Hamburgefonstiv 0123", 14.)),
                    at(8., 40., 152., 30., text("Quick brown fox", 16.)),
                    at(8., 80., 152., 40., text("Ag 0123", 24.)),
                    at(8., 124., 152., 30., text("Wavy 12.5", 12.5)),
                ],
            )
        }),
        ("rich_text_spans", || {
            canvas(
                Color::BLACK,
                vec![at(
                    8.,
                    30.,
                    144.,
                    40.,
                    RichText::new(
                        TextSpan::empty()
                            .style(inter(20.))
                            .child(
                                TextSpan::new("RED ")
                                    .style(TextStyle::new().color(rgb(0xFFFF0000))),
                            )
                            .child(
                                TextSpan::new("BLUE")
                                    .style(TextStyle::new().color(rgb(0xFF0000FF))),
                            ),
                    )
                    .soft_wrap(false),
                )],
            )
        }),
        ("size_transition_clip", || {
            canvas(
                Color::BLACK,
                vec![
                    Positioned::new(SizeTransition::new(
                        0.5,
                        SizedBox::new()
                            .width(120.)
                            .height(100.)
                            .child(ColoredBox::new(Color::WHITE, SizedBox::shrink())),
                    ))
                    .left(20.)
                    .top(20.)
                    .into(),
                ],
            )
        }),
        ("physical_shape_triangle", || {
            let mut path = Path::builder();
            path.move_to(Offset::ZERO)
                .line_to(Offset::new(120., 0.))
                .line_to(Offset::new(60., 120.))
                .close();
            canvas(
                Color::BLACK,
                vec![at(
                    20.,
                    20.,
                    120.,
                    120.,
                    PhysicalShape::new(
                        Arc::new(path.build()),
                        Color::WHITE,
                        Align::new(
                            Alignment::TOP_LEFT,
                            SizedBox::new()
                                .width(40.)
                                .height(40.)
                                .child(solid(rgb(0xFFFF0000))),
                        ),
                    )
                    .elevation(8.),
                )],
            )
        }),
        ("physical_model_circle", || {
            canvas(
                Color::BLACK,
                vec![at(
                    20.,
                    20.,
                    120.,
                    120.,
                    PhysicalModel::new(
                        rgb(0xFF2196F3),
                        Align::new(
                            Alignment::CENTER,
                            SizedBox::new()
                                .width(60.)
                                .height(60.)
                                .child(solid(rgb(0xFFF44336))),
                        ),
                    ),
                )],
            )
        }),
        ("overlay_portal_ancestor_clip", || {
            let popup = Stack::new([Widget::from(
                Positioned::new(
                    SizedBox::new()
                        .width(40.)
                        .height(40.)
                        .child(solid(rgb(0xFFFF9800))),
                )
                .left(110.)
                .top(110.),
            )]);
            let portal = OverlayPortal::new(solid(rgb(0xFF2196F3)))
                .overlay_child_layout_builder(move |_, _| popup.clone().into())
                .show(true);
            canvas(
                Color::BLACK,
                vec![at(
                    0.,
                    0.,
                    80.,
                    80.,
                    ClipRect::new(SizedBox::new().width(80.).height(80.).child(portal)),
                )],
            )
        }),
        ("overlay_portal_no_clip", || {
            let popup = Stack::new([Widget::from(
                Positioned::new(
                    SizedBox::new()
                        .width(40.)
                        .height(40.)
                        .child(solid(rgb(0xFFFF9800))),
                )
                .left(110.)
                .top(110.),
            )]);
            let portal = OverlayPortal::new(solid(rgb(0xFF2196F3)))
                .overlay_child_layout_builder(move |_, _| popup.clone().into())
                .show(true);
            canvas(
                Color::BLACK,
                vec![at(
                    0.,
                    0.,
                    80.,
                    80.,
                    SizedBox::new().width(80.).height(80.).child(portal),
                )],
            )
        }),
        ("clip_r_superellipse", || {
            canvas(
                Color::BLACK,
                vec![at(
                    20.,
                    40.,
                    120.,
                    80.,
                    ClipRSuperellipse::new(
                        32.,
                        SizedBox::new()
                            .width(120.)
                            .height(80.)
                            .child(solid(Color::WHITE)),
                    ),
                )],
            )
        }),
        ("color_filter_matrix_bias", || {
            canvas(
                Color::BLACK,
                vec![at(
                    40.,
                    40.,
                    80.,
                    80.,
                    ColorFiltered::new(
                        ColorFilter::matrix([
                            -1., 0., 0., 0., 255., 0., -1., 0., 0., 255., 0., 0., -1., 0., 255.,
                            0., 0., 0., 1., 0.,
                        ]),
                        solid(Color::WHITE),
                    ),
                )],
            )
        }),
        ("text_dark", || text_lines(Color::WHITE, Color::BLACK)),
        ("text_gray", || text_lines(Color::BLACK, rgb(0xFF808080))),
        ("shadow", || {
            canvas(
                Color::WHITE,
                vec![at(
                    40.,
                    40.,
                    80.,
                    80.,
                    Container::new().decoration(
                        BoxDecoration::new()
                            .color(rgb(0xFF2196F3))
                            .border_radius(BorderRadius::circular(8.))
                            .box_shadow([BoxShadow::new(
                                rgb(0x8A000000),
                                Offset::new(0., 4.),
                                16.,
                                0.,
                            )]),
                    ),
                )],
            )
        }),
        ("blur", || {
            ImageFiltered::blur(
                4.,
                canvas(
                    Color::BLACK,
                    vec![at(50., 50., 60., 60., solid(Color::WHITE))],
                ),
            )
            .into()
        }),
        ("blend", || {
            canvas(
                Color::BLACK,
                vec![
                    at(20., 20., 80., 80., solid(rgb(0xFFFF0000))),
                    at(
                        60.,
                        60.,
                        80.,
                        80.,
                        Opacity::new(0.5, solid(rgb(0xFF0000FF))),
                    ),
                    at(40., 90., 100., 40., solid(rgb(0x80FFFFFF))),
                    at(100., 10., 50., 50., solid(rgb(0x4000FF00))),
                ],
            )
        }),
        ("hairline", || {
            canvas(
                Color::BLACK,
                vec![
                    at(
                        10.3,
                        10.7,
                        50.5,
                        30.25,
                        Container::new().border(Border::new(1., Color::WHITE)),
                    ),
                    at(80.5, 20.25, 40., 40., solid(Color::WHITE)),
                    at(10.25, 70.5, 140., 0.5, solid(Color::WHITE)),
                    at(10., 90.4, 140., 1., solid(Color::WHITE)),
                    at(20.6, 110., 0.75, 40., solid(Color::WHITE)),
                ],
            )
        }),
        // A backdrop blur clipped to a rounded rectangle: the corners
        // outside the rounded shape keep the sharp stripes.
        ("backdrop_rrect", || {
            let mut children: Vec<Widget> = (0..4)
                .map(|index| at(10. + 40. * index as f32, 0., 20., 160., solid(Color::WHITE)))
                .collect();
            children.push(at(
                20.,
                20.,
                120.,
                120.,
                ClipRRect::new(
                    BorderRadius::circular(40.),
                    BackdropFilter::blur(6., SizedBox::expand()),
                ),
            ));
            canvas(Color::BLACK, children)
        }),
    ]
}

#[derive(Default)]
struct Stats {
    max: u8,
    mean: f64,
    over_2: f64,
    over_16: f64,
}

fn compare(ours: &image::RgbaImage, theirs: &image::RgbaImage, diff_path: &FsPath) -> Stats {
    let (width, height) = ours.dimensions();
    let mut stats = Stats::default();
    let mut diff = image::RgbaImage::new(width, height);
    let mut total = 0u64;
    let (mut over_2, mut over_16) = (0u64, 0u64);
    for (x, y, pixel) in ours.enumerate_pixels() {
        let other = theirs.get_pixel(x, y);
        let delta = (0..3)
            .map(|channel| pixel[channel].abs_diff(other[channel]))
            .max()
            .unwrap_or(0);
        stats.max = stats.max.max(delta);
        total += u64::from(delta);
        over_2 += u64::from(delta > 2);
        over_16 += u64::from(delta > 16);
        let shown = delta.saturating_mul(8);
        // Red: Incular brighter, cyan: Flutter brighter.
        let brighter = pixel[0] as i32 + pixel[1] as i32 + pixel[2] as i32
            >= other[0] as i32 + other[1] as i32 + other[2] as i32;
        *diff.get_pixel_mut(x, y) = if brighter {
            image::Rgba([shown, 0, 0, 255])
        } else {
            image::Rgba([0, shown, shown, 255])
        };
    }
    let count = f64::from(width * height);
    stats.mean = total as f64 / count;
    stats.over_2 = over_2 as f64 * 100. / count;
    stats.over_16 = over_16 as f64 * 100. / count;
    diff.save(diff_path).expect("write diff image");
    stats
}

/// `INCULAR_PARITY_PROBE="blend@1.0.png:20,20;21,20"` prints both pixels.
fn probe(file: &str, ours: &image::RgbaImage, theirs: &image::RgbaImage) {
    let Ok(spec) = std::env::var("INCULAR_PARITY_PROBE") else {
        return;
    };
    for entry in spec.split('|') {
        let Some((target, points)) = entry.split_once(':') else {
            continue;
        };
        if target != file {
            continue;
        }
        for point in points.split(';') {
            let Some((x, y)) = point.split_once(',') else {
                continue;
            };
            let (Ok(x), Ok(y)) = (x.trim().parse::<u32>(), y.trim().parse::<u32>()) else {
                continue;
            };
            eprintln!(
                "probe {file} ({x},{y}) incular {:?} flutter {:?}",
                ours.get_pixel(x, y).0,
                theirs.get_pixel(x, y).0
            );
        }
    }
}

/// `INCULAR_PARITY_BANDS="text@1.0.png:8-28;40-62"` prints, for each row
/// band, total ink and its centroid in both images (white-on-black ink).
fn bands(file: &str, ours: &image::RgbaImage, theirs: &image::RgbaImage) {
    let Ok(spec) = std::env::var("INCULAR_PARITY_BANDS") else {
        return;
    };
    let ink = |img: &image::RgbaImage, top: u32, bottom: u32| {
        let (mut sum, mut sx, mut sy) = (0f64, 0f64, 0f64);
        for y in top..bottom.min(img.height()) {
            for x in 0..img.width() {
                let value = f64::from(img.get_pixel(x, y)[0]) / 255.;
                sum += value;
                sx += value * f64::from(x);
                sy += value * f64::from(y);
            }
        }
        let (mut left, mut right) = (u32::MAX, 0);
        for y in top..bottom.min(img.height()) {
            for x in 0..img.width() {
                if img.get_pixel(x, y)[0] > 24 {
                    left = left.min(x);
                    right = right.max(x);
                }
            }
        }
        (sum, sx / sum.max(1e-9), sy / sum.max(1e-9), left, right)
    };
    for entry in spec.split('|') {
        let Some((target, ranges)) = entry.split_once(':') else {
            continue;
        };
        if target != file {
            continue;
        }
        for range in ranges.split(';') {
            let Some((top, bottom)) = range.split_once('-') else {
                continue;
            };
            let (Ok(top), Ok(bottom)) = (top.parse::<u32>(), bottom.parse::<u32>()) else {
                continue;
            };
            let a = ink(ours, top, bottom);
            let b = ink(theirs, top, bottom);
            eprintln!(
                "band {file} {top}-{bottom}: ink {:.1} vs {:.1} ({:+.1}%)  cx {:.3} vs {:.3}  cy {:.3} vs {:.3}  x {}..{} vs {}..{}",
                a.0,
                b.0,
                (a.0 / b.0.max(1e-9) - 1.) * 100.,
                a.1,
                b.1,
                a.2,
                b.2,
                a.3,
                a.4,
                b.3,
                b.4
            );
        }
    }
}

/// `INCULAR_PARITY_TONE="text@1.0.png"` prints the histogram-matched tone
/// curve: for each of our coverage levels, Flutter's value at that quantile.
fn tone(file: &str, ours: &image::RgbaImage, theirs: &image::RgbaImage) {
    let Ok(spec) = std::env::var("INCULAR_PARITY_TONE") else {
        return;
    };
    if !spec.split('|').any(|target| target == file) {
        return;
    }
    let mut a: Vec<u8> = ours.pixels().map(|p| p[0]).collect();
    let mut b: Vec<u8> = theirs.pixels().map(|p| p[0]).collect();
    a.sort_unstable();
    b.sort_unstable();
    let mut line = format!("tone {file}:");
    for level in (8..=248).step_by(16) {
        // Middle quantile of our pixels at this level.
        let lo = a.partition_point(|v| *v < level as u8);
        let hi = a.partition_point(|v| *v <= level as u8 + 7);
        if hi <= lo {
            continue;
        }
        let mid = (lo + hi) / 2;
        line.push_str(&format!(" {level}->{}", b[mid]));
    }
    let mut levels = b.clone();
    levels.dedup();
    eprintln!(
        "{line}
  flutter levels {levels:?}"
    );
}

struct Capture {
    dir: PathBuf,
    report: Vec<String>,
    failed: bool,
}

impl Capture {
    fn run(&mut self, event_loop: &ActiveEventLoop) {
        for ratio in RATIOS {
            let physical = (f64::from(SIDE) * ratio).round() as u32;
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Incular parity")
                            .with_decorations(false)
                            .with_resizable(false)
                            .with_inner_size(winit::dpi::PhysicalSize::new(physical, physical)),
                    )
                    .expect("create window"),
            );
            for (name, build) in scenes() {
                // A renderer per scene: retained effect caches are keyed by
                // layer id and generation, which restart in every new tree.
                let mut renderer = pollster::block_on(WgpuRenderer::new(
                    WindowSurfaceTarget::new(window.clone()),
                    PhysicalSize::new(physical, physical),
                    TransparencyMode::Opaque,
                    Color::BLACK,
                ))
                .expect("create renderer");
                let mut tree = WidgetTree::new();
                tree.mount(Directionality::new(TextDirection::Ltr, build()).into())
                    .expect("mount scene");
                // Two passes, as two runtime frames: builders that run during
                // layout attach their compositor layers on the next pass.
                for _ in 0..2 {
                    tree.layout(Constraints::tight(Size::new(SIDE, SIDE)))
                        .expect("layout scene");
                    tree.update_compositor(incular_core::time::Instant::now())
                        .expect("composite scene");
                }
                let list = tree.paint();
                renderer.request_capture();
                match renderer.render(&list, ratio) {
                    Ok(FrameOutcome::Presented(_)) => {}
                    other => panic!("{name}@{ratio}: frame not presented: {other:?}"),
                }
                let frame = renderer
                    .take_capture()
                    .expect("capture requested")
                    .expect("capture succeeded");
                let ours = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba8)
                    .expect("capture buffer");
                let file = format!("{name}@{ratio:?}.png");
                ours.save(self.dir.join("incular").join(&file))
                    .expect("write capture");
                let reference = self.dir.join("flutter").join(&file);
                let Ok(theirs) = image::open(&reference) else {
                    self.report
                        .push(format!("{file:<20} missing Flutter reference"));
                    self.failed = true;
                    continue;
                };
                let theirs = theirs.to_rgba8();
                if theirs.dimensions() != ours.dimensions() {
                    self.report.push(format!(
                        "{file:<20} size {:?} vs Flutter {:?}",
                        ours.dimensions(),
                        theirs.dimensions()
                    ));
                    self.failed = true;
                    continue;
                }
                let stats = compare(&ours, &theirs, &self.dir.join("diff").join(&file));
                probe(&file, &ours, &theirs);
                bands(&file, &ours, &theirs);
                tone(&file, &ours, &theirs);
                self.report.push(format!(
                    "{file:<20} max {:>3}  mean {:>6.3}  >2: {:>6.2}%  >16: {:>6.2}%",
                    stats.max, stats.mean, stats.over_2, stats.over_16
                ));
            }
        }
    }
}

impl ApplicationHandler for Capture {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.run(event_loop);
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() {
    if std::env::var("INCULAR_PARITY").as_deref() != Ok("1") {
        eprintln!("pixels: skipped; set INCULAR_PARITY=1 (see harness/README.md)");
        return;
    }
    let dir = incular_parity_harness::output_dir().join("pixels");
    for sub in ["incular", "diff"] {
        std::fs::create_dir_all(dir.join(sub)).expect("create output dir");
    }
    let font = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../flutter/fonts/Inter-Regular.ttf"
    ))
    .expect("read Inter");
    register_application_font(font);
    let mut capture = Capture {
        dir,
        report: Vec::new(),
        failed: false,
    };
    EventLoop::new()
        .expect("event loop")
        .run_app(&mut capture)
        .expect("run event loop");
    for line in &capture.report {
        eprintln!("{line}");
    }
    assert!(!capture.failed, "parity capture had missing references");
}
