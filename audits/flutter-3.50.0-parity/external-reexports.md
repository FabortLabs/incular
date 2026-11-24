# External re-exports and scope

The AST inventory follows local framework Dart files. It does not expand the
Dart SDK, Flutter engine, or third-party packages. The edge log preserves
export statements seen during traversal, before deduplication and propagated
show/hide. It contains 332 observations: 67 source-file/directive combinations,
37 distinct statement texts, and 101 distinct names in explicit show lists
(75 dart:ui, 4 listen, 13 meta, 7 typed_data, 1 io, 1 vector_math). These are
edge-log diagnostics, not counts of names resolvable from each public barrel.
The unrestricted package:characters export is separate.

| Source family | Disposition |
| --- | --- |
| `package:listen` | In scope for portable notifications, value reads, subscriptions and merge/lifecycle. Incular has Signal and RAII dependency observation, not a full public API match (F01). |
| `dart:ui` geometry, locale, style and callbacks | Incular geometry/rendering/config and Rust primitives are candidates, not a whole-contract pass. Review geometric edge cases, directionality, clipping and numeric rules explicitly. |
| `dart:ui` painting and media | Source-match externally visible Canvas/Paint/Path, gradients, filters, blending, strokes, clips, images and vertex/texture behavior. Incular's renderer-neutral commands and wgpu are the implementation. Flutter engine handles/representations and specific GPU backends are not port targets. |
| `dart:ui` text | Candidate mappings exist for fonts, affinity, text ranges, direction, glyph and line metrics; detailed offsets, hit regions, placeholders, hyphen controls and edit boundaries need contract review. See P04 and SV01. |
| `dart:ui` accessibility | Semantic updates, validation, accessible features, action events and attributed text remain in scope. Flutter engine wire/object types are not, unless explicitly required for interoperability. See SE01–SE03. |
| `dart:ui` host, scheduling and input | Lifecycle, window/metrics, frame timing, pointer/keyboard and platform-message behaviors remain in scope. Rust window and event types are partial mappings, not proof of parity. Dart isolate identity and deprecated engine-window objects are not exact-type targets. See SC02, SV04 and SV05. |
| `package:vector_math` Matrix4 | Application-visible transformation behavior remains in scope. Incular's core affine Transform is not full 4×4/perspective support; no requirement to port vector_math itself. See W11. |
| `package:characters` | Grapheme traversal and editing boundaries remain in scope. Use Rust Unicode representation and evaluate real strings rather than require a Dart package API. |
| `package:meta`, `dart:typed_data`, `dart:io` | Language annotations, typed storage and HTTP object identities use Rust equivalents. Validate observable contracts such as bounds/byte order, streaming/cancellation and asset loading independently. |

No grouped disposition above constitutes a proven member-level pass.
