//! Form validation and autocomplete primitives.
//!
//! These types are deliberately controller-oriented.  A [`FormField`] owns
//! registration while it is mounted by application state, and can be paired
//! with any editor that uses a [`TextEditingController`].  This keeps form
//! state independent of a particular retained render node.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::{Rc, Weak},
};

use incular_core::{RestorationKey, RestorationScope};

use crate::Widget;
use incular_text::{TextEditingController, TextEditingValue};

/// Stable identity for a field registered with a [`Form`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormFieldId(u64);

/// When a field validates itself in response to user-originated editing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutovalidateMode {
    /// Validation only happens when [`Form::validate`] or
    /// [`FormField::validate`] is called.
    #[default]
    Disabled,
    /// Validate after [`FormField::set_text`] or
    /// [`FormField::notify_user_interaction`] is called.
    OnUserInteraction,
    /// Validate immediately when configured and after each user interaction.
    Always,
}

type Validator = Rc<dyn Fn(&str) -> Option<String>>;
type SubmitCallback = Rc<dyn Fn(String)>;

/// Type-erased view of a registered field, so text-backed fields and typed
/// [`FormFieldState`] values share one registry.
trait FormEntry {
    /// Runs the field's validator and records the resulting error.
    fn validate_entry(&self) -> bool;
    /// Invokes the field's save callback with its current value. Never
    /// validates and never checks the recorded error.
    fn save_entry(&self);
    /// Restores the field to its declared initial value and clears its error.
    fn reset_entry(&self);
    fn error_entry(&self) -> Option<String>;
    fn has_error_entry(&self) -> bool;
}

struct FieldState {
    controller: TextEditingController,
    initial_text: String,
    validator: RefCell<Option<Validator>>,
    on_submit: RefCell<Option<SubmitCallback>>,
    autovalidate: Cell<AutovalidateMode>,
    interacted: Cell<bool>,
    error: RefCell<Option<String>>,
    controller_listener: RefCell<Option<incular_text::TextEditingSubscription>>,
    validating: Cell<bool>,
}

impl FormEntry for FieldState {
    fn validate_entry(&self) -> bool {
        validate_field(self)
    }

    fn save_entry(&self) {
        let callback = self.on_submit.borrow().clone();
        if let Some(callback) = callback {
            callback(self.controller.text());
        }
    }

    fn reset_entry(&self) {
        self.controller.set_text(self.initial_text.clone());
        self.interacted.set(false);
        self.error.replace(None);
    }

    fn error_entry(&self) -> Option<String> {
        self.error.borrow().clone()
    }

    fn has_error_entry(&self) -> bool {
        self.error.borrow().is_some()
    }
}

#[derive(Default)]
struct FormRegistry {
    changes: incular_core::reactivity::DependencySource,
    next_id: u64,
    fields: HashMap<FormFieldId, Weak<dyn FormEntry>>,
    /// Ordered by token, so listeners run in registration order.
    listeners: BTreeMap<usize, Rc<dyn Fn()>>,
    /// Rebuilt when listeners change, so notifications do not copy the set.
    listener_snapshot: Rc<[Rc<dyn Fn()>]>,
    next_listener: usize,
    revision: u64,
}

impl FormRegistry {
    fn rebuild_listener_snapshot(&mut self) {
        self.listener_snapshot = self.listeners.values().cloned().collect::<Vec<_>>().into();
    }
}

type LiveField = (FormFieldId, Rc<dyn FormEntry>);

fn next_field_id(state: &RefCell<FormRegistry>) -> FormFieldId {
    let mut state = state.borrow_mut();
    state.next_id = state
        .next_id
        .checked_add(1)
        .expect("form field identity space exhausted");
    FormFieldId(state.next_id)
}

/// Form-level validation and submit coordination.
#[derive(Clone, Default)]
pub struct Form {
    state: Rc<RefCell<FormRegistry>>,
    child: Option<Widget>,
    autovalidate_mode: AutovalidateMode,
}

/// Rust-native replacement for Flutter's mutable `FormState` lookup handle.
///
/// A clone is the stable controller identity, so it survives declarative
/// rebuilds without requiring a `GlobalKey<FormState>` or a retained widget
/// object. [`FormController`] is an equivalent descriptive alias.
pub type FormState = Form;
pub type FormController = Form;

impl Form {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a declarative Form wrapping a child widget.
    #[must_use]
    pub fn of_child(child: impl Into<Widget>) -> Self {
        Self {
            state: Rc::new(RefCell::new(FormRegistry::default())),
            child: Some(child.into()),
            autovalidate_mode: AutovalidateMode::Disabled,
        }
    }

    /// Replaces only the declarative child while retaining this form's
    /// registry/controller identity.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets the autovalidate mode for the form.
    #[must_use]
    pub fn autovalidate_mode(mut self, mode: AutovalidateMode) -> Self {
        self.autovalidate_mode = mode;
        self
    }

