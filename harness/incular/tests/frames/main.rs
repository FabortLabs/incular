//! Frame-by-frame parity against Flutter.
//!
//! Runs every scenario, writes its trace to `<dir>/frames/incular`, compares
//! it with the Flutter trace of the same name in `<dir>/frames/flutter`
//! (written by `flutter test test/frames` in `harness/flutter`), and writes
//! `<dir>/frames/report.md`. `<dir>` is `$INCULAR_PARITY_DIR` or
//! `harness/out`.
//!
//! Runs only when `INCULAR_PARITY=1`. `PARITY_SCENARIO=<substring>` limits
//! the run to matching scenarios; `PARITY_STRICT=1` fails on a divergence or
//! a missing Flutter trace. See `harness/README.md`.

mod anim_compose;
mod anim_core;
mod anim_curves;
mod focus_input;
mod gestures;
mod host;
mod implicit;
mod layout;
mod misc;
mod navigation;
mod paint;
mod physics;
mod samples;
mod scroll;
mod semantics;
mod slivers;
mod support;
mod switchers;
mod text;
mod transitions;

use incular_parity_harness::{Scenario, Tolerances, compare, output_dir, report, run};

fn scenarios() -> Vec<Scenario> {
    let mut all = Vec::new();
    all.extend(anim_core::scenarios());
    all.extend(anim_curves::scenarios());
    all.extend(anim_compose::scenarios());
    all.extend(physics::scenarios());
    all.extend(implicit::scenarios());
    all.extend(transitions::scenarios());
    all.extend(switchers::scenarios());
    all.extend(navigation::scenarios());
    all.extend(scroll::scenarios());
    all.extend(slivers::scenarios());
    all.extend(gestures::scenarios());
    all.extend(layout::scenarios());
    all.extend(paint::scenarios());
    all.extend(text::scenarios());
    all.extend(focus_input::scenarios());
    all.extend(semantics::scenarios());
    all.extend(misc::scenarios());
    all.extend(support::scenarios());
    all.extend(samples::scenarios());
    all
}

fn main() {
    if std::env::var("INCULAR_PARITY").as_deref() != Ok("1") {
        eprintln!("frames: skipped; set INCULAR_PARITY=1 (see harness/README.md)");
        return;
    }
    let filter = std::env::var("PARITY_SCENARIO").unwrap_or_default();
    let strict = std::env::var("PARITY_STRICT").as_deref() == Ok("1");
    let tolerances = Tolerances::from_env();
    let flutter_dir = output_dir().join("frames").join("flutter");
    let mut comparisons = Vec::new();
    let mut missing = Vec::new();
    let all = scenarios();
    let mut names = std::collections::BTreeSet::new();
    for scenario in &all {
        assert!(
            names.insert(scenario.0),
            "duplicate scenario {}",
            scenario.0
        );
    }
    for scenario in all.iter().filter(|s| s.0.contains(&filter)) {
        let ours = run(scenario);
        let theirs = flutter_dir.join(format!("{}.json", scenario.0));
        if !theirs.exists() {
            missing.push(scenario.0);
            continue;
        }
        comparisons.push(compare(scenario.0, &theirs, &ours, &tolerances));
    }
    let mut text = report(&comparisons, &tolerances);
    if !missing.is_empty() {
        text.push_str("## Missing Flutter traces\n\n");
        for name in &missing {
            text.push_str(&format!("- {name}\n"));
        }
    }
    // A filtered run writes its own report, so concurrent runs of
    // different scenario sets do not overwrite each other.
    let file = if filter.is_empty() {
        "report.md".to_owned()
    } else {
        let safe: String = filter
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        format!("report-{safe}.md")
    };
    let path = output_dir().join("frames").join(file);
    std::fs::write(&path, &text).expect("write report");
    for comparison in &comparisons {
        let status = if comparison.matches() {
            "match"
        } else {
            "DIVERGES"
        };
        eprintln!("{:<40} {status}", comparison.scenario);
        for problem in &comparison.structural {
            eprintln!("    structural: {problem}");
        }
        for d in &comparison.divergences {
            eprintln!(
                "    {} first at frame {} ({:.1} ms): flutter {} incular {} ({} frames, max {:.4})",
                d.what,
                d.frame,
                d.t_us as f64 / 1000.,
                d.flutter,
                d.incular,
                d.frames,
                d.max_diff
            );
        }
    }
    for name in &missing {
        eprintln!("{name:<40} missing Flutter trace");
    }
    eprintln!("report: {}", path.display());
    let diverged = comparisons.iter().filter(|c| !c.matches()).count();
    assert!(
        !strict || (diverged == 0 && missing.is_empty()),
        "{diverged} scenarios diverge, {} lack a Flutter trace",
        missing.len()
    );
}
