import 'package:flutter/widgets.dart';

import 'recorder.dart';

Widget _widgetsApp(Widget child) => WidgetsApp(
  color: const Color(0xFFFFFFFF),
  builder: (_, child) => child!,
  pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
    settings: settings,
    pageBuilder: (context, animation, secondaryAnimation) => builder(context),
    transitionDuration: Duration.zero,
    reverseTransitionDuration: Duration.zero,
  ),
  home: child,
);

void main() {
  frameScenario('focus_input__focus_scope_default_traversal', (h) async {
    final first = FocusNode();
    final middle = FocusNode();
    final last = FocusNode();
    h.value('first_focused', () => first.hasFocus ? 1 : 0);
    h.value('middle_focused', () => middle.hasFocus ? 1 : 0);
    h.value('last_focused', () => last.hasFocus ? 1 : 0);
    await h.mount(
      (_) => _widgetsApp(
        SizedBox(
          width: 120,
          height: 100,
          child: FocusScope(
            child: Stack(
              children: <Widget>[
                Positioned(
                  top: 0,
                  child: Focus(
                    focusNode: first,
                    child: h.probe(
                      'first',
                      const ColoredBox(
                        color: Color(0xFF2196F3),
                        child: SizedBox(width: 20, height: 20),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 40,
                  child: Focus(
                    focusNode: middle,
                    autofocus: true,
                    child: h.probe(
                      'middle',
                      const ColoredBox(
                        color: Color(0xFF4CAF50),
                        child: SizedBox(width: 20, height: 20),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 20,
                  child: Focus(
                    focusNode: last,
                    child: h.probe(
                      'last',
                      const ColoredBox(
                        color: Color(0xFFFF5722),
                        child: SizedBox(width: 20, height: 20),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
  });

  frameScenario('focus_input__shortcuts_action_intent', (h) async {
    final focusNode = FocusNode();
    var calls = 0;
    h.value('calls', () => calls.toDouble());
    await h.mount(
      (_) => _widgetsApp(
        FocusableActionDetector(
          focusNode: focusNode,
          autofocus: true,
          shortcuts: <ShortcutActivator, Intent>{
            SingleActivator(LogicalKeyboardKey.keyK): const ActivateIntent(),
          },
          actions: <Type, Action<Intent>>{
            ActivateIntent: CallbackAction<ActivateIntent>(
              onInvoke: (_) {
                calls++;
                return null;
              },
            ),
          },
          child: h.probe(
            'action_target',
            const ColoredBox(
              color: Color(0xFF2196F3),
              child: SizedBox(width: 40, height: 30),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.keyK);
    await h.frames(1);
  });

  frameScenario('focus_input__keyboard_page_scroll', (h) async {
    final controller = ScrollController();
    final rowFocus = FocusNode();
    h.value('offset', () => controller.offset);
    h.value('row_focused', () => rowFocus.hasFocus ? 1 : 0);
    await h.mount(
      (_) => _widgetsApp(
        PrimaryScrollController(
          controller: controller,
          child: ListView.builder(
            primary: true,
            itemExtent: 50,
            itemCount: 30,
            itemBuilder: (context, index) {
              final child = SizedBox.expand(
                child: ColoredBox(
                  color: index.isEven
                      ? const Color(0xFF2196F3)
                      : const Color(0xFF4CAF50),
                ),
              );
              return index == 0
                  ? Focus(
                      focusNode: rowFocus,
                      autofocus: true,
                      child: h.probe('row0', child),
                    )
                  : child;
            },
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.pageDown);
    await h.frames(10);
  });

  frameScenario('focus_input__mouse_region_enter_hover_exit', (h) async {
    var enters = 0;
    var hovers = 0;
    var exits = 0;
    h.value('enters', () => enters.toDouble());
    h.value('hovers', () => hovers.toDouble());
    h.value('exits', () => exits.toDouble());
    await h.mount(
      (_) => MouseRegion(
        onEnter: (_) => enters++,
        onHover: (_) => hovers++,
        onExit: (_) => exits++,
        child: h.probe(
          'hover_target',
          const SizedBox(
            width: 80,
            height: 60,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );
    await h.hover(const Offset(20, 20));
    await h.frames(1);
    await h.hover(const Offset(30, 30));
    await h.frames(1);
    await h.hover(const Offset(100, 100));
    await h.frames(1);
  });

  frameScenario('focus_input__listener_pointer_lifecycle', (h) async {
    var downs = 0;
    var moves = 0;
    var ups = 0;
    h.value('downs', () => downs.toDouble());
    h.value('moves', () => moves.toDouble());
    h.value('ups', () => ups.toDouble());
    await h.mount(
      (_) => Listener(
        onPointerDown: (_) => downs++,
        onPointerMove: (_) => moves++,
        onPointerUp: (_) => ups++,
        child: h.probe(
          'listener_target',
          const SizedBox(
            width: 80,
            height: 60,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );
    await h.down(const Offset(20, 20));
    await h.frames(1);
    await h.move(const Offset(30, 30));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });

  frameScenario('focus_input__ignore_pointer_falls_through', (h) async {
    var taps = 0;
    h.value('taps', () => taps.toDouble());
    await h.mount(
      (_) => SizedBox(
        width: 80,
        height: 60,
        child: Stack(
          children: <Widget>[
            GestureDetector(
              onTap: () => taps++,
              child: h.probe(
                'underlay',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              ),
            ),
            IgnorePointer(
              child: h.probe(
                'ignored_overlay',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFFFF5722)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.down(const Offset(20, 20));
    await h.up();
    await h.frames(1);
  });

  frameScenario('focus_input__absorb_pointer_blocks_sibling', (h) async {
    var taps = 0;
    h.value('taps', () => taps.toDouble());
    await h.mount(
      (_) => SizedBox(
        width: 80,
        height: 60,
        child: Stack(
          children: <Widget>[
            GestureDetector(
              onTap: () => taps++,
              child: h.probe(
                'underlay',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              ),
            ),
            AbsorbPointer(
              child: h.probe(
                'absorbing_overlay',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFFFF5722)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.down(const Offset(20, 20));
    await h.up();
    await h.frames(1);
  });

  frameScenario('focus_input__childless_sizedbox_passes_hits', (h) async {
    var taps = 0;
    h.value('taps', () => taps.toDouble());
    await h.mount(
      (_) => SizedBox(
        width: 80,
        height: 60,
        child: Stack(
          children: <Widget>[
            GestureDetector(
              onTap: () => taps++,
              child: h.probe(
                'underlay',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              ),
            ),
            Positioned(
              top: 0,
              left: 0,
              child: h.probe('empty_overlay', const SizedBox(width: 20, height: 20)),
            ),
          ],
        ),
      ),
    );
    await h.down(const Offset(10, 10));
    await h.up();
    await h.frames(1);
  });

  frameScenario('focus_input__reading_order_band_overlap', (h) async {
    final a = FocusNode();
    final b = FocusNode();
    final c = FocusNode();
    h.value('a_focused', () => a.hasFocus ? 1 : 0);
    h.value('b_focused', () => b.hasFocus ? 1 : 0);
    h.value('c_focused', () => c.hasFocus ? 1 : 0);
    await h.mount(
      (_) => _widgetsApp(
        SizedBox(
          width: 200,
          height: 100,
          child: FocusTraversalGroup(
            child: Stack(
              children: <Widget>[
                Positioned(
                  top: 0,
                  left: 60,
                  child: Focus(
                    focusNode: a,
                    child: h.probe(
                      'a',
                      const SizedBox(
                        width: 20,
                        height: 20,
                        child: ColoredBox(color: Color(0xFF2196F3)),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 15,
                  left: 0,
                  child: Focus(
                    focusNode: b,
                    child: h.probe(
                      'b',
                      const SizedBox(
                        width: 20,
                        height: 20,
                        child: ColoredBox(color: Color(0xFF4CAF50)),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 40,
                  left: 120,
                  child: Focus(
                    focusNode: c,
                    child: h.probe(
                      'c',
                      const SizedBox(
                        width: 20,
                        height: 20,
                        child: ColoredBox(color: Color(0xFFFF5722)),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
  });

  frameScenario('focus_input__ordered_unordered_reading_order', (h) async {
    final x = FocusNode();
    final y = FocusNode();
    final z = FocusNode();
    h.value('x_focused', () => x.hasFocus ? 1 : 0);
    h.value('y_focused', () => y.hasFocus ? 1 : 0);
    h.value('z_focused', () => z.hasFocus ? 1 : 0);
    await h.mount(
      (_) => _widgetsApp(
        SizedBox(
          width: 200,
          height: 100,
          child: FocusTraversalGroup(
            policy: OrderedTraversalPolicy(),
            child: Stack(
              children: <Widget>[
                Positioned(
                  top: 40,
                  left: 0,
                  child: Focus(
                    focusNode: x,
                    child: h.probe(
                      'x',
                      const SizedBox(
                        width: 20,
                        height: 20,
                        child: ColoredBox(color: Color(0xFF2196F3)),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 0,
                  left: 60,
                  child: Focus(
                    focusNode: y,
                    child: h.probe(
                      'y',
                      const SizedBox(
                        width: 20,
                        height: 20,
                        child: ColoredBox(color: Color(0xFF4CAF50)),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 20,
                  left: 120,
                  child: FocusTraversalOrder(
                    order: NumericFocusOrder(1),
                    child: Focus(
                      focusNode: z,
                      child: h.probe(
                        'z',
                        const SizedBox(
                          width: 20,
                          height: 20,
                          child: ColoredBox(color: Color(0xFFFF5722)),
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
  });

  frameScenario('focus_input__key_event_bubbles_past_ignored', (h) async {
    final outerNode = FocusNode();
    final innerNode = FocusNode();
    var innerCalls = 0;
    var outerCalls = 0;
    h.value('inner_calls', () => innerCalls.toDouble());
    h.value('outer_calls', () => outerCalls.toDouble());
    await h.mount(
      (_) => _widgetsApp(
        Focus(
          focusNode: outerNode,
          onKeyEvent: (node, event) {
            outerCalls++;
            return KeyEventResult.handled;
          },
          child: Focus(
            focusNode: innerNode,
            autofocus: true,
            onKeyEvent: (node, event) {
              innerCalls++;
              return KeyEventResult.ignored;
            },
            child: h.probe(
              'key_target',
              const SizedBox(
                width: 40,
                height: 30,
                child: ColoredBox(color: Color(0xFF2196F3)),
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.keyK);
    await h.frames(1);
  });

  frameScenario('focus_input__mouse_region_opaque_blocks_underlay', (h) async {
    var underEnters = 0;
    var topEnters = 0;
    h.value('under_enters', () => underEnters.toDouble());
    h.value('top_enters', () => topEnters.toDouble());
    await h.mount(
      (_) => SizedBox(
        width: 80,
        height: 60,
        child: Stack(
          children: <Widget>[
            MouseRegion(
              onEnter: (_) => underEnters++,
              child: h.probe(
                'under',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              ),
            ),
            MouseRegion(
              onEnter: (_) => topEnters++,
              child: h.probe(
                'top',
                const SizedBox(
                  width: 40,
                  height: 30,
                  child: ColoredBox(color: Color(0xFFFF5722)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.hover(const Offset(20, 20));
    await h.frames(1);
    await h.hover(const Offset(60, 50));
    await h.frames(1);
  });

  frameScenario('focus_input__mouse_region_translucent_passes_underlay', (h) async {
    var underEnters = 0;
    var topEnters = 0;
    h.value('under_enters', () => underEnters.toDouble());
    h.value('top_enters', () => topEnters.toDouble());
    await h.mount(
      (_) => SizedBox(
        width: 80,
        height: 60,
        child: Stack(
          children: <Widget>[
            MouseRegion(
              onEnter: (_) => underEnters++,
              child: h.probe(
                'under',
                const SizedBox(
                  width: 80,
                  height: 60,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              ),
            ),
            MouseRegion(
              opaque: false,
              onEnter: (_) => topEnters++,
              child: h.probe(
                'top',
                const SizedBox(
                  width: 40,
                  height: 30,
                  child: ColoredBox(color: Color(0xFFFF5722)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.hover(const Offset(20, 20));
    await h.frames(1);
    await h.hover(const Offset(60, 50));
    await h.frames(1);
  });

  frameScenario('focus_input__mouse_region_nested_enter_order', (h) async {
    var enterSequence = 0;
    var exitSequence = 0;
    h.value('enter_sequence', () => enterSequence.toDouble());
    h.value('exit_sequence', () => exitSequence.toDouble());
    await h.mount(
      (_) => MouseRegion(
        onEnter: (_) => enterSequence = enterSequence * 10 + 1,
        onExit: (_) => exitSequence = exitSequence * 10 + 1,
        child: MouseRegion(
          onEnter: (_) => enterSequence = enterSequence * 10 + 2,
          onExit: (_) => exitSequence = exitSequence * 10 + 2,
          child: h.probe(
            'nested_target',
            const SizedBox(
              width: 80,
              height: 60,
              child: ColoredBox(color: Color(0xFF2196F3)),
            ),
          ),
        ),
      ),
    );
    await h.hover(const Offset(20, 20));
    await h.frames(1);
    await h.hover(const Offset(100, 100));
    await h.frames(1);
  });

  frameScenario('focus_input__tap_region_group_outside', (h) async {
    var aInside = 0;
    var aOutside = 0;
    var bInside = 0;
    var bOutside = 0;
    var cInside = 0;
    var cOutside = 0;
    h.value('a_inside', () => aInside.toDouble());
    h.value('a_outside', () => aOutside.toDouble());
    h.value('b_inside', () => bInside.toDouble());
    h.value('b_outside', () => bOutside.toDouble());
    h.value('c_inside', () => cInside.toDouble());
    h.value('c_outside', () => cOutside.toDouble());
    await h.mount(
      (_) => SizedBox(
        width: 120,
        height: 60,
        child: TapRegionSurface(
          child: Stack(
            children: <Widget>[
              Positioned(
                top: 0,
                left: 0,
                child: TapRegion(
                  groupId: 'g',
                  onTapInside: (_) => aInside++,
                  onTapOutside: (_) => aOutside++,
                  child: h.probe(
                    'region_a',
                    const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xFF2196F3)),
                    ),
                  ),
                ),
              ),
              Positioned(
                top: 0,
                left: 40,
                child: TapRegion(
                  groupId: 'g',
                  onTapInside: (_) => bInside++,
                  onTapOutside: (_) => bOutside++,
                  child: h.probe(
                    'region_b',
                    const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xFF4CAF50)),
                    ),
                  ),
                ),
              ),
              Positioned(
                top: 0,
                left: 80,
                child: TapRegion(
                  onTapInside: (_) => cInside++,
                  onTapOutside: (_) => cOutside++,
                  child: h.probe(
                    'region_c',
                    const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xFFFF5722)),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    await h.down(const Offset(20, 15));
    await h.up();
    await h.frames(1);
    await h.down(const Offset(60, 15));
    await h.up();
    await h.frames(1);
    await h.down(const Offset(100, 15));
    await h.up();
    await h.frames(1);
  });

  frameScenario('focus_input__arrow_directional_focus', (h) async {
    final a = FocusNode();
    final b = FocusNode();
    final c = FocusNode();
    h.value('a_focused', () => a.hasFocus ? 1 : 0);
    h.value('b_focused', () => b.hasFocus ? 1 : 0);
    h.value('c_focused', () => c.hasFocus ? 1 : 0);
    await h.mount(
      (_) => _widgetsApp(
        SizedBox(
          width: 200,
          height: 60,
          child: Row(
            children: <Widget>[
              Focus(
                focusNode: a,
                autofocus: true,
                child: h.probe(
                  'a',
                  const SizedBox(
                    width: 40,
                    height: 30,
                    child: ColoredBox(color: Color(0xFF2196F3)),
                  ),
                ),
              ),
              Focus(
                focusNode: b,
                child: h.probe(
                  'b',
                  const SizedBox(
                    width: 40,
                    height: 30,
                    child: ColoredBox(color: Color(0xFF4CAF50)),
                  ),
                ),
              ),
              Focus(
                focusNode: c,
                child: h.probe(
                  'c',
                  const SizedBox(
                    width: 40,
                    height: 30,
                    child: ColoredBox(color: Color(0xFFFF5722)),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.arrowRight);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.arrowRight);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.arrowLeft);
    await h.frames(1);
  });

  frameScenario('focus_input__shift_tab_reverse_traversal', (h) async {
    final a = FocusNode();
    final b = FocusNode();
    final c = FocusNode();
    h.value('a_focused', () => a.hasFocus ? 1 : 0);
    h.value('b_focused', () => b.hasFocus ? 1 : 0);
    h.value('c_focused', () => c.hasFocus ? 1 : 0);
    await h.mount(
      (_) => _widgetsApp(
        SizedBox(
          width: 200,
          height: 60,
          child: FocusTraversalGroup(
            child: Row(
              children: <Widget>[
                Focus(
                  focusNode: a,
                  autofocus: true,
                  child: h.probe(
                    'a',
                    const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xFF2196F3)),
                    ),
                  ),
                ),
                Focus(
                  focusNode: b,
                  child: h.probe(
                    'b',
                    const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xFF4CAF50)),
                    ),
                  ),
                ),
                Focus(
                  focusNode: c,
                  child: h.probe(
                    'c',
                    const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xFFFF5722)),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.tab);
    await h.frames(1);
    await h.keyDown(LogicalKeyboardKey.shiftLeft);
    await h.key(LogicalKeyboardKey.tab);
    await h.keyUp(LogicalKeyboardKey.shiftLeft);
    await h.frames(1);
  });

  frameScenario('focus_input__shortcut_chord_requires_modifiers', (h) async {
    final focusNode = FocusNode();
    var ctrlCalls = 0;
    var plainCalls = 0;
    h.value('ctrl_calls', () => ctrlCalls.toDouble());
    h.value('plain_calls', () => plainCalls.toDouble());
    await h.mount(
      (_) => _widgetsApp(
        FocusableActionDetector(
          focusNode: focusNode,
          autofocus: true,
          shortcuts: <ShortcutActivator, Intent>{
            SingleActivator(LogicalKeyboardKey.keyK, control: true): const ActivateIntent(),
            SingleActivator(LogicalKeyboardKey.keyK): const ButtonActivateIntent(),
          },
          actions: <Type, Action<Intent>>{
            ActivateIntent: CallbackAction<ActivateIntent>(
              onInvoke: (_) {
                ctrlCalls++;
                return null;
              },
            ),
            ButtonActivateIntent: CallbackAction<ButtonActivateIntent>(
              onInvoke: (_) {
                plainCalls++;
                return null;
              },
            ),
          },
          child: h.probe(
            'chord_target',
            const ColoredBox(
              color: Color(0xFF2196F3),
              child: SizedBox(width: 40, height: 30),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.keyDown(LogicalKeyboardKey.controlLeft);
    await h.key(LogicalKeyboardKey.keyK);
    await h.keyUp(LogicalKeyboardKey.controlLeft);
    await h.frames(1);
    await h.key(LogicalKeyboardKey.keyK);
    await h.frames(1);
  });
}