    /// Registers an editor with this form.  Keep the returned handle alive for
    /// as long as the corresponding UI field is mounted; dropping it removes
    /// the registration automatically.
    #[must_use]
    pub fn register(&self, controller: TextEditingController) -> FormField {
        let id = next_field_id(&self.state);
        let field = Rc::new(FieldState {
            initial_text: controller.text(),
            controller: controller.clone(),
            validator: RefCell::new(None),
            on_submit: RefCell::new(None),
            autovalidate: Cell::new(AutovalidateMode::Disabled),
            interacted: Cell::new(false),
            error: RefCell::new(None),
            controller_listener: RefCell::new(None),
            validating: Cell::new(false),
        });
        let field_weak = Rc::downgrade(&field);
        let form_weak = Rc::downgrade(&self.state);
        let listener = controller.observe(move |_| {
            if let Some(field) = field_weak.upgrade()
                && field.autovalidate.get() != AutovalidateMode::Disabled
            {
                let _ = validate_field(&field);
            }
            if let Some(form) = form_weak.upgrade() {
                notify_form_listeners(&form);
            }
        });
        *field.controller_listener.borrow_mut() = Some(listener);
        let entry: Rc<dyn FormEntry> = field.clone();
        self.state
            .borrow_mut()
            .fields
            .insert(id, Rc::downgrade(&entry));
        notify_form_listeners(&self.state);
        FormField {
            id,
            field,
            form: Rc::downgrade(&self.state),
        }
    }

    /// Creates a typed field state registered with this form. The form
    /// validates, saves and resets the field alongside text-backed fields.
    ///
    /// The returned [`FormFieldState`] owns the registration: once every clone
    /// is dropped the form prunes the field on its next pass.
    #[must_use]
    pub fn register_value<T: Clone + 'static>(&self, initial: Option<T>) -> FormFieldState<T> {
        let id = next_field_id(&self.state);
        let state = FormFieldState::new(initial);
        state.inner.registration.set(Some(id));
        state.inner.form.replace(Some(Rc::downgrade(&self.state)));
        let entry: Rc<dyn FormEntry> = state.inner.clone();
        self.state
            .borrow_mut()
            .fields
            .insert(id, Rc::downgrade(&entry));
        notify_form_listeners(&self.state);
        state
    }

    /// Registers an editor and binds its committed value to a stable
    /// restoration key.
    ///
    /// Registration happens before binding so [`Self::reset`] still returns
    /// the caller's original default rather than the restored text.
    #[must_use]
    pub fn register_with_restoration(
        &self,
        controller: TextEditingController,
        scope: RestorationScope,
        key: RestorationKey,
    ) -> FormField {
        let field = self.register(controller);
        field.bind_restoration(scope, key);
        field
    }

    /// Registers an initially empty editor and restores it when a snapshot is
    /// available.
    #[must_use]
    pub fn register_restored(&self, scope: RestorationScope, key: RestorationKey) -> FormField {
        self.register_with_restoration(TextEditingController::new(), scope, key)
    }

    /// Validates every live field.  Fields that were unmounted are pruned as
    /// part of the operation, so forms cannot retain stale registrations.
    /// Every field in the initial live snapshot is visited even after a failure.
    /// The form notifies observers after the pass. User callbacks may cause
    /// additional notifications through their own mutations.
    #[must_use]
    pub fn validate(&self) -> bool {
        let valid = validate_fields(&self.live_fields());
        notify_form_listeners(&self.state);
        valid
    }

    /// Validates the whole form and returns the stable IDs of exactly the
    /// invalid fields. All attached fields have their error state updated.
    #[must_use]
    pub fn validate_granularly(&self) -> BTreeSet<FormFieldId> {
        let invalid = self
            .live_fields()
            .into_iter()
            .filter_map(|(id, field)| (!field.validate_entry()).then_some(id))
            .collect();
        notify_form_listeners(&self.state);
        invalid
    }

    /// Restores every live field to the value it had when registered.
    pub fn reset(&self) {
        for (_, field) in self.live_fields() {
            field.reset_entry();
        }
        notify_form_listeners(&self.state);
    }

    /// Invokes every live field's save callback with its current value, in
    /// registration order, whether or not the field is valid (Flutter
    /// `FormState.save`). Validators do not run here; use [`Self::submit`] for
    /// validate-then-save.
    ///
    /// Returns `true` when no field had a recorded error as of its last
    /// validation. The return value never gates the callbacks.
    #[must_use]
    pub fn save(&self) -> bool {
        let fields = self.live_fields();
        let all_error_free = fields.iter().all(|(_, field)| !field.has_error_entry());
        for (_, field) in &fields {
            field.save_entry();
        }
        notify_form_listeners(&self.state);
        all_error_free
    }

    /// Validates every live field, then invokes the save callbacks only when
    /// the whole form is valid. Returns whether the form was valid.
    #[must_use]
    pub fn submit(&self) -> bool {
        let fields = self.live_fields();
        if !validate_fields(&fields) {
            notify_form_listeners(&self.state);
            return false;
        }
        for (_, field) in &fields {
            field.save_entry();
        }
        notify_form_listeners(&self.state);
        true
    }

    /// Returns the current errors keyed by stable field identity.
    #[must_use]
    pub fn errors(&self) -> Vec<(FormFieldId, String)> {
        self.state.borrow().changes.track();
        self.live_fields()
            .into_iter()
            .filter_map(|(id, field)| field.error_entry().map(|error| (id, error)))
            .collect()
    }

    #[must_use]
    pub fn field_count(&self) -> usize {
        self.state.borrow().changes.track();
        self.live_fields().len()
    }

    /// Observes changes until the returned token is dropped.
    pub fn observe(&self, listener: impl Fn() + 'static) -> incular_core::reactivity::Subscription {
        self.state.borrow().changes.subscribe((), listener)
    }

    /// Adds a lightweight UI invalidation listener. The callback runs after a
    /// field edit, validation, save, or reset.
    pub fn add_listener(&self, listener: impl Fn() + 'static) -> usize {
        let mut state = self.state.borrow_mut();
        state.next_listener = state
            .next_listener
            .checked_add(1)
            .expect("form listener identity space exhausted");
        let token = state.next_listener;
        state.listeners.insert(token, Rc::new(listener));
        state.rebuild_listener_snapshot();
        token
    }

    /// Removes a listener previously returned by [`Self::add_listener`].
    pub fn remove_listener(&self, token: usize) -> bool {
        let mut state = self.state.borrow_mut();
        let removed = state.listeners.remove(&token).is_some();
        state.rebuild_listener_snapshot();
        removed
    }

    /// Reads the tracked revision of the form.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.borrow().changes.track();
        self.state.borrow().revision
    }

    /// Live fields in registration order, matching Flutter's save order.
    fn live_fields(&self) -> Vec<LiveField> {
        let mut state = self.state.borrow_mut();
        state.fields.retain(|_, field| field.strong_count() > 0);
        let mut fields: Vec<LiveField> = state
            .fields
            .iter()
            .filter_map(|(id, field)| field.upgrade().map(|field| (*id, field)))
            .collect();
        fields.sort_unstable_by_key(|(id, _)| *id);
        fields
    }
}

