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
}
