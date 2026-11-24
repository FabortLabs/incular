# Input, widgets, and retained layout rechecks

Fifteen strict fixture outcomes matched pinned Flutter 3.50.0 `ab2912035e`.
Incular production revision remains `c647cb85`.

| Scratch batch | Scenario | Frames |
| --- | --- | ---: |
| `20261124-160831-646` | `focus_input__shortcut_chord_requires_modifiers` | 4 |
| `20261124-160831-646` | `focus_input__mouse_region_opaque_blocks_underlay` | 3 |
| `20261124-160831-646` | `focus_input__mouse_region_translucent_passes_underlay` | 3 |
| `20261124-160831-646` | `transitions__compositor__fade_slide_scale_rotation` | 6 |
| `20261124-160831-646` | `transitions__size__reverse_mid_flight` | 33 |
| `20261124-161009-757` | `misc__stream_builder_events` | 5 |
| `20261124-161009-757` | `misc__safe_area_nested_consumption` | 3 |
| `20261124-161009-757` | `misc__directionality_row_order` | 3 |
| `20261124-161009-757` | `misc__form_field_lifecycle` | 7 |
| `20261124-161009-757` | `navigation__router__provider_route_change` | 5 |
| `20261124-161249-808` | `gestures__draggable_drop_target` | 6 |
| `20261124-161249-808` | `gestures__interactive_viewer_mouse_wheel_zoom` | 3 |
| `20261124-161249-808` | `layout__custom_single_child` | 2 |
| `20261124-161249-808` | `layout__custom_multi_child` | 2 |
| `20261124-161249-808` | `layout__flow__child_transforms` | 2 |

Small frame fixtures cover targeted inputs, not each widget's full
semantics, async, or geometry surface. Ordinary nested SafeArea and
stream delivery are controls for the independent intentionally red
maintained-padding and reentrancy tests.