/// Registration and configuration handle for one text-backed form field.
pub struct FormField {
    id: FormFieldId,
    field: Rc<FieldState>,
    form: Weak<RefCell<FormRegistry>>,
}

impl FormField {
    #[must_use]
    pub const fn id(&self) -> FormFieldId {
        self.id
    }

    /// Installs the field's validation function.  `Some(message)` represents
    /// a validation failure. The callback runs without a configuration borrow;
    /// replacing it during invocation takes effect on its next invocation.
    #[must_use]
    pub fn validator(self, validator: impl Fn(&str) -> Option<String> + 'static) -> Self {
        self.field.validator.replace(Some(Rc::new(validator)));
        if self.field.autovalidate.get() == AutovalidateMode::Always {
            let _ = self.validate();
        }
        self.notify_form();
        self
    }

    #[must_use]
    pub fn autovalidate(self, mode: AutovalidateMode) -> Self {
        self.field.autovalidate.set(mode);
        if mode == AutovalidateMode::Always {
            let _ = self.validate();
        }
        self.notify_form();
        self
    }

    #[must_use]
    pub fn on_submit(self, callback: impl Fn(String) + 'static) -> Self {
        self.field.on_submit.replace(Some(Rc::new(callback)));
        self.notify_form();
        self
    }

    /// Installs the callback invoked by [`Form::save`]. This is the
    /// Flutter-shaped spelling; `on_submit` remains an interaction alias for
    /// existing callers.
    #[must_use]
    pub fn on_saved(self, callback: impl Fn(String) + 'static) -> Self {
        self.on_submit(callback)
    }

    /// Updates the backing editor and records user interaction for
    /// autovalidation.  Direct controller mutation remains supported for
    /// programmatic changes and does not imply user interaction.
    pub fn set_text(&self, text: impl Into<String>) {
        self.field.controller.set_text(text);
        self.notify_user_interaction();
    }

    /// Records an edit made through an externally-owned editor.
    pub fn notify_user_interaction(&self) {
        let first = !self.field.interacted.replace(true);
        if self.field.autovalidate.get() != AutovalidateMode::Disabled {
            let _ = self.validate();
        } else if first {
            self.notify_form();
        }
    }

    #[must_use]
    pub fn has_interacted_by_user(&self) -> bool {
        if let Some(form) = self.form.upgrade() {
            form.borrow().changes.track();
        }
        self.field.interacted.get()
    }

    /// Evaluates the validator without changing the displayed error.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        query_field(&self.field, false)
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let valid = validate_field(&self.field);
        self.notify_form();
        valid
    }

    #[must_use]
    pub fn error(&self) -> Option<String> {
        if let Some(form) = self.form.upgrade() {
            form.borrow().changes.track();
        }
        self.field.error.borrow().clone()
    }

    /// Persists this field's committed editor value and selection under a
    /// stable key. Validation errors are derived from the current validator
    /// and are intentionally not persisted.
    pub fn bind_restoration(&self, scope: RestorationScope, key: RestorationKey) {
        self.field.controller.bind_restoration(scope, key);
    }

    /// Stops persisting subsequent editor changes without removing the saved
    /// value from the restoration manager.
    pub fn unbind_restoration(&self) {
        self.field.controller.unbind_restoration();
    }

    pub fn reset(&self) {
        self.field.reset_entry();
        self.notify_form();
    }

    fn notify_form(&self) {
        if let Some(form) = self.form.upgrade() {
            notify_form_listeners(&form);
        }
    }
}

impl Drop for FormField {
    fn drop(&mut self) {
        self.field.controller_listener.borrow_mut().take();
        if let Some(form) = self.form.upgrade() {
            form.borrow_mut().fields.remove(&self.id);
            notify_form_listeners(&form);
        }
    }
}

fn validate_fields(fields: &[LiveField]) -> bool {
    let mut valid = true;
    for (_, field) in fields {
        // Evaluate every field so one failure cannot leave other errors stale.
        valid &= field.validate_entry();
    }
    valid
}

/// Records the result of `validate` in `error`. Reentrant validators (a
/// validator calling validate/save/set_text) return the last settled validity
/// instead of recursing unboundedly.
fn run_validation(
    validating: &Cell<bool>,
    error: &RefCell<Option<String>>,
    commit_error: bool,
    validate: impl FnOnce() -> Option<String>,
) -> bool {
    if validating.replace(true) {
        return error.borrow().is_none();
    }
    struct Reset<'a>(&'a Cell<bool>);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.0.set(false);
        }
    }
    let _guard = Reset(validating);
    let next_error = validate();
    let valid = next_error.is_none();
    if commit_error {
        error.replace(next_error);
    }
    valid
}

