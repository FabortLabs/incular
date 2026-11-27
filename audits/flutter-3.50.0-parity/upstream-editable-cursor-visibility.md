# Editable cursor visibility and focus

Five pinned originals in `widgets/editable_text_cursor_test.dart`
passed. The renderer-neutral runtime checks:

- The configured 10×10 cursor with radius 2 reaches the retained rounded
  paint command.
- An explicitly hidden cursor emits no caret paint even while its editor
  has focus, including the three 200 ms checkpoints.
- Blur stops cursor paint and animation demand; requesting focus again
  restores the cursor on the same retained editor.
- Toggling visibility off, on, and off changes the caret without replacing
  the render ID or stealing keyboard focus.
- An initially unset selection in an autofocused scope collapses at the
  end of `hello world`; the focused editor paints the first caret.

All 10 focused runtime caret and focus tests passed. The rounded paint
assertion is not a rendered-pixel golden. Exact repeated blink phase
timing is a separate audit slice.
