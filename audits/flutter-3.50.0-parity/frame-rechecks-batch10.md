# Focused frame rechecks, batch ten

Fresh strict frames: Flutter 3.50.0 `ab2912035e` and Incular
`c647cb85`. Cumulative: **55/253 existing registrations (21.74%),
50 match, five differ**.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `slivers__custom_scroll_view__cache_extent` | match | 3 |
| `slivers__grid_and_fill_viewport__geometry` | match | 4 |
| `scroll__single_child__keep_offset_page_storage` | match | 9 |
| `scroll__page_view__fraction_initial_page` | match | 4 |
| `semantics__live_region` | match | 2 |

The cache and page-storage results cover the recorded fixture inputs, not
all retention/restore paths. The live-region result covers the harness
semantics probes, not native assistive technology. Scratch batch:
`20261124-152654-896`; [focused runner](run-focused-frames.ps1).