fn validate_field(field: &FieldState) -> bool {
    query_field(field, true)
}

fn query_field(field: &FieldState, commit_error: bool) -> bool {
    run_validation(&field.validating, &field.error, commit_error, || {
        let validator = field.validator.borrow().clone();
        validator.and_then(|validator| validator(&field.controller.text()))
    })
}

fn notify_registry(form: &Weak<RefCell<FormRegistry>>) {
    if let Some(form) = form.upgrade() {
        notify_form_listeners(&form);
    }
}

fn notify_form_listeners(form: &Rc<RefCell<FormRegistry>>) {
    let listeners = {
        let mut state = form.borrow_mut();
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
        Rc::clone(&state.listener_snapshot)
    };
    for listener in listeners.iter() {
        listener();
    }
    let changes = form.borrow().changes.clone();
    changes.notify();
}

impl From<Form> for Widget {
    fn from(value: Form) -> Self {
        value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into())
    }
}

/// Validator for a typed field. The value is `None` when the field has no
/// value, matching Flutter's nullable `T?` validator.
type GenericValidator<T> = Rc<dyn Fn(Option<&T>) -> Option<String>>;

/// Save callback for a typed field; it runs only while a value is present.
type GenericSaveCallback<T> = Rc<dyn Fn(T)>;

/// Generic FormField widget managing arbitrary typed form state and validation.
///
/// The builder receives the field's current value. Validation, saving and
/// autovalidation need retained state: attach a [`FormFieldState`] created with
/// [`Form::register_value`] through [`Self::state`]. When a state is attached,
/// `initial_value` is ignored (the state owns its initial value) and the
/// configured validator, save callback and autovalidate mode are installed on
/// it. Without a state the field is value-only: those settings have no
/// retained identity to run from.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct GenericFormField<T: Clone + 'static> {
    initial_value: Option<T>,
    validator: Option<GenericValidator<T>>,
    on_saved: Option<Rc<dyn Fn(T)>>,
    on_reset: Option<Rc<dyn Fn()>>,
    autovalidate_mode: AutovalidateMode,
    enabled: bool,
    restoration_id: Option<String>,
    state: Option<FormFieldState<T>>,
    builder: Rc<dyn Fn(Option<&T>) -> Widget>,
}

impl<T: Clone + 'static> GenericFormField<T> {
    #[must_use]
    pub fn new<W>(builder: impl Fn(Option<&T>) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            initial_value: None,
            validator: None,
            on_saved: None,
            on_reset: None,
            autovalidate_mode: AutovalidateMode::Disabled,
            enabled: true,
            restoration_id: None,
            state: None,
            builder: Rc::new(move |val| builder(val).into()),
        }
    }

    #[must_use]
    pub fn initial_value(mut self, value: T) -> Self {
        self.initial_value = Some(value);
        self
    }

    /// Connects the field to a retained [`FormFieldState`], typically one
    /// returned by [`Form::register_value`].
    #[must_use]
    pub fn state(mut self, state: FormFieldState<T>) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets the validator. `Some(message)` is a failure; `None` is the value
    /// passed to the validator when the field has no value.
    #[must_use]
    pub fn validator(mut self, validator: impl Fn(Option<&T>) -> Option<String> + 'static) -> Self {
        self.validator = Some(Rc::new(validator));
        self
    }

    #[must_use]
    pub fn on_saved(mut self, on_saved: impl Fn(T) + 'static) -> Self {
        self.on_saved = Some(Rc::new(on_saved));
        self
    }

    /// Runs after the retained field restores its initial value.
    #[must_use]
    pub fn on_reset(mut self, on_reset: impl Fn() + 'static) -> Self {
        self.on_reset = Some(Rc::new(on_reset));
        self
    }

    #[must_use]
    pub fn autovalidate_mode(mut self, mode: AutovalidateMode) -> Self {
        self.autovalidate_mode = mode;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn restoration_id(mut self, id: impl Into<String>) -> Self {
        self.restoration_id = Some(id.into());
        self
    }
}

impl<T: Clone + 'static> From<GenericFormField<T>> for Widget {
    fn from(value: GenericFormField<T>) -> Self {
        let Some(state) = value.state else {
            return (value.builder)(value.initial_value.as_ref());
        };
        if let Some(validator) = value.validator {
            state.inner.validator.replace(Some(validator));
        }
        if let Some(on_saved) = value.on_saved {
            state.inner.on_saved.replace(Some(on_saved));
        }
        state.inner.on_reset.replace(value.on_reset);
        if value.autovalidate_mode != AutovalidateMode::Disabled {
            // Installs after the validator so `Always` validates against it.
            state.set_autovalidate_mode(value.autovalidate_mode);
        }
        let current = state.value();
        (value.builder)(current.as_ref())
    }
}

/// Flutter `AutocompleteOptionsBuilder`: the options for the field's value.
pub type AutocompleteOptionsBuilder<T> = Rc<dyn Fn(&TextEditingValue) -> Vec<T>>;
/// Flutter `AutocompleteOnSelected`: selects an option from the options view.
pub type AutocompleteOnSelected<T> = Rc<dyn Fn(T)>;
/// Flutter `AutocompleteOptionsViewBuilder`: builds the options view from the
/// select callback and the current options. The highlighted option index is
/// [`AutocompleteHighlightedOption::of`].
pub type AutocompleteOptionsViewBuilder<T> =
    Rc<dyn for<'a> Fn(&crate::BuildContext<'a>, AutocompleteOnSelected<T>, &[T]) -> Widget>;
