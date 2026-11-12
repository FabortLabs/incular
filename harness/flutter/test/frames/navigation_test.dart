import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);

Widget _page(Color color) => ColoredBox(color: color);

class _TestPage extends Page<void> {
  const _TestPage({required super.key, required this.child});
  final Widget child;

  @override
  Route<void> createRoute(BuildContext context) => PageRouteBuilder<void>(
        settings: this,
        transitionDuration: Duration.zero,
        reverseTransitionDuration: Duration.zero,
        pageBuilder: (context, animation, secondaryAnimation) => child,
      );
}

void main() {
  frameScenario('navigation__navigator__push_pop_can_pop', (h) async {
    final key = GlobalKey<NavigatorState>();
    var pushes = 0;
    var pops = 0;
    h.value('can_pop', () => key.currentState?.canPop() == true ? 1 : 0);
    h.value('pushes', () => pushes.toDouble());
    h.value('pops', () => pops.toDouble());
    await h.mount((_) => Navigator(
          key: key,
          pages: <Page<void>>[
            _TestPage(key: const ValueKey<String>('home'), child: _page(blue)),
          ],
          onDidRemovePage: (_) {},
          observers: <NavigatorObserver>[_CountingObserver(() => pushes++, () => pops++)],
        ));
    await h.frames(1);
    key.currentState!.push<void>(PageRouteBuilder<void>(
      settings: const RouteSettings(name: '/details'),
      transitionDuration: Duration.zero,
      reverseTransitionDuration: Duration.zero,
      pageBuilder: (context, animation, secondaryAnimation) => _page(orange),
    ));
    await h.frames(2);
    key.currentState!.pop<void>();
    await h.frames(2);
  });

  frameScenario('navigation__pop_scope__blocked_root_route', (h) async {
    final key = GlobalKey<NavigatorState>();
    var callbacks = 0;
    var didPop = -1.0;
    h.value('callbacks', () => callbacks.toDouble());
    h.value('did_pop', () => didPop);
    await h.mount((_) => Navigator(
          key: key,
          pages: <Page<void>>[
            _TestPage(
              key: const ValueKey<String>('root'),
              child: PopScope<void>(
                canPop: false,
                onPopInvokedWithResult: (popped, result) {
                  callbacks++;
                  didPop = popped ? 1 : 0;
                },
              child: const SizedBox(width: 100, height: 80, child: ColoredBox(color: blue)),
              ),
            ),
          ],
          onDidRemovePage: (_) {},
        ));
    await h.frames(1);
    await key.currentState!.maybePop<void>();
    await h.frames(1);
  });

  frameScenario('navigation__overlay_portal__content_outside_anchor_clip', (h) async {
    var taps = 0;
    h.value('popup_taps', () => taps.toDouble());
    final controller = OverlayPortalController();
    await h.mount((_) => Overlay(
          initialEntries: <OverlayEntry>[
            OverlayEntry(
              builder: (context) => Align(
                alignment: Alignment.topLeft,
                child: ClipRect(
                  child: SizedBox(
                    width: 100,
                    height: 100,
                    child: OverlayPortal.overlayChildLayoutBuilder(
                    controller: controller,
                    overlayChildBuilder: (context, info) => Stack(children: <Widget>[
                      Positioned(
                        left: 200,
                        top: 200,
                        child: GestureDetector(
                          onTap: () => taps++,
                          child: h.probe(
                            'popup',
                            const SizedBox(width: 80, height: 60, child: ColoredBox(color: orange)),
                          ),
                        ),
                      ),
                    ]),
                      child: Align(
                        alignment: Alignment.center,
                        child: h.probe(
                          'anchor',
                          const SizedBox(
                            width: 60,
                            height: 50,
                            child: ColoredBox(color: blue, child: SizedBox.shrink()),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ));
    await h.frames(1);
    controller.show();
    await h.frames(1);
    await h.down(const Offset(225, 225));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });
}

class _CountingObserver extends NavigatorObserver {
  _CountingObserver(this.onPush, this.onPop);
  final VoidCallback onPush;
  final VoidCallback onPop;
  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) => onPush();
  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) => onPop();
}

