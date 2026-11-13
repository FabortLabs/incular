import 'dart:async';

import 'package:flutter/widgets.dart';

import 'recorder.dart';

const _blue = Color(0xFF2196F3);
const _orange = Color(0xFFFF5722);
const _green = Color(0xFF4CAF50);
const _red = Color(0xFFF44336);

Widget _box(Color color, double width, double height) => SizedBox(
  width: width,
  height: height,
  child: ColoredBox(color: color),
);

// Flutter Placeholder builds LimitedBox(400x400) around an infinite CustomPaint;
// this twin keeps that LimitedBox and fills the same constraints with
// SizedBox.expand, which Incular can express (CustomPaint rejects infinite sizes).
Widget _fallbackFill(Widget child) => LimitedBox(
  maxWidth: 400,
  maxHeight: 400,
  child: child,
);

Widget _fillBox(Widget child) => ColoredBox(color: _blue, child: child);

double _connectionCode(ConnectionState state) => switch (state) {
  ConnectionState.none => 0,
  ConnectionState.waiting => 1,
  ConnectionState.active => 2,
  ConnectionState.done => 3,
};

enum _PairAspect { leading, trailing }

class _PairModel extends InheritedModel<_PairAspect> {
  const _PairModel({
    required this.leading,
    required this.trailing,
    required super.child,
  });

  final double leading;
  final double trailing;

  @override
  bool updateShouldNotify(_PairModel oldWidget) =>
      leading != oldWidget.leading || trailing != oldWidget.trailing;

  @override
  bool updateShouldNotifyDependent(
    _PairModel oldWidget,
    Set<_PairAspect> dependencies,
  ) =>
      (leading != oldWidget.leading &&
          dependencies.contains(_PairAspect.leading)) ||
      (trailing != oldWidget.trailing &&
          dependencies.contains(_PairAspect.trailing));
}