/// Flutter `AutocompleteFieldViewBuilder`: builds the input field bound to
/// the controller and focus node; calling the callback submits the field
/// (Flutter `onFieldSubmitted`), which selects the highlighted option.
pub type AutocompleteFieldViewBuilder = Rc<
    dyn for<'a> Fn(
        &crate::BuildContext<'a>,
        &TextEditingController,
        &crate::FocusNode,
        Rc<dyn Fn()>,
    ) -> Widget,
>;

type SelectedCallback<T> = Rc<dyn Fn(&T)>;

/// Flutter `RawAutocomplete._pageSize`: options a page key moves.
const AUTOCOMPLETE_PAGE_SIZE: usize = 4;

/// Flutter's `RawAutocomplete` (`widgets/autocomplete.dart`): a text field
/// whose options, built from the field's value, show in an overlay under the
/// field while it has focus. Arrow keys move the highlighted option (page
/// keys by four, Ctrl or Cmd with an arrow to the first or last), submitting
/// the field selects the highlighted option, Escape hides the options, and
/// selecting sets the field text to the option's display string, calls
/// `on_selected` and closes the options.
#[derive(Clone)]
pub struct RawAutocomplete<T: Clone + 'static> {
    options: Rc<[T]>,
    display_string_for_option: Rc<dyn Fn(&T) -> String>,
    child: Option<Widget>,
    options_builder: Option<AutocompleteOptionsBuilder<T>>,
    options_view_builder: Option<AutocompleteOptionsViewBuilder<T>>,
    field_view_builder: Option<AutocompleteFieldViewBuilder>,
    controller: Option<TextEditingController>,
    focus_node: Option<crate::FocusNode>,
    on_selected: Option<SelectedCallback<T>>,
}

