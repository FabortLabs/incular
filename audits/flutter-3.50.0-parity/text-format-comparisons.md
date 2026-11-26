# Paired slice: text shaping and editing formatters

Flutter 3.50.0 `ab2912035e` vs Incular implementation `c647cb85`.
All three Flutter checks passed; all three Incular parity assertions failed
after their controls.

| ID | Case | Flutter | Incular |
| --- | --- | --- | --- |
| C28 | Measure "a a" with word spacing +12 | width increases by 12 px (60→72) | no change (18.081055→18.081055), both warm cache and fresh engine |
| Control | Same path with letter spacing +2 | width increases 6 px | width increases 6 px |
| C29 | Digits-only filter for 1a2 (caret after a) and 1😀2 (caret after emoji) | Both become 12 with caret at 1 | Both become 12, but raw formatter offsets are 2 and 5 |
| C30 | Enforced max length 3, start abc at caret 1, try aXbc | Rejects edit; abc and caret 1 retained | aXb, caret 2; old suffix lost |
| Control | In-bound replacement abc→aYc | aYc | aYc |

C28 compares within-framework differences because default font size and font
selection differ. Dart used TextPainter/paragraph and Rust TextEngine/Parley.
Letter spacing proves the probe is sensitive to a spacing change. A fresh
engine still ignores word spacing, so a cache-key change alone is insufficient.
Flutter `painting/text_style.dart:1368`; Incular
`crates/incular-text/src/engine.rs:495–549,2003–2034`.

C29 starts after the same visible glyph (UTF-16 offset 3 vs UTF-8 offset 5 for
emoji), then compares offsets only after the result is identical ASCII. This
is a remapping failure, not an expectation that byte encodings match. The raw
Rust formatter returns 5 even though the trimmed text has two characters;
the controller clamps afterward and still cannot restore the correct position.
C30 proves loss of an existing character, not refusal of a new character.
Flutter `services/text_formatter.dart:392–423,575–586`; Incular
`crates/incular-widgets/src/forms.rs:1401–1423,1467–1498`;
`crates/incular-text/src/editing.rs:1047–1048`.

## Focused fixtures

Use the pinned Flutter scratch package from [paired comparisons](focused-comparisons.md).
Run `flutter test --no-pub test/word_spacing_slice_test.dart` and
`flutter test --no-pub test/formatter_slice_test.dart` separately.

```dart
import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
void main(){
  test('word spacing increases width independently of letter spacing',(){
    double width({double word=0,double letter=0}){
      final p=TextPainter(text:TextSpan(text:'a a',style:TextStyle(fontSize:20,wordSpacing:word,letterSpacing:letter)),textDirection:TextDirection.ltr)..layout();
      final w=p.width;p.dispose();return w;
    }
    final base=width(),letter=width(letter:2),word=width(word:12);
    print('base=$base letter=$letter word=$word');
    expect(letter,greaterThan(base));
    expect(word-base,closeTo(12,0.01));
  });
}
```

```dart
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
TextEditingValue value(String text,int caret)=>TextEditingValue(text:text,selection:TextSelection.collapsed(offset:caret));
void main(){
  test('digits filter remaps the caret across removed characters',(){
    final f=FilteringTextInputFormatter.digitsOnly;
    for(final (text,caret) in [('1a2',2),('1😀2',3)]){
      final got=f.formatEditUpdate(value('12',1),value(text,caret));
      expect(got.text,'12');
      expect(got.selection.extentOffset,1);
    }
  });
  test('already full enforced field rejects middle insert without losing its suffix',(){
    final f=LengthLimitingTextInputFormatter(3,maxLengthEnforcement:MaxLengthEnforcement.enforced);
    final old=value('abc',1);
    expect(f.formatEditUpdate(old,value('aYc',2)).text,'aYc');
    expect(f.formatEditUpdate(old,value('aXbc',2)),old);
  });
}
```

Use temporary tests `crates/incular-text/tests/audit_word_spacing_tmp.rs`
and `crates/incular-widgets/tests/audit_formatter_tmp.rs`, running only
the exact targets. The temporary source targets were removed; raw logs
and source copies are in `.tmp/flutter350-full-audit/`.

```rust
use incular_text::{TextAlign,TextEngine,TextStyle};
#[test]
fn word_spacing_increases_width_warm_and_cold(){
    let plain=TextStyle::default();
    let adjusted=plain.clone().word_spacing(Some(12.));
    let letters=plain.clone().letter_spacing(2.);
    let mut warm=TextEngine::new();
    let layout=|engine:&mut TextEngine,s:&TextStyle| engine.layout("a a",s,None,TextAlign::Start).lines[0].width;
    let base=layout(&mut warm,&plain);
    let with_letter=layout(&mut warm,&letters);
    let with_word=layout(&mut warm,&adjusted);
    let cold_word=layout(&mut TextEngine::new(),&adjusted);
    eprintln!("base={base} letter={with_letter} warm_word={with_word} cold_word={cold_word}");
    assert!(with_letter>base);
    assert!((with_word-base-12.).abs()<0.01&&(cold_word-base-12.).abs()<0.01);
}
```

```rust
use incular_text::{TextEditingValue,TextSelection};
use incular_widgets::{FilteringTextInputFormatter,LengthLimitingTextInputFormatter,TextInputFormatter};
fn value(text:&str,caret:usize)->TextEditingValue{
    TextEditingValue{text:text.into(),selection:TextSelection::collapsed(caret),composing:None}
}
#[test]
fn digits_filter_remaps_caret_across_removed_characters(){
    let f=FilteringTextInputFormatter::digits_only();
    let actual=[("1a2",2),("1😀2",5)].map(|(s,n)|{
        let got=f.format_edit_update(&value("12",1),&value(s,n));
        (got.text,got.selection.extent)
    });
    assert_eq!(actual,[("12".to_owned(),1),("12".to_owned(),1)]);
}
#[test]
fn full_enforced_field_rejects_middle_insert_without_losing_suffix(){
    let f=LengthLimitingTextInputFormatter::new(3);
    let old=value("abc",1);
    assert_eq!(f.format_edit_update(&old,&value("aYc",2)).text,"aYc");
    let got=f.format_edit_update(&old,&value("aXbc",2));
    assert_eq!((got.text,got.selection.extent),(old.text,old.selection.extent));
}
```

## C28 word spacing repair

The original pinned reference still measures widths 60, 66 and 72
for base, letter spacing 2 and word spacing 12. Incular now forwards
word spacing into shaping and includes the effective spacing in
layout, width-reuse, and document cache identities. The already
committed regression passes on both a warm and fresh engine.

A second paired authored probe measures hard-break lines after
changing spacing to +9, −3, 0 and +17. Both engines apply the
change once per internal space on each line. Native checks also
alternate the styles on one warm engine and restore the original
width. Eight focused Rust tests passed. These are authored probes,
not additional upstream registration credit.
