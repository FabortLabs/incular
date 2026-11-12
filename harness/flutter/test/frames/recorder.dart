// Frame-by-frame recorder: the Flutter side of the Incular frame parity
// harness. The Incular side is harness/incular/src/lib.rs; both write the
// same trace format and a scenario must be written identically on both
// sides (same widgets, same probes, same timeline).
//
// A scenario mounts a phase-driven scene, then advances a fake clock frame
// by frame. After every frame each probe (a subtree wrapped with
// `h.probe(name, child)`) records its global rect, full 2D transform, size,
// accumulated opacity and first fill color, plus any named scalar values
// the scenario samples (`h.value`). Input is stamped with the fake clock.
//
// Run: flutter test test/frames        (from harness/flutter)
// Traces go to $PARITY_FRAMES_OUT or ../out/frames/flutter/<scenario>.json.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

export 'package:flutter/services.dart' show LogicalKeyboardKey;
export 'package:flutter_test/flutter_test.dart';

/// The logical surface every scenario runs in (device pixel ratio 1).
const Size kSurface = Size(800, 600);

/// The frame interval scenarios advance by unless they pump explicitly.
const Duration kInterval = Duration(milliseconds: 16);

/// The text style at the root of every scene. Text must use Inter, which
/// both sides load from the same file, or glyph metrics differ.
const TextStyle kRootTextStyle = TextStyle(
  fontFamily: 'Inter',
  fontSize: 14,
  color: Color(0xFF000000),
);

bool _fontsLoaded = false;

Future<void> _loadFonts() async {
  if (_fontsLoaded) return;
  _fontsLoaded = true;
  final bytes = File('fonts/Inter-Regular.ttf').readAsBytesSync();
  final loader = FontLoader('Inter')
    ..addFont(Future.value(ByteData.view(bytes.buffer)));
  await loader.load();
}

/// Registers a frame scenario. `body` mounts a scene and drives the clock.
void frameScenario(String name, Future<void> Function(FrameHarness h) body) {
  setUpAll(_loadFonts);
  testWidgets(name, (tester) async {
    tester.view.physicalSize = kSurface;
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    final harness = FrameHarness._(tester, name);
    await body(harness);
    harness._write();
  });
}

/// The key prefix that marks a probe. A probe keys its subtree exactly as
/// Incular's `KeyedSubtree` does, so reconciliation matches on both sides;
/// probe inside, not around, a widget that needs its own key.
const String kProbePrefix = 'probe:';

class FrameHarness {
  FrameHarness._(this.tester, this.name);

  final WidgetTester tester;
  final String name;
  final ValueNotifier<int> _phase = ValueNotifier<int>(0);
  final List<Map<String, Object?>> _frames = <Map<String, Object?>>[];
  final Map<String, double Function()> _values = <String, double Function()>{};
  Duration _elapsed = Duration.zero;
  TestGesture? _touch;
  TestGesture? _secondTouch;
  TestGesture? _mouse;

  /// Marks `child` as the probe `name`.
  Widget probe(String name, Widget child) =>
      KeyedSubtree(key: ValueKey<String>('$kProbePrefix$name'), child: child);

  /// The current phase; `build` sees it and rebuilds when it changes.
  int get phase => _phase.value;

  /// The time since mount.
  Duration get elapsed => _elapsed;

  /// Samples `read` after every frame as the value `name`.
  void value(String name, double Function() read) => _values[name] = read;

