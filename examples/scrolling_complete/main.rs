//! Scroll policy and nested-delta visual exercise.
//!
//! The inner list is deliberately scrollable inside an outer header/content
//! viewport. At its top or bottom, wheel remainder transfers to the outer
//! viewport exactly once. The variable rows use the retained sliver list path.
use incular::prelude::*;
use incular::widgets::internal::ScrollView;

// Local orchid palette.
const DEMO_CANVAS: Color = Color::rgba(35, 25, 39, 255);
const DEMO_SURFACE: Color = Color::rgba(51, 37, 56, 255);
const DEMO_TEXT: Color = Color::rgba(253, 240, 237, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    // Applications may drive kinetic completion from their monotonic frame
    // callback with this policy; the retained wheel path uses clamped nested
    // transfer by default.
    let _touch_policy = ScrollPhysics::clamping().bouncing().page_snapping(320.);
    let app = Application::new(move |_| {
        let content: Widget = {
            let list: Widget = CustomScrollView::new(vec![Box::new(SliverVariedExtentList::new(
                2_000,
                |item| if item % 3 == 0 { 56. } else { 32. },
                |item| {
                    Widget::box_(
                        Size::new(360., if item % 3 == 0 { 56. } else { 32. }),
                        Color::rgba(45, 85 + (item % 4) as u8 * 24, 145, 255),
                    )
                },
            )) as Box<dyn Sliver>])
            .controller(inner.clone())
            .into();
            let inner_view: Widget = SizedBox::from_size(Size::new(360., 320.))
                .child(list)
                .into();
            ScrollView::vertical(
                outer.clone(),
                Widget::from(Column::new(Vec::<Widget>::from([
                    Widget::box_(Size::new(360., 120.), DEMO_SURFACE),
                    inner_view,
                    Widget::box_(Size::new(360., 700.), DEMO_SURFACE),
                ]))),
            )
        };
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::TOP_LEFT)
            .child(DefaultTextStyle::new(
                TextStyle::new().color(DEMO_TEXT),
                content,
            ))
            .into()
    })
    .expect("valid scrolling application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native scrolling application");
}