void main() {
  frameScenario('misc__expansible_height_animation', (h) async {
    final controller = ExpansibleController();
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 120,
          child: h.probe(
            'expansible',
            Expansible(
              controller: controller,
              headerBuilder: (_, _) => _box(_blue, 120, 32),
              bodyBuilder: (_, _) => _box(_orange, 120, 80),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    controller.expand();
    await h.frames(14);
  });

  frameScenario('misc__rotated_box_layout_size', (h) async {
    await h.mount(
      (_) => Center(
        child: h.probe(
          'rotated_box',
          RotatedBox(quarterTurns: 1, child: _box(_green, 80, 40)),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('misc__spacer_flex_shares', (h) async {
    await h.mount(
      (_) => SizedBox(
        width: 300,
        height: 32,
        child: Row(
          children: <Widget>[
            h.probe('spacer_first', _box(_blue, 40, 32)),
            const Spacer(),
            h.probe('spacer_middle', _box(_orange, 40, 32)),
            const Spacer(flex: 2),
            h.probe('spacer_last', _box(_green, 40, 32)),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('misc__inherited_model_aspects', (h) async {
    var leadingBuilds = 0;
    var trailingBuilds = 0;
    h.value('leading_builds', () => leadingBuilds.toDouble());
    h.value('trailing_builds', () => trailingBuilds.toDouble());
    final child = Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Builder(
          builder: (context) {
            leadingBuilds++;
            final model = InheritedModel.inheritFrom<_PairModel>(
              context,
              aspect: _PairAspect.leading,
            )!;
            return h.probe('inherited_leading', _box(_blue, model.leading, 24));
          },
        ),
        Builder(
          builder: (context) {
            trailingBuilds++;
            final model = InheritedModel.inheritFrom<_PairModel>(
              context,
              aspect: _PairAspect.trailing,
            )!;
            return h.probe(
              'inherited_trailing',
              _box(_orange, model.trailing, 24),
            );
          },
        ),
      ],
    );
    await h.mount(
      (phase) => _PairModel(
        leading: phase == 0 ? 60 : 90,
        trailing: phase == 2 ? 120 : 70,
        child: child,
      ),
    );
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
    h.setPhase(2);
    await h.frames(1);
  });

  frameScenario('misc__media_query_size_aspect', (h) async {
    var builderRuns = 0;
    h.value('builder_runs', () => builderRuns.toDouble());
    final child = Center(
      child: Builder(
        builder: (context) {
          builderRuns++;
          return h.probe(
            'media_size',
            _box(_green, MediaQuery.sizeOf(context).width / 2, 24),
          );
        },
      ),
    );
    await h.mount(
      (phase) => MediaQuery(
        data: MediaQueryData(
          size: const Size(320, 200),
          devicePixelRatio: phase == 0 ? 1 : 2,
        ),
        child: child,
      ),
    );
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
  });

  frameScenario('misc__form_save_invalid_field', (h) async {
    final formKey = GlobalKey<FormState>();
    var isValid = -1.0;
    var savedCount = 0;
    h.value('form_valid', () => isValid);
    h.value('saved_count', () => savedCount.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: h.probe(
          'form',
          Form(
            key: formKey,
            child: FormField<String>(
              initialValue: '',
              validator: (value) => value!.isEmpty ? 'required' : null,
              onSaved: (_) => savedCount++,
              builder: (_) => _box(_blue, 160, 32),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    isValid = formKey.currentState!.validate() ? 1 : 0;
    await h.frames(1);
    formKey.currentState!.save();
    await h.frames(1);
  });

  frameScenario('misc__form_field_lifecycle', (h) async {
    final fieldKey = GlobalKey<FormFieldState<String>>();
    var fieldValueLength = -1.0;
    var fieldHasError = 0.0;
    var savedValueLength = -1.0;
    h.value('field_value_length', () => fieldValueLength);
    h.value('field_has_error', () => fieldHasError);
    h.value('saved_value_length', () => savedValueLength);
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: FormField<String>(
          key: fieldKey,
          initialValue: '',
          autovalidateMode: AutovalidateMode.onUserInteraction,
          validator: (value) => value!.isEmpty ? 'required' : null,
          onSaved: (value) => savedValueLength = value!.length.toDouble(),
          builder: (state) {
            fieldValueLength = state.value!.length.toDouble();
            fieldHasError = state.hasError ? 1 : 0;
            return h.probe(
              'form_field',
              _box(
                state.hasError ? _red : _blue,
                120 + state.value!.length * 4,
                32,
              ),
            );
          },
        ),
      ),
    );
    final state = fieldKey.currentState!;
    await h.frames(1);
    state.didChange('');
    await h.frames(1);
    state.didChange('ready');
    await h.frames(1);
    state.validate();
    await h.frames(1);
    state.save();
    await h.frames(1);
    state.reset();
    await h.frames(1);
  });

  frameScenario('misc__future_builder_keyed_reorder', (h) async {
    final firstCompleter = Completer<String>();
    final secondCompleter = Completer<String>();
    var firstState = -1.0;
    var firstLength = -1.0;
    var secondState = -1.0;
    var secondLength = -1.0;
    h.value('first_state', () => firstState);
    h.value('first_length', () => firstLength);
    h.value('second_state', () => secondState);
    h.value('second_length', () => secondLength);

    Widget first() => KeyedSubtree(
      key: const ValueKey<String>('first'),
      child: FutureBuilder<String>(
        future: firstCompleter.future,
        initialData: 'f0',
        builder: (context, snapshot) {
          firstState = _connectionCode(snapshot.connectionState);
          firstLength = snapshot.data!.length.toDouble();
          return h.probe(
            'future_first',
            _box(_blue, 40 + snapshot.data!.length * 4, 24),
          );
        },
      ),
    );

    Widget second() => KeyedSubtree(
      key: const ValueKey<String>('second'),
      child: FutureBuilder<String>(
        future: secondCompleter.future,
        initialData: 's0',
        builder: (context, snapshot) {
          secondState = _connectionCode(snapshot.connectionState);
          secondLength = snapshot.data!.length.toDouble();
          return h.probe(
            'future_second',
            _box(_orange, 40 + snapshot.data!.length * 4, 24),
          );
        },
      ),
    );

    await h.mount(
      (phase) => Column(
        mainAxisSize: MainAxisSize.min,
        children: phase == 0
            ? <Widget>[first(), second()]
            : <Widget>[second(), first()],
      ),
    );
    await h.frames(1);
    firstCompleter.complete('alpha');
    secondCompleter.complete('beta');
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
  });

  frameScenario('misc__stream_builder_events', (h) async {
    final controller = StreamController<String>.broadcast();
    var connection = -1.0;
    var dataLength = -1.0;
    var hasError = 0.0;
    h.value('stream_state', () => connection);
    h.value('stream_data_length', () => dataLength);
    h.value('stream_has_error', () => hasError);
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: StreamBuilder<String>(
          stream: controller.stream,
          initialData: 'seed',
          builder: (context, snapshot) {
            connection = _connectionCode(snapshot.connectionState);
            dataLength = snapshot.data?.length.toDouble() ?? 0;
            hasError = snapshot.hasError ? 1 : 0;
            return h.probe(
              'stream_builder',
              _box(snapshot.hasError ? _red : _green, 40 + dataLength * 4, 24),
            );
          },
        ),
      ),
    );
    await h.frames(1);
    controller.add('one');
    await h.frames(1);
    controller.addError('bad');
    await h.frames(1);
    unawaited(controller.close());
    await h.frames(1);
  });

  frameScenario('misc__safe_area_nested_consumption', (h) async {
    var childTopPadding = 0.0;
    h.value('child_top_padding', () => childTopPadding);
    await h.mount(
      (phase) => MediaQuery(
        data: MediaQueryData(
          size: const Size(800, 600),
          padding: phase == 0
              ? EdgeInsets.zero
              : const EdgeInsets.only(top: 20),
          viewPadding: phase == 0
              ? EdgeInsets.zero
              : const EdgeInsets.only(top: 20),
        ),
        child: Center(
          child: h.probe(
            'safe_area',
            SafeArea(
              child: SafeArea(
                child: Builder(
                  builder: (context) {
                    childTopPadding = MediaQuery.paddingOf(context).top;
                    return _box(_blue, 100, 50);
                  },
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
  });

  frameScenario('misc__directionality_row_order', (h) async {
    await h.mount(
      (phase) => Directionality(
        textDirection: phase == 0 ? TextDirection.ltr : TextDirection.rtl,
        child: Center(
          child: SizedBox(
            width: 300,
            height: 32,
            child: Row(
              children: <Widget>[
                h.probe('direction_first', _box(_blue, 40, 32)),
                h.probe('direction_second', _box(_orange, 40, 32)),
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
  });

  frameScenario('misc__repeating_builder_reverse_pause', (h) async {
    var repeatValue = 0.0;
    h.value('repeat_value', () => repeatValue);
    await h.mount(
      (phase) => Align(
        alignment: Alignment.topLeft,
        child: h.probe(
          'repeat',
          RepeatingAnimationBuilder<double>(
            animatable: Tween<double>(begin: 0, end: 100),
            duration: const Duration(milliseconds: 200),
            repeatMode: RepeatMode.reverse,
            paused: phase == 1,
            builder: (context, value, child) {
              repeatValue = value;
              return _box(_blue, value, 10);
            },
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(5);
    h.setPhase(2);
    await h.frames(15);
  });

  frameScenario('misc__orientation_builder_constraints', (h) async {
    var orientation = 0.0;
    h.value('orientation', () => orientation);
    await h.mount(
      (phase) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: phase == 0 ? 300 : 500,
          height: phase == 0 ? 500 : 300,
          child: OrientationBuilder(
            builder: (context, value) {
              orientation = value == Orientation.landscape ? 1 : 0;
              return h.probe('orientation_box', _box(_blue, 40, 40));
            },
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(2);
  });

  frameScenario('misc__placeholder_fallback_size', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _fallbackFill(h.probe('placeholder_row', _fillBox(SizedBox.expand()))),
              ],
            ),
            _fallbackFill(h.probe('placeholder_bounded', _fillBox(SizedBox(width: 120, height: 80, child: SizedBox.expand())))),
          ],
        ),
      ),
    );
    await h.frames(1);
  });
}
