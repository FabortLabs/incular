//! Outline icons for the DevTools workspace, drawn natively.
//!
//! Every icon is a set of strokes on a 24 x 24 grid with round caps and
//! joins (the Lucide convention), built once and shared, then scaled to the
//! requested size when painted. Icons are decorative: the control that hosts
//! one always carries its own accessible label.

use incular::core::Transform;
use incular::prelude::*;
use std::sync::{Arc, OnceLock};

/// Icon glyphs used across the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    Overview,
    Tree,
    Activity,
    Monitor,
    Accessibility,
    Layers,
    Chip,
    Terminal,
    Transfer,
    AppWindow,
    Search,
    Command,
    Keyboard,
    Sun,
    Moon,
    Refresh,
    Download,
    Crosshair,
    Eye,
    PanelLeft,
    Close,
    Copy,
    Check,
    ChevronRight,
    ChevronDown,
    Record,
    Stop,
    Trash,
    Pause,
    Play,
    Zap,
    Alert,
    Info,
    Plug,
    Filter,
    ArrowLeft,
    ArrowRight,
    Target,
    Sparkles,
}

/// A stroke-only path builder on the 24-unit grid.
struct Pen(PathBuilder);

impl Pen {
    fn new() -> Self {
        Self(Path::builder())
    }

    fn line(&mut self, points: &[(f32, f32)]) -> &mut Self {
        if let Some((first, rest)) = points.split_first() {
            self.0.move_to(Offset::new(first.0, first.1));
            for point in rest {
                self.0.line_to(Offset::new(point.0, point.1));
            }
        }
        self
    }

    fn closed(&mut self, points: &[(f32, f32)]) -> &mut Self {
        self.line(points);
        self.0.close();
        self
    }

    fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, radius: f32) -> &mut Self {
        self.0.rounded_rect(
            incular::core::Rect::from_origin_size(Offset::new(x, y), Size::new(width, height)),
            CornerRadii::uniform(radius),
        );
        self
    }

    /// A circular arc from `start` to `end` radians (clockwise, y down).
    fn arc(&mut self, cx: f32, cy: f32, radius: f32, start: f32, end: f32) -> &mut Self {
        let segments = ((end - start).abs() / std::f32::consts::FRAC_PI_2)
            .ceil()
            .max(1.) as usize;
        let step = (end - start) / segments as f32;
        let handle = 4. / 3. * (step / 4.).tan() * radius;
        let point = |angle: f32| (cx + radius * angle.cos(), cy + radius * angle.sin());
        let (x, y) = point(start);
        self.0.move_to(Offset::new(x, y));
        for index in 0..segments {
            let a0 = start + step * index as f32;
            let a1 = a0 + step;
            let (x0, y0) = point(a0);
            let (x1, y1) = point(a1);
            self.0.cubic_to(
                Offset::new(x0 - handle * a0.sin(), y0 + handle * a0.cos()),
                Offset::new(x1 + handle * a1.sin(), y1 - handle * a1.cos()),
                Offset::new(x1, y1),
            );
        }
        self
    }

    fn circle(&mut self, cx: f32, cy: f32, radius: f32) -> &mut Self {
        self.arc(cx, cy, radius, 0., std::f32::consts::TAU);
        self.0.close();
        self
    }

    fn finish(self) -> Arc<Path> {
        Arc::new(self.0.build())
    }
}

