use std::{cell::RefCell, rc::Rc};

use incular_config::{Brightness, Constraints, RuntimeEnvironment};
use incular_core::Size;
use incular_widgets::{
    Builder, Column, MediaQuery, MediaQueryData, Orientation, SizedBox, Widget,
    internal::WidgetTree,
};

#[test]
fn strict_and_optional_queries_use_no_implicit_default() {
    let child = Builder::new(|context| {
        assert_eq!(MediaQuery::maybe_of(context), None);
        let error =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| MediaQuery::of(context)))
                .expect_err("strict media lookup requires an ancestor");
        let message = error
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| error.downcast_ref::<String>().map(String::as_str))
            .unwrap();
        assert!(message.contains("No MediaQuery widget ancestor found"));
        assert_eq!(MediaQuery::maybe_size_of(context), None);
        assert_eq!(MediaQuery::maybe_width_of(context), None);
        assert_eq!(MediaQuery::maybe_height_of(context), None);
        assert_eq!(MediaQuery::maybe_orientation_of(context), None);
        SizedBox::shrink().into()
    });
    let mut tree = WidgetTree::new();
    tree.mount(child.into()).unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
}

#[test]
fn strict_and_optional_queries_inherit_same_nearest_default_data() {
    let child = Builder::new(|context| {
        let data = MediaQueryData::default();
        assert_eq!(MediaQuery::of(context), data);
        assert_eq!(MediaQuery::maybe_of(context), Some(data));
        SizedBox::shrink().into()
    });
    let mut tree = WidgetTree::new();
    tree.mount(
        MediaQuery::new(
            MediaQueryData {
                text_scale: 4.,
                ..Default::default()
            },
            MediaQuery::new(MediaQueryData::default(), child),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
}

#[test]
fn size_width_height_and_orientation_notify_separately() {
    let runs = Rc::new(RefCell::new(Vec::<(usize, String)>::new()));
    let children: Vec<Widget> = (0..8)
        .map(|slot| {
            let runs = runs.clone();
            Builder::new(move |context| {
                let state = match slot {
                    0 => format!("{:?}", MediaQuery::size_of(context)),
                    1 => format!("{:?}", MediaQuery::maybe_size_of(context).unwrap()),
                    2 => MediaQuery::width_of(context).to_string(),
                    3 => MediaQuery::maybe_width_of(context).unwrap().to_string(),
                    4 => MediaQuery::height_of(context).to_string(),
                    5 => MediaQuery::maybe_height_of(context).unwrap().to_string(),
                    6 => format!("{:?}", MediaQuery::orientation_of(context)),
                    7 => format!("{:?}", MediaQuery::maybe_orientation_of(context).unwrap()),
                    _ => unreachable!(),
                };
                runs.borrow_mut().push((slot, state));
                SizedBox::shrink().into()
            })
            .into()
        })
        .collect();
    let child: Widget = Column::new(children).into();
    let scope = |width, height, brightness| {
        MediaQuery::new(
            RuntimeEnvironment {
                viewport: Size::new(width, height),
                brightness,
                ..Default::default()
            },
            child.clone(),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let constraints = Constraints::loose(Size::new(500., 500.));
    let root = tree.mount(scope(200., 100., Brightness::Light)).unwrap();
    tree.layout(constraints).unwrap();
    let read = |slots: &[usize]| {
        let run = std::mem::take(&mut *runs.borrow_mut());
        let mut ran = run.iter().map(|(slot, _)| *slot).collect::<Vec<_>>();
        ran.sort_unstable();
        assert_eq!(ran, slots);
        run
    };
    let initial = read(&[0, 1, 2, 3, 4, 5, 6, 7]);
    assert!(
        initial
            .iter()
            .any(|(i, state)| *i == 6 && *state == format!("{:?}", Orientation::Landscape))
    );
    for ((w, h, brightness), slots) in [
        ((260., 100., Brightness::Light), &[0, 1, 2, 3][..]),
        ((260., 180., Brightness::Light), &[0, 1, 4, 5][..]),
        ((260., 260., Brightness::Light), &[0, 1, 4, 5, 6, 7][..]),
        ((260., 300., Brightness::Light), &[0, 1, 4, 5][..]),
        ((260., 300., Brightness::Dark), &[][..]),
    ] {
        tree.update(root, scope(w, h, brightness)).unwrap();
        tree.layout(constraints).unwrap();
        let ran = read(slots);
        if w == h {
            assert!(
                ran.iter()
                    .any(|(i, state)| *i == 6 && *state == format!("{:?}", Orientation::Portrait))
            );
        }
    }
}
