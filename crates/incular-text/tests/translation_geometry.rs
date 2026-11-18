use incular_text::{TextAlign, TextEngine, TextLayoutOptions, TextStyle};
use std::fmt::Write;

#[test]
fn translating_successive_paragraphs_preserves_selection_and_carets() {
    let mut reused = TextEngine::new();
    let mut all = String::new();
    for text in [
        "abcabcXYZ",
        "123456789012345",
        "1234567890123456",
        "12345678901234567",
        "abc אבג office ffi مرحبا 1234567890 👩‍💻",
        "office ffi",
        "abc אבג 12",
        "مرحبا xyz",
        "a\u{301}b",
        "a👩\u{200d}💻b",
        "क्\u{200d}ष",
        "one\n\ntwo",
        "",
        "ไทย 中文",
    ] {
        for width in [22., 320.] {
            let options = TextLayoutOptions::new(Some(width), TextAlign::Start);
            let snapshot = |engine: &mut TextEngine| {
                let layout = if text == "abcabcXYZ" {
                    let runs = [(0..3, 14.), (3..6, 18.), (6..9, 14.)].map(|(range, size)| {
                        incular_text::StyledTextRun {
                            text: text[range.clone()].into(),
                            style: TextStyle::default().font_size(size),
                            range,
                        }
                    });
                    let layout = engine.layout_runs_with_options(text, &runs, options);
                    assert_eq!(
                        layout
                            .lines
                            .iter()
                            .flat_map(|l| l.glyphs.iter().map(|g| g.cluster))
                            .collect::<Vec<_>>(),
                        (0..9).collect::<Vec<_>>()
                    );
                    layout
                } else {
                    engine.layout_with_options(text, &TextStyle::default(), options)
                };
                let mut geometry = String::new();
                for (i, line) in layout.lines.iter().enumerate() {
                    writeln!(
                        geometry,
                        "line {} {} {}",
                        line.start,
                        line.end,
                        line.baseline.to_bits()
                    )
                    .unwrap();
                    for g in line.glyphs.iter() {
                        writeln!(
                            geometry,
                            "g {} {} {} {} {}",
                            g.id,
                            g.cluster,
                            g.offset.x.to_bits(),
                            g.offset.y.to_bits(),
                            g.advance.to_bits()
                        )
                        .unwrap();
                    }
                    for c in layout.line_caret_positions(i) {
                        writeln!(
                            geometry,
                            "c {} {:?} {} {}",
                            c.offset,
                            c.affinity,
                            c.line,
                            c.x.to_bits()
                        )
                        .unwrap();
                    }
                    for start in 0..=text.len() {
                        for end in start..=text.len() {
                            let spans: Vec<_> = line
                                .selection_spans(start, end)
                                .into_iter()
                                .map(|(l, r)| (l.to_bits(), r.to_bits()))
                                .collect();
                            writeln!(geometry, "{start}..{end} {spans:?}").unwrap();
                        }
                    }
                }
                geometry
            };
            let actual = snapshot(&mut reused);
            assert_eq!(
                actual,
                snapshot(&mut TextEngine::new()),
                "{text:?} width {width}"
            );
            all.push_str(&actual);
        }
    }
    if let Some(path) = std::env::var_os("INCULAR_TRANSLATION_SNAPSHOT") {
        std::fs::write(path, all).unwrap();
    }
}