fn build(name: IconName) -> Arc<Path> {
    use std::f32::consts::PI;
    let mut p = Pen::new();
    match name {
        IconName::Overview => {
            p.rect(3., 3., 7., 9., 1.5)
                .rect(14., 3., 7., 5., 1.5)
                .rect(14., 12., 7., 9., 1.5)
                .rect(3., 16., 7., 5., 1.5);
        }
        IconName::Tree => {
            p.rect(3., 3., 6., 5., 1.)
                .line(&[(6., 8.), (6., 17.), (13., 17.)])
                .line(&[(6., 12.5), (13., 12.5)])
                .line(&[(16., 12.5), (21., 12.5)])
                .line(&[(16., 17.), (21., 17.)])
                .line(&[(12., 5.5), (21., 5.5)]);
        }
        IconName::Activity => {
            p.line(&[
                (22., 12.),
                (18., 12.),
                (15., 21.),
                (9., 3.),
                (6., 12.),
                (2., 12.),
            ]);
        }
        IconName::Monitor => {
            p.rect(2., 3., 20., 14., 2.)
                .line(&[(8., 21.), (16., 21.)])
                .line(&[(12., 17.), (12., 21.)]);
        }
        IconName::Accessibility => {
            p.circle(12., 4.5, 1.6)
                .line(&[(5., 8.5), (12., 10.), (19., 8.5)])
                .line(&[(12., 10.), (12., 15.)])
                .line(&[(8.5, 21.), (12., 15.), (15.5, 21.)]);
        }
        IconName::Layers => {
            p.closed(&[(12., 2.5), (22., 7.5), (12., 12.5), (2., 7.5)])
                .line(&[(2., 12.), (12., 17.), (22., 12.)])
                .line(&[(2., 16.5), (12., 21.5), (22., 16.5)]);
        }
        IconName::Chip => {
            p.rect(5., 5., 14., 14., 2.).rect(9., 9., 6., 6., 1.);
            for at in [9., 15.] {
                p.line(&[(at, 2.), (at, 5.)])
                    .line(&[(at, 19.), (at, 22.)])
                    .line(&[(2., at), (5., at)])
                    .line(&[(19., at), (22., at)]);
            }
        }
        IconName::Terminal => {
            p.line(&[(4., 17.), (10., 11.), (4., 5.)])
                .line(&[(12., 19.), (20., 19.)]);
        }
        IconName::Transfer => {
            p.line(&[(8., 3.), (4., 7.), (8., 11.)])
                .line(&[(4., 7.), (20., 7.)])
                .line(&[(16., 21.), (20., 17.), (16., 13.)])
                .line(&[(20., 17.), (4., 17.)]);
        }
        IconName::AppWindow => {
            p.rect(2., 4., 20., 16., 2.)
                .line(&[(2., 9.), (22., 9.)])
                .line(&[(6., 6.5), (6.01, 6.5)])
                .line(&[(9., 6.5), (9.01, 6.5)]);
        }
        IconName::Search => {
            p.circle(11., 11., 7.).line(&[(21., 21.), (16.2, 16.2)]);
        }
        IconName::Command => {
            p.arc(6., 6., 3., 0., -1.5 * PI)
                .arc(18., 6., 3., PI, 2.5 * PI)
                .arc(18., 18., 3., -PI / 2., PI)
                .arc(6., 18., 3., -PI / 2., -2. * PI)
                .line(&[(9., 9.), (15., 9.), (15., 15.), (9., 15.), (9., 9.)])
                .line(&[(9., 6.), (9., 9.)])
                .line(&[(15., 6.), (15., 9.)])
                .line(&[(9., 15.), (9., 18.)])
                .line(&[(15., 15.), (15., 18.)]);
        }
        IconName::Keyboard => {
            p.rect(2., 5., 20., 14., 2.)
                .line(&[(6., 9.), (6.01, 9.)])
                .line(&[(10., 9.), (10.01, 9.)])
                .line(&[(14., 9.), (14.01, 9.)])
                .line(&[(18., 9.), (18.01, 9.)])
                .line(&[(6., 12.5), (6.01, 12.5)])
                .line(&[(18., 12.5), (18.01, 12.5)])
                .line(&[(8., 15.5), (16., 15.5)]);
        }
        IconName::Sun => {
            p.circle(12., 12., 4.);
            for index in 0..8 {
                let angle = index as f32 * PI / 4.;
                let (s, c) = angle.sin_cos();
                p.line(&[
                    (12. + c * 7.5, 12. + s * 7.5),
                    (12. + c * 9.5, 12. + s * 9.5),
                ]);
            }
        }
        IconName::Moon => {
            p.arc(12., 12., 9., 0., 1.5 * PI);
            p.0.cubic_to(
                Offset::new(10.5, 7.5),
                Offset::new(15.5, 13.5),
                Offset::new(21., 12.),
            );
        }
        IconName::Refresh => {
            p.arc(12., 12., 8.5, -0.9 * PI, -0.12 * PI)
                .line(&[(20., 3.5), (20.2, 8.4), (15.3, 8.)])
                .arc(12., 12., 8.5, 0.1 * PI, 0.88 * PI)
                .line(&[(4., 20.5), (3.8, 15.6), (8.7, 16.)]);
        }
        IconName::Download => {
            p.line(&[(12., 3.), (12., 15.)])
                .line(&[(7., 10.), (12., 15.), (17., 10.)])
                .line(&[(4., 16.), (4., 19.), (20., 19.), (20., 16.)]);
        }
        IconName::Crosshair => {
            p.circle(12., 12., 8.)
                .line(&[(12., 2.), (12., 6.)])
                .line(&[(12., 18.), (12., 22.)])
                .line(&[(2., 12.), (6., 12.)])
                .line(&[(18., 12.), (22., 12.)]);
        }
        IconName::Eye => {
            p.0.move_to(Offset::new(2., 12.));
            p.0.cubic_to(
                Offset::new(5., 5.5),
                Offset::new(19., 5.5),
                Offset::new(22., 12.),
            );
            p.0.cubic_to(
                Offset::new(19., 18.5),
                Offset::new(5., 18.5),
                Offset::new(2., 12.),
            );
            p.circle(12., 12., 3.);
        }
        IconName::PanelLeft => {
            p.rect(3., 3., 18., 18., 2.).line(&[(9., 3.), (9., 21.)]);
        }
        IconName::Close => {
            p.line(&[(18., 6.), (6., 18.)])
                .line(&[(6., 6.), (18., 18.)]);
        }
        IconName::Copy => {
            p.rect(8., 8., 13., 13., 2.)
                .line(&[(4., 16.), (4., 5.), (5., 4.), (16., 4.)]);
        }
        IconName::Check => {
            p.line(&[(20., 6.), (9., 17.), (4., 12.)]);
        }
        IconName::ChevronRight => {
            p.line(&[(9., 18.), (15., 12.), (9., 6.)]);
        }
        IconName::ChevronDown => {
            p.line(&[(6., 9.), (12., 15.), (18., 9.)]);
        }
        IconName::Record => {
            p.circle(12., 12., 9.).circle(12., 12., 3.5);
        }
        IconName::Stop => {
            p.rect(5., 5., 14., 14., 2.);
        }
        IconName::Trash => {
            p.line(&[(3., 6.), (21., 6.)])
                .line(&[
                    (19., 6.),
                    (18., 20.),
                    (17., 21.),
                    (7., 21.),
                    (6., 20.),
                    (5., 6.),
                ])
                .line(&[
                    (8., 6.),
                    (8., 4.),
                    (9., 3.),
                    (15., 3.),
                    (16., 4.),
                    (16., 6.),
                ]);
        }
        IconName::Pause => {
            p.rect(6., 4., 4., 16., 1.).rect(14., 4., 4., 16., 1.);
        }
        IconName::Play => {
            p.closed(&[(6., 3.), (20., 12.), (6., 21.)]);
        }
        IconName::Zap => {
            p.closed(&[
                (13., 2.),
                (3., 14.),
                (12., 14.),
                (11., 22.),
                (21., 10.),
                (12., 10.),
            ]);
        }
        IconName::Alert => {
            p.closed(&[(12., 3.), (22., 20.), (2., 20.)])
                .line(&[(12., 9.), (12., 13.5)])
                .line(&[(12., 17.), (12.01, 17.)]);
        }
        IconName::Info => {
            p.circle(12., 12., 9.5)
                .line(&[(12., 16.), (12., 11.5)])
                .line(&[(12., 8.), (12.01, 8.)]);
        }
        IconName::Plug => {
            p.line(&[(12., 22.), (12., 17.)])
                .line(&[(9., 2.), (9., 8.)])
                .line(&[(15., 2.), (15., 8.)])
                .line(&[(6., 8.), (18., 8.), (18., 11.)]);
            p.0.cubic_to(
                Offset::new(18., 14.5),
                Offset::new(15.5, 17.),
                Offset::new(12., 17.),
            );
            p.0.cubic_to(
                Offset::new(8.5, 17.),
                Offset::new(6., 14.5),
                Offset::new(6., 11.),
            );
            p.0.close();
        }
        IconName::Filter => {
            p.closed(&[
                (3., 4.),
                (21., 4.),
                (14., 12.5),
                (14., 20.),
                (10., 18.),
                (10., 12.5),
            ]);
        }
        IconName::ArrowLeft => {
            p.line(&[(19., 12.), (5., 12.)])
                .line(&[(11., 18.), (5., 12.), (11., 6.)]);
        }
        IconName::ArrowRight => {
            p.line(&[(5., 12.), (19., 12.)])
                .line(&[(13., 6.), (19., 12.), (13., 18.)]);
        }
        IconName::Target => {
            p.circle(12., 12., 9.)
                .circle(12., 12., 5.)
                .circle(12., 12., 1.);
        }
        IconName::Sparkles => {
            p.closed(&[
                (11., 3.),
                (12.8, 8.2),
                (18., 10.),
                (12.8, 11.8),
                (11., 17.),
                (9.2, 11.8),
                (4., 10.),
                (9.2, 8.2),
            ])
            .line(&[(19., 15.), (19., 21.)])
            .line(&[(16., 18.), (22., 18.)]);
        }
    }
    p.finish()
}

