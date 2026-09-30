# Snake

A native Incular port of the Ply website's Snake example. It uses the public
Incular facade, not Ply, Macroquad, a browser, or a custom WGPU renderer.

```text
cargo run -p incular --example snake
```

Use **arrow keys or WASD** to steer. Press **Space** after losing or winning to
restart. The game starts automatically, as in the original.

## Source layout

- `main.rs`: application, retained state/focus, keyboard input, and UI-thread
  timer completion.
- `game.rs`: dependency-free rules and seeded random food selection.
- `view.rs`: checkerboard canvas, segment geometry/colors, responsive layout,
  score badge, and terminal overlays.
- `tests/`: model, input, geometry, palette, layout, and simulation checks.

## Behavior

The port keeps the original 20 x 15 grid, 120 ms step interval, three-cell
starting snake, green head, 12-degree body hue steps, circular red food,
direction-aware rounded ends/turns, score badge, and restart overlay. The board
uses half the window width and its 4:3 aspect ratio; it also shrinks to fit a
short window. Overlay text scales down only when the board is very narrow.

Collision behavior intentionally matches Ply, including collision with the
currently occupied tail cell. Queued direction changes are checked against
the last executed direction. Auto-repeat events are ignored.

Two edge cases are handled explicitly: filling the board shows `You Win!`
instead of looping forever while looking for food, and a restarted game gets
a new full timer interval before it moves. Random food positions are not
intended to match the original random sequence.

State and focus identity live outside the application builder. At most one
Tokio sleep is pending. The sleep does not capture a `Signal`; `spawn_into`
delivers its completion on the UI thread, updates the game, and triggers the
next retained rebuild. No new timer is scheduled after losing or winning.
No dependencies or framework modifications are required beyond registering
the example and integration-test targets in `crates/incular/Cargo.toml`.

## Tests

Run all 31 included Rust tests, including the native-facade input/view checks:

```text
cargo test -p incular --test example_snake
```

For the native input/capture simulation on PowerShell:

```powershell
$env:INCULAR_EXAMPLE_SIMULATION = "1"
$env:INCULAR_EXAMPLE_SIMULATION_EXIT = "1"
cargo run -p incular --example snake
Remove-Item Env:INCULAR_EXAMPLE_SIMULATION
Remove-Item Env:INCULAR_EXAMPLE_SIMULATION_EXIT
```

On Linux/macOS:

```sh
INCULAR_EXAMPLE_SIMULATION=1 INCULAR_EXAMPLE_SIMULATION_EXIT=1 \
  cargo run -p incular --example snake
```

The simulation uses a fixed initial state and manual steps through the actual
keyboard listener. Real-time sleeps are disabled in this mode, so captures
do not depend on machine speed. F12 is the simulation-only step key; it is
ignored in ordinary play. The existing example harness saves these PPM files
under `target/example-review/screenshots/snake/`:

```text
initial.ppm
food-eaten.ppm
turned.ppm
game-over.ppm
after-interaction.ppm
```

These are application-frame captures, not OS desktop screenshots. The
simulation is an input/capture smoke test, not an image-diff assertion suite.

## Validation

The example compiles against the workspace's public facade. All 31 model,
input, and view tests pass. Use the native simulation above to exercise
rendering, food pickup, turns, collision, and restart through keyboard input.

For focused verification:

```text
cargo fmt --all -- --check
cargo check -p incular --example snake
cargo test -p incular --test example_snake
cargo clippy -p incular --example snake --test example_snake -- -D warnings
```

## References

Original design and behavior:
https://github.com/TheRedDeveloper/ply-website/blob/main/interactive-examples/src/examples/snake.rs

Original source blob read for this port:
`d86cb7abb2c31d78bcf8e9873d0957b65c2fac18`

Framework:
https://github.com/FabortLabs/incular

API examples consulted: `examples/canvas_layers/main.rs`,
`examples/async_runtime/main.rs`, `examples/layout_complete/main.rs`, and
`examples/undecorated_window/main.rs`. Input, corner radii, alignment, and
simulation signatures were also checked in the corresponding implementation
files. This port is not an upstream Ply release.
