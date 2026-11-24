# Paired slice: synchronous broadcast lifecycle

Incular implementation `c647cb85` vs the pinned Flutter 3.50.0 Dart 3.14.0
SDK's `StreamController.broadcast(sync:true)`. The synchronous mode is the
comparison because Incular's controller calls listeners inline. A Dart
single-subscription synchronous stream may buffer its own nested sends
and cannot model this two-listener test.

| ID | Scenario | Dart actual | Incular actual |
| --- | --- | --- | --- |
| C15 | Listener A cancels B on event 1 before B's turn | B receives nothing | B receives [1] |
| C16 | Listener A attempts to send event 2 while handling event 1 | StateError caught by A; B receives [1] | B receives [2,1] |
| C17 | Listener A attempts to send event 2 from its Done callback | StateError caught; both receive Done only | A receives [Done,data2], B receives [data2,Done] |
| Control | Ordinary two-listener delivery of 1, 2, then Done | Both receive both events and complete | Both receive both events and complete |

Incular upgrades weak listeners to a strong batch before invoking A and
sets the closed flag after Done dispatch
(`crates/incular-widgets/src/async_builders.rs:379–404,450–469`).
Pinned Dart SDK `lib/async/stream.dart:2156–2173`,
`stream_controller.dart:359–383`,
`broadcast_stream_controller.dart:225–239,266–275,325–339,368–394`.
Alternative Rust behavior may safely reject or queue reentrant adds, but
must document that choice; it must not reorder already-published events
or deliver data after Done. Error message strings were not compared.

Remaining: error events and listener mutation independently, pause/buffer
and transform ordering, widget subscriptions on rebuild, callback exceptions,
and asynchronous/native streams. These are custom probes, not closure of
upstream registrations.

## Focused reproduction

Place this in the pinned scratch Flutter package as
`test/stream_slice_test.dart`, and run `flutter test --no-pub test/stream_slice_test.dart`.
All four Dart tests passed.

```dart
import 'dart:async';
import 'package:flutter_test/flutter_test.dart';
void main() {
  test('ordinary broadcast and close control', () async {
    final c=StreamController<int>.broadcast(sync:true);
    final a=<int>[], b=<int>[];
    var done=0;
    c.stream.listen(a.add,onDone:()=>done++);
    c.stream.listen(b.add,onDone:()=>done++);
    c.add(1); c.add(2);
    await c.close();
    expect(a,[1,2]); expect(b,[1,2]); expect(done,2);
  });
  test('cancellation skips a listener whose turn has not arrived', () async {
    final c=StreamController<int>.broadcast(sync:true);
    final b=<int>[];
    late StreamSubscription<int> sub;
    c.stream.listen((_) {sub.cancel();});
    sub=c.stream.listen(b.add);
    c.add(1);
    expect(b,isEmpty);
    await c.close();
  });
  test('same-controller reentrant add does not overtake', () async {
    final c=StreamController<int>.broadcast(sync:true);
    final b=<int>[];
    var rejected=false;
    c.stream.listen((v){
      if(v==1){try {c.add(2);} on StateError {rejected=true;}}
    });
    c.stream.listen(b.add);
    c.add(1);
    expect(rejected,isTrue);
    expect(b,[1]);
    await c.close();
  });
  test('Done is terminal across listeners', () async {
    final c=StreamController<int>.broadcast(sync:true);
    final a=<String>[],b=<String>[];
    var rejected=false;
    c.stream.listen((v)=>a.add('data$v'),onDone:(){
      a.add('Done');
      try{c.add(2);}on StateError{rejected=true;}
    });
    c.stream.listen((v)=>b.add('data$v'),onDone:()=>b.add('Done'));
    await c.close();
    expect(rejected,isTrue);
    expect(a,['Done']); expect(b,['Done']);
  });
}
```

Place this in a temporary `crates/incular-widgets/tests/audit_stream_slice_tmp.rs`
target, and run `cargo test --locked -p incular-widgets --test audit_stream_slice_tmp`.
The positive control passed; the three parity assertions failed. The
temporary test target was removed.

```rust
use incular_widgets::{StreamEvent, UiStreamController};
use std::{cell::{Cell,RefCell},rc::Rc};
#[test]
fn ordinary_broadcast_and_close() {
    let c=UiStreamController::<i32,()>::new();
    let counter=Rc::new(Cell::new(0));
    let one=counter.clone();
    let two=counter.clone();
    let _a=c.stream().listen(move |_| one.set(one.get()+1));
    let _b=c.stream().listen(move |_| two.set(two.get()+1));
    c.add(1); c.add(2); c.close();
    assert_eq!(counter.get(),6);
}
#[test]
fn cancellation_skips_unreached_listener() {
    let c=UiStreamController::<i32,()>::new();
    let b=Rc::new(RefCell::new(None));
    let owner=b.clone();
    let _a=c.stream().listen(move |_| { owner.borrow_mut().take(); });
    let values=Rc::new(RefCell::new(Vec::new()));
    let data=values.clone();
    *b.borrow_mut()=Some(c.stream().listen(move |e| {
        if let StreamEvent::Data(v)=e {data.borrow_mut().push(*v);}
    }));
    c.add(1);
    assert!(values.borrow().is_empty(),"{:?}",values.borrow());
}
#[test]
fn reentrant_dispatch_does_not_overtake() {
    let c=Rc::new(UiStreamController::<i32,()>::new());
    let sender=c.clone();
    let _a=c.stream().listen(move |e| {
        if matches!(e,StreamEvent::Data(1)) { sender.add(2); }
    });
    let b=Rc::new(RefCell::new(Vec::new()));
    let dest=b.clone();
    let _b=c.stream().listen(move |e| { if let StreamEvent::Data(v)=e {dest.borrow_mut().push(*v);} });
    c.add(1);
    assert_eq!(*b.borrow(),[1]);
}
#[test]
fn done_is_terminal_across_listeners() {
    let c=Rc::new(UiStreamController::<i32,()>::new());
    let sender=c.clone();
    let a=Rc::new(RefCell::new(Vec::new()));
    let one=a.clone();
    let _a=c.stream().listen(move |e|match e {
        StreamEvent::Data(v)=>one.borrow_mut().push(format!("data{v}")),
        StreamEvent::Done=>{one.borrow_mut().push("Done".to_string());sender.add(2);}
        _=>{}
    });
    let b=Rc::new(RefCell::new(Vec::new()));
    let two=b.clone();
    let _b=c.stream().listen(move |e|match e {
        StreamEvent::Data(v)=>two.borrow_mut().push(format!("data{v}")),
        StreamEvent::Done=>two.borrow_mut().push("Done".to_string()),
        _=>{}
    });
    c.close();
    assert_eq!((&*a.borrow(),&*b.borrow()), (&vec!["Done".to_string()],&vec!["Done".to_string()]));
}
```
