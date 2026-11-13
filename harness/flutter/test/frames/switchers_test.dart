import 'package:flutter/widgets.dart';

import 'recorder.dart';

const _blue = Color(0xFF2196F3);
const _orange = Color(0xFFFF5722);
const _green = Color(0xFF4CAF50);
const _duration = Duration(milliseconds: 300);

Widget _box(Color color, double width, double height) => SizedBox(
      width: width,
      height: height,
      child: ColoredBox(color: color),
    );

void main() {
  frameScenario('switchers__switcher_overlapping_children', (h) async {
    await h.mount((phase) {
      final Widget child = switch (phase) {
        0 => h.probe('switch_a', _box(_blue, 80, 48)),
        1 => h.probe('switch_b', _box(_orange, 140, 64)),
        _ => h.probe('switch_c', _box(_green, 100, 80)),
      };
      return Center(
        child: AnimatedSwitcher(duration: _duration, child: child),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(5);
    h.setPhase(2);
    await h.frames(22);
  });

  frameScenario('switchers__cross_fade_reversed_different_sizes', (h) async {
    await h.mount((phase) {
      return Center(
        child: AnimatedCrossFade(
          firstChild: h.probe('cross_first', _box(_blue, 90, 40)),
          secondChild: h.probe('cross_second', _box(_orange, 150, 76)),
          crossFadeState: phase == 0 || phase == 2
              ? CrossFadeState.showFirst
              : CrossFadeState.showSecond,
          duration: _duration,
          firstCurve: Curves.linear,
          secondCurve: Curves.linear,
          sizeCurve: Curves.linear,
          alignment: Alignment.topCenter,
        ),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(8);
    h.setPhase(2);
    await h.frames(22);
  });

  frameScenario('switchers__animated_size_retarget', (h) async {
    await h.mount((phase) {
      final width = phase == 0 ? 80.0 : phase == 1 ? 180.0 : 120.0;
      final height = phase == 0 ? 40.0 : phase == 1 ? 100.0 : 64.0;
      return Center(
        child: h.probe(
          'animated_size',
          AnimatedSize(
            duration: _duration,
            curve: Curves.linear,
            alignment: Alignment.center,
            clipBehavior: Clip.hardEdge,
            child: _box(_green, width, height),
          ),
        ),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(8);
    h.setPhase(2);
    await h.frames(22);
  });

  frameScenario('switchers__animated_list_insert_size_transition', (h) async {
    final key = GlobalKey<AnimatedListState>();
    final items = <String>['list_a', 'list_b'];
    Animation<double>? incomingAnimation;
    var inserted = false;
    h.value('item_count', () => items.length.toDouble());
    h.value('incoming_value', () => incomingAnimation?.value ?? 0);
    await h.mount((phase) {
      if (phase == 1 && !inserted) {
        inserted = true;
        items.insert(1, 'list_new');
        key.currentState!.insertItem(1, duration: _duration);
      }
      return AnimatedList(
        key: key,
        initialItemCount: 2,
        padding: EdgeInsets.zero,
        itemBuilder: (context, index, animation) {
          final name = items[index];
          if (name == 'list_new') incomingAnimation = animation;
          final color = name == 'list_a' ? _blue : name == 'list_b' ? _orange : _green;
          return h.probe(
            name,
            SizeTransition(
              sizeFactor: animation,
              axis: Axis.vertical,
              axisAlignment: 0,
              child: SizedBox(height: 48, child: ColoredBox(color: color)),
            ),
          );
        },
      );
    });
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
    await h.frames(21);
  });

  frameScenario('switchers__animated_grid_insert_size_transition', (h) async {
    final key = GlobalKey<AnimatedGridState>();
    final items = <String>['grid_a', 'grid_b'];
    Animation<double>? incomingAnimation;
    var inserted = false;
    h.value('item_count', () => items.length.toDouble());
    h.value('incoming_value', () => incomingAnimation?.value ?? 0);
    await h.mount((phase) {
      if (phase == 1 && !inserted) {
        inserted = true;
        items.insert(1, 'grid_new');
        key.currentState!.insertItem(1, duration: _duration);
      }
      return AnimatedGrid(
        key: key,
        initialItemCount: 2,
        gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
          crossAxisCount: 2,
          mainAxisExtent: 60,
        ),
        itemBuilder: (context, index, animation) {
          final name = items[index];
          if (name == 'grid_new') incomingAnimation = animation;
          final color = name == 'grid_a' ? _blue : name == 'grid_b' ? _orange : _green;
          return h.probe(
            name,
            SizeTransition(
              sizeFactor: animation,
              axis: Axis.vertical,
              axisAlignment: 0,
              child: SizedBox(height: 60, child: ColoredBox(color: color)),
            ),
          );
        },
      );
    });
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
    await h.frames(21);
  });

  frameScenario('switchers__sliver_animated_list_initial_layout', (h) async {
    const items = <String>['sliver_list_a', 'sliver_list_b'];
    await h.mount((_) {
      return CustomScrollView(
        slivers: <Widget>[
          SliverAnimatedList(
            initialItemCount: 2,
            itemBuilder: (context, index, animation) {
              final name = items[index];
              final color = index == 0 ? _blue : _orange;
              return h.probe(
                name,
                SizeTransition(
                  sizeFactor: animation,
                  axis: Axis.vertical,
                  axisAlignment: 0,
                  child: SizedBox(height: 48, child: ColoredBox(color: color)),
                ),
              );
            },
          ),
        ],
      );
    });
    await h.frames(6);
  });

  frameScenario('switchers__sliver_animated_grid_insert_size_transition', (h) async {
    final key = GlobalKey<SliverAnimatedGridState>();
    final items = <String>['sliver_grid_a', 'sliver_grid_b'];
    Animation<double>? incomingAnimation;
    var inserted = false;
    h.value('item_count', () => items.length.toDouble());
    h.value('incoming_value', () => incomingAnimation?.value ?? 0);
    await h.mount((phase) {
      if (phase == 1 && !inserted) {
        inserted = true;
        items.insert(1, 'sliver_grid_new');
        key.currentState!.insertItem(1, duration: _duration);
      }
      return CustomScrollView(
        slivers: <Widget>[
          SliverAnimatedGrid(
            key: key,
            initialItemCount: 2,
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              mainAxisExtent: 60,
            ),
            itemBuilder: (context, index, animation) {
              final name = items[index];
              if (name == 'sliver_grid_new') incomingAnimation = animation;
              final color = name == 'sliver_grid_a' ? _blue : name == 'sliver_grid_b' ? _orange : _green;
              return h.probe(
                name,
                SizeTransition(
                  sizeFactor: animation,
                  axis: Axis.vertical,
                  axisAlignment: 0,
                  child: SizedBox(height: 60, child: ColoredBox(color: color)),
                ),
              );
            },
          ),
        ],
      );
    });
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
    await h.frames(21);
  });

  frameScenario('switchers__animated_modal_barrier_color_and_dismiss', (h) async {
    final controller = AnimationController(duration: _duration, vsync: const TestVSync());
    addTearDown(controller.dispose);
    var dismissals = 0;
    h.value('dismissals', () => dismissals.toDouble());
    await h.mount((_) {
      return h.probe(
        'barrier',
        AnimatedModalBarrier(
          color: ColorTween(
            begin: const Color(0x80000000),
            end: const Color(0x804C77A0),
          ).animate(controller),
          dismissible: true,
          onDismiss: () => dismissals += 1,
          semanticsLabel: 'Dismiss',
        ),
      );
    });
    await h.frames(2);
    controller.forward();
    await h.frames(8);
    await h.down(const Offset(400, 300));
    await h.up();
    await h.frames(1);
    await h.frames(15);
  });

  frameScenario('switchers__animated_list_remove_shifts_items', (h) async {
    final key = GlobalKey<AnimatedListState>();
    final items = <String>['remove_a', 'remove_b', 'remove_c'];
    Animation<double>? outgoingAnimation;
    var removed = false;
    h.value('item_count', () => items.length.toDouble());
    h.value('outgoing_value', () => outgoingAnimation?.value ?? 1);
    await h.mount((phase) {
      if (phase == 1 && !removed) {
        removed = true;
        items.removeAt(0);
        key.currentState!.removeItem(
          0,
          (context, animation) {
            outgoingAnimation = animation;
            return h.probe(
              'removed_a',
              SizeTransition(
                sizeFactor: animation,
                axis: Axis.vertical,
                axisAlignment: 0,
                child: const SizedBox(height: 48, child: ColoredBox(color: _blue)),
              ),
            );
          },
          duration: _duration,
        );
      }
      return AnimatedList(
        key: key,
        initialItemCount: 3,
        padding: EdgeInsets.zero,
        itemBuilder: (context, index, animation) {
          final name = items[index];
          final color = name == 'remove_a' ? _blue : name == 'remove_b' ? _orange : _green;
          return h.probe(
            name,
            SizeTransition(
              sizeFactor: animation,
              axis: Axis.vertical,
              axisAlignment: 0,
              child: SizedBox(height: 48, child: ColoredBox(color: color)),
            ),
          );
        },
      );
    });
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
    await h.frames(21);
  });

  frameScenario('switchers__switcher_same_key_no_transition', (h) async {
    await h.mount((phase) {
      return Center(
        child: AnimatedSwitcher(
          duration: _duration,
          child: h.probe('same_key', _box(phase == 0 ? _blue : _orange, 80, 48)),
        ),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(8);
  });

  frameScenario('switchers__switcher_reverse_duration', (h) async {
    await h.mount((phase) {
      return Center(
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 150),
          reverseDuration: const Duration(milliseconds: 450),
          child: phase == 0
              ? h.probe('switch_a', _box(_blue, 80, 48))
              : h.probe('switch_b', _box(_orange, 140, 64)),
        ),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(30);
  });

  frameScenario('switchers__switcher_layout_builder_top_left', (h) async {
    await h.mount((phase) {
      final Widget child = phase == 0
          ? h.probe('switch_a', _box(_blue, 80, 48))
          : h.probe('switch_b', _box(_orange, 140, 64));
      return Center(
        child: AnimatedSwitcher(
          duration: _duration,
          layoutBuilder: (current, previous) => Stack(
            alignment: Alignment.topLeft,
            children: <Widget>[...previous, ?current],
          ),
          child: child,
        ),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(22);
  });

  frameScenario('switchers__animated_size_top_left_alignment', (h) async {
    await h.mount((phase) {
      final width = phase == 0 ? 80.0 : 180.0;
      final height = phase == 0 ? 40.0 : 100.0;
      return Center(
        child: h.probe(
          'animated_size',
          AnimatedSize(
            duration: _duration,
            curve: Curves.linear,
            alignment: Alignment.topLeft,
            clipBehavior: Clip.hardEdge,
            child: h.probe('size_child', _box(_green, width, height)),
          ),
        ),
      );
    });
    await h.frames(2);
    h.setPhase(1);
    await h.frames(22);
  });
}