macro_rules! cached {
    ($name:expr, $($variant:ident),* $(,)?) => {
        match $name {
            $(
                IconName::$variant => {
                    static PATH: OnceLock<Arc<Path>> = OnceLock::new();
                    PATH.get_or_init(|| build(IconName::$variant)).clone()
                }
            )*
        }
    };
}

/// The shared outline path of `name` on the 24-unit grid.
pub fn path(name: IconName) -> Arc<Path> {
    cached!(
        name,
        Overview,
        Tree,
        Activity,
        Monitor,
        Accessibility,
        Layers,
        Chip,
        Terminal,
        Transfer,
        AppWindow,
        Search,
        Command,
        Keyboard,
        Sun,
        Moon,
        Refresh,
        Download,
        Crosshair,
        Eye,
        PanelLeft,
        Close,
        Copy,
        Check,
        ChevronRight,
        ChevronDown,
        Record,
        Stop,
        Trash,
        Pause,
        Play,
        Zap,
        Alert,
        Info,
        Plug,
        Filter,
        ArrowLeft,
        ArrowRight,
        Target,
        Sparkles,
    )
}

/// `name` drawn `size` logical pixels square in `color`.
pub fn icon(name: IconName, size: f32, color: Color) -> Widget {
    let mut canvas = Canvas::default();
    canvas.save_transform(Transform::scale(size / 24.));
    canvas.stroke_path(
        path(name),
        color,
        Stroke {
            width: if size <= 14. { 2.25 } else { 2. },
            cap: LineCap::Round,
            join: LineJoin::Round,
            ..Stroke::default()
        },
    );
    canvas.restore();
    CustomPaint::new(Size::new(size, size), canvas.finish()).into()
}