impl<T: Clone + 'static> RawAutocomplete<T> {
    /// An autocomplete over fixed `options`, offered whole until
    /// [`Self::options_builder`] filters them.
    #[must_use]
    pub fn new(
        options: impl IntoIterator<Item = T>,
        display_string_for_option: impl Fn(&T) -> String + 'static,
    ) -> Self {
        Self {
            options: options.into_iter().collect(),
            display_string_for_option: Rc::new(display_string_for_option),
            child: None,
            options_builder: None,
            options_view_builder: None,
            field_view_builder: None,
            controller: None,
            focus_node: None,
            on_selected: None,
        }
    }

    /// A static field view, shown instead of the default editor. Prefer
    /// [`Self::field_view_builder`], which receives the controller and focus
    /// node the autocomplete listens to.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Flutter `optionsBuilder`, run whenever the field text changes.
    #[must_use]
    pub fn options_builder(
        mut self,
        builder: impl Fn(&TextEditingValue) -> Vec<T> + 'static,
    ) -> Self {
        self.options_builder = Some(Rc::new(builder));
        self
    }

    /// Flutter `optionsViewBuilder`.
    #[must_use]
    pub fn options_view_builder(
        mut self,
        builder: impl for<'a> Fn(&crate::BuildContext<'a>, AutocompleteOnSelected<T>, &[T]) -> Widget
        + 'static,
    ) -> Self {
        self.options_view_builder = Some(Rc::new(builder));
        self
    }

    /// Flutter `fieldViewBuilder`.
    #[must_use]
    pub fn field_view_builder(
        mut self,
        builder: impl for<'a> Fn(
            &crate::BuildContext<'a>,
            &TextEditingController,
            &crate::FocusNode,
            Rc<dyn Fn()>,
        ) -> Widget
        + 'static,
    ) -> Self {
        self.field_view_builder = Some(Rc::new(builder));
        self
    }

    /// Flutter `textEditingController`; an internal one is used otherwise.
    #[must_use]
    pub fn text_editing_controller(mut self, controller: TextEditingController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Flutter `focusNode`; an internal one is used otherwise.
    #[must_use]
    pub fn focus_node(mut self, node: crate::FocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Flutter `onSelected`.
    #[must_use]
    pub fn on_selected(mut self, callback: impl Fn(&T) + 'static) -> Self {
        self.on_selected = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn options(&self) -> &[T] {
        &self.options
    }

    #[must_use]
    pub fn display_string(&self, option: &T) -> String {
        (self.display_string_for_option)(option)
    }

    fn build_options(&self, value: &TextEditingValue) -> Rc<[T]> {
        match &self.options_builder {
            Some(builder) => builder(value).into(),
            None => Rc::clone(&self.options),
        }
    }
}

/// `_RawAutocompleteState`: the options, the highlighted index and whether
/// the options view shows.
struct RawAutocompleteState<T> {
    revision: RefCell<Rc<Cell<u64>>>,
    controller: TextEditingController,
    focus_node: crate::FocusNode,
    options: RefCell<Rc<[T]>>,
    /// `_lastFieldText`: options are rebuilt only when the text changes.
    last_text: RefCell<Option<String>>,
    had_focus: Cell<bool>,
    highlighted: Cell<usize>,
    /// `_optionsViewController.isShowing`.
    showing: Cell<bool>,
    /// `_selecting`: the field change a selection makes builds no options.
    selecting: Cell<bool>,
}

impl<T: 'static> crate::tree::RetainedState for RawAutocompleteState<T> {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl<T: Clone + 'static> RawAutocompleteState<T> {
    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// `_canShowOptionsView`.
    fn can_show(&self, focus: &crate::FocusNode) -> bool {
        focus.has_focus() && !self.options.borrow().is_empty()
    }

    /// `_updateOptionsViewVisibility`.
    fn update_visibility(&self, focus: &crate::FocusNode) {
        let show = self.can_show(focus);
        if self.showing.replace(show) != show {
            self.bump();
        }
    }

    /// `_updateHighlight`: clamps into the options.
    fn set_highlight(&self, index: isize) {
        let count = self.options.borrow().len();
        let next = if count == 0 {
            0
        } else {
            index.clamp(0, count as isize - 1) as usize
        };
        if self.highlighted.replace(next) != next {
            self.bump();
        }
    }

    /// `_highlightOption`: re-shows a hidden options view, then moves.
    fn highlight(&self, focus: &crate::FocusNode, index: isize) {
        self.update_visibility(focus);
        self.set_highlight(index);
    }

    /// `_onChangedField` and `_onFocusChange`, run while building.
    fn sync(
        &self,
        config: &RawAutocomplete<T>,
        controller: &TextEditingController,
        focus: &crate::FocusNode,
    ) {
        let focused = focus.has_focus();
        let text_changed = controller
            .with_value(|value| self.last_text.borrow().as_deref() != Some(value.text.as_str()));
        if text_changed && !self.selecting.get() {
            let value = controller.value();
            *self.last_text.borrow_mut() = Some(value.text.clone());
            *self.options.borrow_mut() = config.build_options(&value);
            // Building already rebuilds: no revision bump here.
            let count = self.options.borrow().len();
            self.highlighted
                .set(self.highlighted.get().min(count.saturating_sub(1)));
            self.showing.set(self.can_show(focus));
        } else if text_changed {
            let value = controller.value();
            *self.last_text.borrow_mut() = Some(value.text);
        }
        if self.had_focus.replace(focused) != focused {
            // Gaining focus can open the options view; losing it closes it.
            self.showing.set(self.can_show(focus));
        }
    }

    /// `_select`.
    fn select(&self, config: &RawAutocomplete<T>, controller: &TextEditingController, option: T) {
        self.selecting.set(true);
        let text = config.display_string(&option);
        // The listener skips this change while `_selecting`: the options
        // stay and the view does not reopen.
        *self.last_text.borrow_mut() = Some(text.clone());
        let end = text.len();
        controller.set_value(
            TextEditingValue::new(text).with_selection(incular_text::TextSelection::collapsed(end)),
        );
        if let Some(on_selected) = &config.on_selected {
            on_selected(&option);
        }
        if self.showing.replace(false) {
            self.bump();
        }
        self.selecting.set(false);
    }

    /// Flutter's autocomplete shortcuts and `DismissIntent`; a key is
    /// consumed only while the options can show.
    fn handle_key(&self, focus: &crate::FocusNode, event: &incular_core::KeyboardEvent) -> bool {
        use incular_core::Code;
        if !event.state.is_down() {
            return false;
        }
        if event.code == Code::Escape {
            // `_hideOptions`: an open view closes; otherwise the key
            // continues to the enclosing handlers.
            if self.showing.replace(false) {
                self.bump();
                return true;
            }
            return false;
        }
        if !self.can_show(focus) {
            return false;
        }
        let current = self.highlighted.get() as isize;
        let last = self.options.borrow().len() as isize - 1;
        let page = AUTOCOMPLETE_PAGE_SIZE as isize;
        let jump = event.modifiers.ctrl() || event.modifiers.meta();
        let target = match event.code {
            Code::ArrowUp if jump => 0,
            Code::ArrowDown if jump => last,
            Code::ArrowUp => current - 1,
            Code::ArrowDown => current + 1,
            Code::PageUp => current - page,
            Code::PageDown => current + page,
            _ => return false,
        };
        self.highlight(focus, target);
        true
    }
}

impl<T: Clone + 'static> From<RawAutocomplete<T>> for Widget {
    fn from(value: RawAutocomplete<T>) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            {
                let config = config.clone();
                move |revision| RawAutocompleteState::<T> {
                    revision: RefCell::new(revision),
                    controller: config.controller.clone().unwrap_or_default(),
                    focus_node: config.focus_node.clone().unwrap_or_default(),
                    options: RefCell::new(Rc::from([])),
                    last_text: RefCell::new(None),
                    had_focus: Cell::new(false),
                    highlighted: Cell::new(0),
                    showing: Cell::new(false),
                    selecting: Cell::new(false),
                }
            },
            move |state, context, _| {
                let controller = config
                    .controller
                    .clone()
                    .unwrap_or_else(|| state.controller.clone());
                let focus = config
                    .focus_node
                    .clone()
                    .unwrap_or_else(|| state.focus_node.clone());
                state.sync(&config, &controller, &focus);

                let weak = Rc::downgrade(state);
                let select: AutocompleteOnSelected<T> = Rc::new({
                    let config = config.clone();
                    let controller = controller.clone();
                    let weak = weak.clone();
                    move |option| {
                        if let Some(state) = weak.upgrade() {
                            state.select(&config, &controller, option);
                        }
                    }
                });
                // `_onFieldSubmitted`: selects the highlighted option while
                // the options view shows.
                let submitted: Rc<dyn Fn()> = Rc::new({
                    let select = select.clone();
                    let weak = weak.clone();
                    move || {
                        let Some(state) = weak.upgrade() else {
                            return;
                        };
                        if !state.showing.get() {
                            return;
                        }
                        let option = state.options.borrow().get(state.highlighted.get()).cloned();
                        if let Some(option) = option {
                            select(option);
                        }
                    }
                });
                let field = match (&config.field_view_builder, &config.child) {
                    (Some(builder), _) => builder(context, &controller, &focus, submitted),
                    (None, Some(child)) => child.clone(),
                    (None, None) => {
                        let submitted = submitted.clone();
                        crate::Focus::new(
                            crate::EditableText::new(controller.clone())
                                .on_submit(move |_| submitted()),
                        )
                        .node(focus.clone())
                        .into()
                    }
                };
                let field: Widget = crate::KeyboardListener::new(field)
                    .on_key({
                        let weak = weak.clone();
                        let focus = focus.clone();
                        move |event| {
                            weak.upgrade()
                                .is_some_and(|state| state.handle_key(&focus, &event))
                        }
                    })
                    .into();

                let showing = state.showing.get() && config.options_view_builder.is_some();
                let mut portal = crate::OverlayPortal::new(field).show(showing);
                if showing && let Some(view) = config.options_view_builder.clone() {
                    let options = Rc::clone(&state.options.borrow());
                    let highlighted = state.highlighted.get();
                    let dismiss = weak.clone();
                    portal = portal
                        .on_dismiss(move |_| {
                            if let Some(state) = dismiss.upgrade()
                                && state.showing.replace(false)
                            {
                                state.bump();
                            }
                        })
                        .overlay_child_layout_builder(move |_, info| {
                            // `_buildOptionsView` opening down: the field's
                            // width, from its bottom edge.
                            let anchor = info.anchor_rect;
                            let below = (info.overlay_size.height
                                - (anchor.origin.y + anchor.size.height))
                                .max(48.);
                            // The view builds below the highlight scope, so
                            // `AutocompleteHighlightedOption::of` reads it.
                            let (view, select, options) =
                                (view.clone(), select.clone(), options.clone());
                            let content = Widget::environment_scope(
                                AutocompleteHighlight(highlighted),
                                Widget::stateful_builder(Rc::default(), move |context| {
                                    view(context, select.clone(), &options)
                                }),
                            );
                            crate::Stack::new([crate::Positioned::builder()
                                .left(anchor.origin.x)
                                .top(anchor.origin.y + anchor.size.height)
                                .width(anchor.size.width)
                                .height(below)
                                .child(crate::Align::new(
                                    incular_config::Alignment::TOP_LEFT,
                                    crate::TextFieldTapRegion::new(content),
                                ))
                                .build()])
                            .into()
                        });
                }
                portal.into()
            },
            false,
        )
    }
}

