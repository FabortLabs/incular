# Forms fixes (group: forms)

## misc-004: Form.save skips callbacks for invalid fields
- Outcome: fixed (confirmed before fixing: scenario diverged, saved_count 1 vs 0).
- Cause: Form::save validated first and returned early on any invalid field. Flutter FormState.save (widgets/form.dart:314-319, 643-645) saves every field unconditionally.
- Change: crates/incular-widgets/src/forms.rs. save runs every save callback in registration order without validating. submit now holds validate-then-save. Fields are ordered by registration id.
- Tests: crates/incular-widgets/tests/forms.rs form_save_invokes_every_callback_in_registration_order_even_when_invalid, form_submit_validates_before_any_save_callback.
- Harness: misc__form_save_invalid_field now match (4 frames).
- Limitations: save returns cached validity (no revalidation); the bool is kept for compatibility.

## misc-007: GenericFormField not connected to retained state
- Outcome: fixed (confirmed before fixing: validator and on_saved were dropped in From<GenericFormField>).
- Cause: GenericFormField converted through the builder only; FormFieldState had no callbacks and no registration.
- Change: crates/incular-widgets/src/forms.rs. FormFieldState wraps a retained state registered with the form via Form::register_value. GenericFormField::state attaches it; conversion installs validator, on_saved and autovalidate, and the builder receives the retained value. Text and typed fields share a FormEntry trait and run_validation helper. crates/incular-widgets/src/lib.rs: GenericFormField re-exported (one line).
- Tests: crates/incular-widgets/tests/forms.rs typed_form_field_* and generic_form_field_* cases.
- Harness: no frame scenario added.
- API change: GenericFormField::validator takes Option<&T>; state-based setters added.
- Limitations: without a state, callbacks have no identity; enabled and restoration_id remain unimplemented; null values skip on_saved.

## misc-005: RawAutocomplete has no interactive options behavior
- Outcome: deferred.
- Reason: option view, highlight, keyboard shortcuts and overlay need a stateful widget bound to a text controller, focus and an overlay layer, outside forms.rs. A forms.rs-only model would be disconnected. Recommended for the controls/focus worker, building on incular-controls/src/autocomplete.rs.