  /// Mounts the scene at time zero and records frame 0.
  Future<void> mount(Widget Function(int phase) build) async {
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: DefaultTextStyle(
          style: kRootTextStyle,
          child: ValueListenableBuilder<int>(
            valueListenable: _phase,
            builder: (context, phase, _) => build(phase),
          ),
        ),
      ),
    );
    _record();
  }

  /// Changes the phase. The scene rebuilds in the next frame.
  void setPhase(int phase) => _phase.value = phase;

  /// Advances `count` frames of `interval`, recording each.
  Future<void> frames(int count, {Duration interval = kInterval}) async {
    for (var i = 0; i < count; i++) {
      await pump(interval);
    }
  }

  /// Advances one frame of `duration` and records it.
  Future<void> pump(Duration duration) async {
    _elapsed += duration;
    await tester.pump(duration);
    _record();
  }

  /// The center of a probe's global rect, for input.
  Offset center(String probe) {
    final element = _probes()[probe];
    if (element == null) throw StateError('no probe $probe');
    final geometry = _geometry(element.renderObject!);
    return (geometry['rectObject']! as Rect).center;
  }

  /// A touch pointer goes down at `position` (Flutter's default test kind).
  Future<void> down(Offset position) async {
    _touch = await tester.createGesture(kind: PointerDeviceKind.touch);
    await _touch!.down(position, timeStamp: _elapsed);
  }

  /// The touch pointer moves to `position`.
  Future<void> move(Offset position) async =>
      _touch!.moveTo(position, timeStamp: _elapsed);

  /// The touch pointer lifts.
  Future<void> up() async {
    await _touch!.up(timeStamp: _elapsed);
    _touch = null;
  }

  /// A second independent touch pointer, for pinch and rotation scenarios.
  Future<void> secondDown(Offset position) async {
    if (_secondTouch != null) throw StateError('second touch already down');
    _secondTouch = await tester.createGesture(kind: PointerDeviceKind.touch);
    await _secondTouch!.down(position, timeStamp: _elapsed);
  }

  Future<void> secondMove(Offset position) async =>
      _secondTouch!.moveTo(position, timeStamp: _elapsed);

  Future<void> secondUp() async {
    await _secondTouch!.up(timeStamp: _elapsed);
    _secondTouch = null;
  }

  Future<void> secondCancel() async {
    await _secondTouch!.cancel(timeStamp: _elapsed);
    _secondTouch = null;
  }

  /// The touch pointer is canceled.
  Future<void> cancel() async {
    await _touch!.cancel(timeStamp: _elapsed);
    _touch = null;
  }

  /// A mouse pointer hovers at `position`.
  Future<void> hover(Offset position) async {
    if (_mouse == null) {
      _mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await _mouse!.addPointer(location: position, timeStamp: _elapsed);
    }
    await _mouse!.moveTo(position, timeStamp: _elapsed);
  }

  /// A mouse wheel scroll of `delta` logical pixels at `position`
  /// (positive y scrolls content up, as Flutter's `scrollDelta`).
  Future<void> wheel(Offset position, Offset delta) async {
    await hover(position);
    await tester.sendEventToBinding(PointerScrollEvent(
      timeStamp: _elapsed,
      position: position,
      scrollDelta: delta,
      kind: PointerDeviceKind.mouse,
    ));
  }

  /// A key press and release (a logical key, as Flutter's
  /// `sendKeyEvent`).
  Future<void> key(LogicalKeyboardKey key) => tester.sendKeyEvent(key);

  Map<String, Element> _probes() {
    final found = <String, Element>{};
    void visit(Element element) {
      final key = element.widget.key;
      if (key is ValueKey<String> && key.value.startsWith(kProbePrefix)) {
        found.putIfAbsent(key.value.substring(kProbePrefix.length), () => element);
      }
      element.visitChildren(visit);
    }

    WidgetsBinding.instance.rootElement?.visitChildren(visit);
    return found;
  }

  void _record() {
    final probes = <String, Object?>{};
    final found = _probes();
    for (final entry in found.entries) {
      final render = entry.value.renderObject;
      if (render == null || !render.attached) continue;
      final geometry = _geometry(render)..remove('rectObject');
      geometry['opacity'] = _opacity(render);
      geometry['color'] = _color(entry.value);
      probes[entry.key] = geometry;
    }
    _frames.add(<String, Object?>{
      't_us': _elapsed.inMicroseconds,
      'probes': probes,
      'values': <String, Object?>{
        for (final entry in _values.entries) entry.key: entry.value(),
      },
    });
  }

  static Map<String, Object?> _geometry(RenderObject render) {
    final transform = render.getTransformTo(null);
    final bounds = render.paintBounds;
    final rect = MatrixUtils.transformRect(transform, bounds);
    final s = transform.storage;
    return <String, Object?>{
      'rect': <double>[rect.left, rect.top, rect.right, rect.bottom],
      'rectObject': rect,
      'transform': <double>[s[0], s[1], s[4], s[5], s[12], s[13]],
      'size': <double>[bounds.width, bounds.height],
    };
  }

  /// The product of every opacity between the probe and the root.
  static double _opacity(RenderObject render) {
    var opacity = 1.0;
    RenderObject? current = render;
    while (current != null) {
      if (current is RenderOpacity) {
        opacity *= current.opacity;
      } else if (current is RenderAnimatedOpacity) {
        opacity *= current.opacity.value;
      } else if (current is RenderSliverOpacity) {
        opacity *= current.opacity;
      } else if (current is RenderSliverAnimatedOpacity) {
        opacity *= current.opacity.value;
      } else if (current is RenderOffstage && current.offstage) {
        opacity = 0;
      }
      current = current.parent;
    }
    return opacity;
  }

  /// The first solid fill at or below the probe: a `ColoredBox` color or a
  /// `DecoratedBox`'s `BoxDecoration.color`, as `[a, r, g, b]`.
  static List<int>? _color(Element element) {
    Color? found;
    void visit(Element current) {
      if (found != null) return;
      final widget = current.widget;
      if (widget is ColoredBox) {
        found = widget.color;
      } else if (widget is PhysicalModel) {
        found = widget.color;
      } else if (widget is PhysicalShape) {
        found = widget.color;
      } else if (widget is DecoratedBox) {
        final decoration = widget.decoration;
        if (decoration is BoxDecoration && decoration.color != null) {
          found = decoration.color;
        }
      }
      if (found == null) current.visitChildren(visit);
    }

    visit(element);
    final color = found;
    if (color == null) return null;
    int channel(double value) => (value * 255).round().clamp(0, 255);
    return <int>[channel(color.a), channel(color.r), channel(color.g), channel(color.b)];
  }

  void _write() {
    final dir = Directory(
      Platform.environment['PARITY_FRAMES_OUT'] ?? '../out/frames/flutter',
    )..createSync(recursive: true);
    // One frame per line, so traces diff and grep by frame.
    final frames = _frames.map(jsonEncode).join(',\n');
    File('${dir.path}/$name.json').writeAsStringSync(
      '{"scenario": ${jsonEncode(name)}, "frames": [\n$frames\n]}\n',
    );
  }
}
