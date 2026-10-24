//! Flutter's `FadeInImage` (`widgets/fade_in_image.dart`): a placeholder
//! image that fades out as the target image, once loaded, fades in.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, Curve};
use incular_config::Alignment;
use incular_image::ImageHandle;

use crate::tree::ImageFit;
use crate::tree::RetainedState;
use crate::{Image, Opacity, Semantics, Stack, StackFit, Widget};

/// Flutter's `FadeInImage`. Incular decodes images synchronously, so the
/// application supplies `image` once its own loading completes: a target
/// present from the first build shows at once (Flutter's synchronously
/// loaded path), and one that arrives later fades in.
#[derive(Clone)]
pub struct FadeInImage {
    placeholder: ImageHandle,
    image: Option<ImageHandle>,
    fade_out_duration: Duration,
    fade_out_curve: Curve,
    fade_in_duration: Duration,
    fade_in_curve: Curve,
    width: Option<f32>,
    height: Option<f32>,
    fit: Option<ImageFit>,
    placeholder_fit: Option<ImageFit>,
    alignment: Alignment,
    image_semantic_label: Option<String>,
    exclude_from_semantics: bool,
}

impl FadeInImage {
    #[must_use]
    pub fn new(placeholder: ImageHandle, image: Option<ImageHandle>) -> Self {
        Self {
            placeholder,
            image,
            fade_out_duration: Duration::from_millis(300),
            fade_out_curve: Curve::EaseOut,
            fade_in_duration: Duration::from_millis(700),
            fade_in_curve: Curve::EaseIn,
            width: None,
            height: None,
            fit: None,
            placeholder_fit: None,
            alignment: Alignment::CENTER,
            image_semantic_label: None,
            exclude_from_semantics: false,
        }
    }

    /// Defaults to 300 ms.
    #[must_use]
    pub fn fade_out_duration(mut self, duration: Duration) -> Self {
        self.fade_out_duration = duration;
        self
    }

    /// Defaults to ease-out.
    #[must_use]
    pub fn fade_out_curve(mut self, curve: Curve) -> Self {
        self.fade_out_curve = curve;
        self
    }

    /// Defaults to 700 ms.
    #[must_use]
    pub fn fade_in_duration(mut self, duration: Duration) -> Self {
        self.fade_in_duration = duration;
        self
    }

    /// Defaults to ease-in.
    #[must_use]
    pub fn fade_in_curve(mut self, curve: Curve) -> Self {
        self.fade_in_curve = curve;
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    #[must_use]
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = Some(fit);
        self
    }

    /// Defaults to `fit`.
    #[must_use]
    pub fn placeholder_fit(mut self, fit: ImageFit) -> Self {
        self.placeholder_fit = Some(fit);
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub fn image_semantic_label(mut self, label: impl Into<String>) -> Self {
        self.image_semantic_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    fn image(&self, handle: ImageHandle, fit: Option<ImageFit>) -> Image {
        let mut image = Image::new(handle).alignment(self.alignment);
        if let Some(width) = self.width {
            image = image.width(width);
        }
        if let Some(height) = self.height {
            image = image.height(height);
        }
        if let Some(fit) = fit {
            image = image.fit(fit);
        }
        image
    }
}

struct FadeState {
    config: RefCell<Option<Rc<FadeInImage>>>,
    controller: AnimationController,
    /// `wasSynchronouslyLoaded`: the target was there on the first build.
    synchronous: Cell<bool>,
    /// The target opacity `_AnimatedFadeOutFadeIn` animates toward.
    loaded: Cell<bool>,
    listening: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for FadeState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl FadeState {
    fn update(&self, config: &Rc<FadeInImage>) {
        let first = self.config.replace(Some(config.clone())).is_none();
        let loaded = config.image.is_some();
        self.controller
            .set_duration(config.fade_out_duration + config.fade_in_duration);
        if first {
            self.synchronous.set(loaded);
            self.loaded.set(loaded);
            return;
        }
        // `ImplicitlyAnimatedWidget.didUpdateWidget`: a new target restarts
        // the animation from zero.
        if loaded != self.loaded.replace(loaded) && !self.synchronous.get() {
            self.controller.set_value(0.0);
            self.controller.forward_on_next_tick();
        }
    }

    /// The `TweenSequence`s of `_AnimatedFadeOutFadeInState.didUpdateTweens`:
    /// once the target loads, the placeholder fades out over the fade-out
    /// share of the timeline and the target fades in over the rest.
    fn opacities(&self, config: &FadeInImage) -> (f32, f32) {
        if !self.loaded.get() {
            return (1., 0.);
        }
        let out = config.fade_out_duration.as_secs_f32();
        let total = out + config.fade_in_duration.as_secs_f32();
        let split = if total > 0. { out / total } else { 0. };
        let t = self.controller.value();
        let placeholder = if t < split {
            1. - config.fade_out_curve.apply(t / split)
        } else {
            0.
        };
        let target = if t <= split {
            0.
        } else {
            config.fade_in_curve.apply((t - split) / (1. - split))
        };
        (placeholder, target)
    }

    fn build(&self) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the image updates before it builds");
        let target = config
            .image
            .clone()
            .map(|handle| config.image(handle, config.fit));
        let result: Widget =
            if self.synchronous.get() || (self.loaded.get() && self.controller.is_completed()) {
                target.map_or_else(
                    || {
                        config
                            .image(
                                config.placeholder.clone(),
                                config.placeholder_fit.or(config.fit),
                            )
                            .into()
                    },
                    Widget::from,
                )
            } else {
                let (placeholder_opacity, target_opacity) = self.opacities(&config);
                let placeholder = Opacity::new(
                    placeholder_opacity,
                    config.image(
                        config.placeholder.clone(),
                        config.placeholder_fit.or(config.fit),
                    ),
                );
                let mut children: Vec<Widget> = Vec::new();
                if let Some(target) = target {
                    children.push(Opacity::new(target_opacity, target).into());
                }
                children.push(placeholder.into());
                Stack::new(children)
                    .fit(StackFit::Passthrough)
                    .alignment(Alignment::CENTER)
                    .into()
            };
        let result = Widget::animation_ticker_with_revision(
            self.controller.clone(),
            false,
            self.revision.borrow().clone(),
            result,
        );
        if config.exclude_from_semantics {
            return result;
        }
        let mut semantics = Semantics::new(result).image(true);
        if let Some(label) = &config.image_semantic_label {
            semantics = semantics.label(label.clone());
        }
        semantics.into()
    }
}

impl From<FadeInImage> for Widget {
    fn from(value: FadeInImage) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |revision| FadeState {
                config: RefCell::new(None),
                controller: AnimationController::new(Duration::ZERO),
                synchronous: Cell::new(false),
                loaded: Cell::new(false),
                listening: Cell::new(false),
                revision: RefCell::new(revision),
            },
            move |state, _, _| {
                if !state.listening.replace(true) {
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |_| {
                        if let Some(state) = weak.upgrade() {
                            let revision = state.revision.borrow().clone();
                            revision.set(revision.get().wrapping_add(1));
                        }
                    });
                }
                state.update(&config);
                state.build()
            },
            false,
        )
    }
}