/// The highlighted option index an autocomplete gives its options view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AutocompleteHighlight(usize);

/// Highlights the currently focused option in an autocomplete view.
#[derive(Clone, Debug, PartialEq)]
pub struct AutocompleteHighlightedOption {
    highlighted: bool,
    child: Widget,
}

impl AutocompleteHighlightedOption {
    #[must_use]
    pub fn new(highlighted: bool, child: impl Into<Widget>) -> Self {
        Self {
            highlighted,
            child: child.into(),
        }
    }

    /// Flutter `AutocompleteHighlightedOption.of`: the index of the option
    /// the arrow keys highlighted, for an options view built by a
    /// [`RawAutocomplete`]; 0 elsewhere.
    #[must_use]
    pub fn of(context: &crate::BuildContext<'_>) -> usize {
        context
            .depend_on::<AutocompleteHighlight>()
            .map_or(0, |highlight| highlight.0)
    }
}

impl From<AutocompleteHighlightedOption> for Widget {
    fn from(value: AutocompleteHighlightedOption) -> Self {
        value.child
    }
}

/// Flutter's autofill context for its mounted text input descendants. A
/// nested group starts its own scope rather than joining the outer group.
#[derive(Clone, Debug, PartialEq)]
pub struct AutofillGroup {
    child: Widget,
    action: incular_platform::autofill::AutofillContextAction,
}

impl AutofillGroup {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            action: incular_platform::autofill::AutofillContextAction::Commit,
        }
    }

    /// Controls whether the outermost context is saved or cancelled when
    /// the group is removed. Defaults to commit.
    #[must_use]
    pub fn on_dispose_action(mut self, action: incular_platform::autofill::AutofillContextAction) -> Self {
        self.action = action;
        self
    }
}

impl From<AutofillGroup> for Widget {
    fn from(value: AutofillGroup) -> Self {
        struct GroupScope;
        Widget::environment_scope(GroupScope, value.child).with_autofill_group(value.action)
    }
}

/// Manages undo and redo history for editable text input.
#[derive(Clone, Debug, PartialEq)]
pub struct UndoHistory {
    child: Widget,
    max_entries: usize,
}

impl UndoHistory {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            max_entries: 100,
        }
    }

    /// Limits the number of committed editor states retained by the runtime
    /// history controller. Zero disables recording while leaving editing
    /// enabled.
    #[must_use]
    pub fn max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }
}

impl From<UndoHistory> for Widget {
    fn from(value: UndoHistory) -> Self {
        value.child.with_undo_history_max_entries(value.max_entries)
    }
}

/// Retained value, error and callbacks of a typed form field.
///
/// Create one with [`Form::register_value`] so the form validates, saves and
/// resets it, or with [`Self::new`] for a state no form drives. Clones share
/// the same state. Attach it to a [`GenericFormField`] with
/// [`GenericFormField::state`] to render it.
#[derive(Clone)]
pub struct FormFieldState<T> {
    inner: Rc<GenericFieldState<T>>,
}

