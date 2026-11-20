//! Motion showcase: a photo grid with hover lift, press, a group-revealed
//! label, staggered entry, animated removal, drag-to-reorder with siblings
//! gliding aside, a shared-element morph into a detail view, a one-bounce
//! shuffle (`with_transition`) and a circular theme reveal
//! (`view_transition`).

use incular::motion::{
    Group, Move, Pose, Reveal, Shadow, Stagger, Transition, ViewTransitionExt as _,
};
use incular::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Photo {
    id: u32,
    color: Color,
}

const CARD: Group = Group::new("card");

fn palette() -> Vec<Photo> {
    [
        (230, 90, 70),
        (240, 170, 60),
        (120, 190, 90),
        (60, 160, 200),
        (110, 100, 210),
        (200, 90, 170),
        (90, 90, 90),
        (40, 140, 120),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (r, g, b))| Photo {
        id: index as u32,
        color: Color::rgba(r, g, b, 255),
    })
    .collect()
}

fn lift(p: Pose) -> Pose {
    p.y(-3.).shadow(Shadow::new(
        Color::rgba(0, 0, 0, 70),
        Offset::new(0., 6.),
        14.,
    ))
}

fn text(value: String, color: Color) -> Widget {
    Text::new(value)
        .style(TextStyle::new().font_size(13.0).color(color))
        .into()
}

fn photo_card(photo: Photo, selected: Signal<Option<u32>>, photos: Signal<Vec<Photo>>) -> Widget {
    let open = selected.clone();
    let remove = photos.clone();
    // Revealed while the pointer is over the card's group.
    let label = text(format!("#{}", photo.id), Color::WHITE).motion(|m| {
        m.rest(|p| p.opacity(0.).x(-6.))
            .when(CARD.hovered() | CARD.focus_within(), |p| {
                p.opacity(1.).x(0.)
            })
    });
    let close = GestureDetector::new(text("remove".to_owned(), Color::WHITE))
        .on_tap(move || remove.update(|list| list.retain(|p| p.id != photo.id)));
    let face = Container::new()
        .width(120.)
        .height(120.)
        .color(photo.color)
        .padding(EdgeInsets::all(8.))
        .child(Column::new([label, close.into()]))
        .motion(|m| m.shared(("photo", photo.id)));
    Widget::from(GestureDetector::new(face).on_tap(move || {
        open.set(Some(photo.id));
    }))
    .with_key(u64::from(photo.id))
    .reorder_item(photo.id)
    .label(format!("Photo {}", photo.id))
    .motion(|m| {
        m.group(CARD)
            .enter(|p| p.opacity(0.).y(12.))
            .exit(|p| p.opacity(0.).scale(0.95))
            .hovered(lift)
            .pressed(|p| p.scale(0.98).transition(Transition::ease(ms(80))))
            .dragging(|p| p.scale(1.06).lift(28.))
    })
}

fn detail(photo: Photo, selected: Signal<Option<u32>>) -> Widget {
    let big = Container::new()
        .width(320.)
        .height(320.)
        .color(photo.color)
        .motion(|m| m.shared(("photo", photo.id)));
    let scrim = GestureDetector::new(
        Container::new()
            .color(Color::rgba(0, 0, 0, 120))
            .child(Center::new(big)),
    )
    .on_tap(move || {
        selected.set(None);
    });
    Widget::from(scrim)
        .with_key("detail")
        .motion(|m| m.enter(|p| p.opacity(0.)).exit(|p| p.opacity(0.)))
}

fn button(label: &str, on_tap: impl Fn() + 'static) -> Widget {
    GestureDetector::new(
        Container::new()
            .padding(EdgeInsets::symmetric(12., 6.))
            .color(Color::rgba(40, 40, 48, 255))
            .child(text(label.to_owned(), Color::WHITE)),
    )
    .on_tap(on_tap)
    .motion(|m| m.hovered(|p| p.brightness(1.15)).pressed(|p| p.scale(0.96)))
}

fn main() {
    let photos = Signal::new(palette());
    let selected = Signal::new(None::<u32>);
    let dark = Signal::new(false);
    let app = Application::new(move |_cx| -> Widget {
        let list = photos.get();
        let reorder = photos.clone();
        let grid = Wrap::new(
            list.iter()
                .map(|photo| photo_card(*photo, selected.clone(), photos.clone())),
        )
        .spacing(12.)
        .run_spacing(12.)
        .reorderable(move |mv: Move<u32>| reorder.update(|v| mv.apply(v, |p| p.id)))
        .motion(|m| m.stagger(Stagger::every(ms(30))));

        let shuffle = {
            let photos = photos.clone();
            button("Shuffle", move || {
                with_transition(Spring::bouncy(), || photos.update(|v| v.reverse()));
            })
        };
        let reset = {
            let photos = photos.clone();
            button("Reset", move || {
                photos.set(palette());
            })
        };
        let theme = {
            let dark = dark.clone();
            button("Theme", move || {
                let _ = view_transition(|| dark.update(|d| *d = !*d))
                    .reveal(Reveal::circle_from(Offset::new(40., 20.)))
                    .duration(ms(450));
            })
        };
        let background = if dark.get() {
            Color::rgba(24, 24, 28, 255)
        } else {
            Color::rgba(247, 243, 235, 255)
        };
        let page: Widget = Container::new()
            .color(background)
            .padding(EdgeInsets::all(20.))
            .child(Column::new([
                Widget::from(Row::new([shuffle, reset, theme]).spacing(8.)),
                SizedBox::new().height(16.).into(),
                grid,
            ]))
            .into();
        let open = selected
            .get()
            .and_then(|id| list.iter().find(|p| p.id == id).copied());
        let mut layers = vec![page];
        if let Some(photo) = open {
            layers.push(detail(photo, selected.clone()));
        }
        Directionality::new(
            TextDirection::Ltr,
            Stack::new(layers).fit(incular::widgets::StackFit::Expand),
        )
        .into()
    })
    .expect("valid application");
    incular::run(app).expect("run application");
}

// Keeps the trait import used on every target (scopes are optional here).
#[expect(dead_code, reason = "keeps ViewTransitionExt in scope on every target")]
fn scoped(widget: Widget, scope: &incular::motion::ViewScope) -> Widget {
    widget.view_transition_scope(scope)
}