struct GenericFieldState<T> {
    initial: Option<T>,
    value: RefCell<Option<T>>,
    error: RefCell<Option<String>>,
    validator: RefCell<Option<GenericValidator<T>>>,
    on_saved: RefCell<Option<GenericSaveCallback<T>>>,
    on_reset: RefCell<Option<Rc<dyn Fn()>>>,
    autovalidate: Cell<AutovalidateMode>,
    interacted: Cell<bool>,
    registration: Cell<Option<FormFieldId>>,
    validating: Cell<bool>,
    form: RefCell<Option<Weak<RefCell<FormRegistry>>>>,
}

impl<T: Clone + 'static> GenericFieldState<T> {
    fn validate_value(&self) -> bool {
        self.query_validity(true)
    }

    fn query_validity(&self, commit_error: bool) -> bool {
        let value = self.value.borrow().clone();
        run_validation(&self.validating, &self.error, commit_error, || {
            let validator = self.validator.borrow().clone();
            validator.and_then(|validator| validator(value.as_ref()))
        })
    }

    fn reset_value(&self, value: Option<T>) {
        self.value.replace(value);
        self.error.replace(None);
        self.interacted.set(false);
        let callback = self.on_reset.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }

    fn save_value(&self) {
        let callback = self.on_saved.borrow().clone();
        let value = self.value.borrow().clone();
        if let (Some(callback), Some(value)) = (callback, value) {
            callback(value);
        }
    }

    fn notify_form(&self) {
        let form = self.form.borrow().clone();
        if let Some(form) = form {
            notify_registry(&form);
        }
    }
}

impl<T: Clone + 'static> FormEntry for GenericFieldState<T> {
    fn validate_entry(&self) -> bool {
        self.validate_value()
    }

    fn save_entry(&self) {
        self.save_value();
    }

    fn reset_entry(&self) {
        self.reset_value(self.initial.clone());
    }

    fn error_entry(&self) -> Option<String> {
        self.error.borrow().clone()
    }

    fn has_error_entry(&self) -> bool {
        self.error.borrow().is_some()
    }
}

impl<T: Clone + 'static> FormFieldState<T> {
    /// Creates a standalone state that no form validates or saves. Use
    /// [`Form::register_value`] to join a form.
    #[must_use]
    pub fn new(initial: Option<T>) -> Self {
        Self {
            inner: Rc::new(GenericFieldState {
                value: RefCell::new(initial.clone()),
                initial,
                error: RefCell::new(None),
                validator: RefCell::new(None),
                on_saved: RefCell::new(None),
                on_reset: RefCell::new(None),
                autovalidate: Cell::new(AutovalidateMode::Disabled),
                interacted: Cell::new(false),
                registration: Cell::new(None),
                validating: Cell::new(false),
                form: RefCell::new(None),
            }),
        }
    }

    #[must_use]
    pub fn value(&self) -> Option<T> {
        self.inner.value.borrow().clone()
    }

    #[must_use]
    pub fn error_text(&self) -> Option<String> {
        self.inner.error.borrow().clone()
    }

    #[must_use]
    pub fn has_error(&self) -> bool {
        self.inner.error.borrow().is_some()
    }

    /// The field's identity in its form, or none for standalone state.
    #[must_use]
    pub fn id(&self) -> Option<FormFieldId> {
        self.inner.registration.get()
    }

    #[must_use]
    pub fn has_interacted_by_user(&self) -> bool {
        self.inner.interacted.get()
    }

    /// Evaluates the validator without updating the displayed error.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.inner.query_validity(false)
    }

    /// Installs the validator. `Some(message)` is a failure.
    pub fn set_validator(&self, validator: impl Fn(Option<&T>) -> Option<String> + 'static) {
        self.inner.validator.replace(Some(Rc::new(validator)));
        self.inner.notify_form();
    }

    /// Installs the callback invoked by [`Self::save`] and [`Form::save`]. It
    /// runs only while a value is present.
    pub fn set_on_saved(&self, on_saved: impl Fn(T) + 'static) {
        self.inner.on_saved.replace(Some(Rc::new(on_saved)));
    }

    pub fn set_on_reset(&self, on_reset: impl Fn() + 'static) {
        self.inner.on_reset.replace(Some(Rc::new(on_reset)));
    }

    /// Sets when the field revalidates. [`AutovalidateMode::Always`]
    /// validates immediately, as Flutter does during build.
    pub fn set_autovalidate_mode(&self, mode: AutovalidateMode) {
        self.inner.autovalidate.set(mode);
        if mode == AutovalidateMode::Always {
            let _ = self.validate();
        } else {
            self.inner.notify_form();
        }
    }

    /// Records a new value (Flutter `FormFieldState.didChange`) and revalidates
    /// when autovalidation is enabled.
    pub fn did_change(&self, value: T) {
        self.inner.value.replace(Some(value));
        self.inner.interacted.set(true);
        if self.inner.autovalidate.get() != AutovalidateMode::Disabled {
            let _ = self.inner.validate_value();
        }
        self.inner.notify_form();
    }

    /// Validates the current value, records any error and returns validity.
    #[must_use]
    pub fn validate(&self) -> bool {
        let valid = self.inner.validate_value();
        self.inner.notify_form();
        valid
    }

    /// Invokes the save callback with the current value when one is present.
    pub fn save(&self) {
        self.inner.save_value();
    }

    /// Replaces the value with `initial` and clears the recorded error.
    pub fn reset(&self, initial: Option<T>) {
        self.inner.reset_value(initial);
        self.inner.notify_form();
    }
}

pub use crate::text_formatters::{
    CallbackTextInputFormatter, FilteringTextInputFormatter, LengthLimitingTextInputFormatter,
    MaxLengthEnforcement, TextInputFormatter,
};
