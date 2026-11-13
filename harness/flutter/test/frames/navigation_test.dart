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

  frameScenario('navigation__router__provider_route_change', (h) async {
    final provider = _TestRouteInformationProvider('/home');
    final delegate = _TestRouterDelegate(h);
    h.value('route_is_details', () => delegate.path == '/details' ? 1.0 : 0.0);
    h.value(
      'provider_is_details',
      () => provider.value.uri.path == '/details' ? 1.0 : 0.0,
    );
    await h.mount(
      (_) => Router<String>(
        routeInformationProvider: provider,
        routeInformationParser: _TestRouteInformationParser(),
        routerDelegate: delegate,
      ),
    );
    await h.frames(2);
    provider.setPath('/details');
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

  frameScenario('navigation__overlay_portal__show_hide', (h) async {
    final controller = OverlayPortalController();
    await h.mount((_) => Overlay(
          initialEntries: <OverlayEntry>[
            OverlayEntry(
              builder: (context) => Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 100,
                  height: 100,
                  child: OverlayPortal.overlayChildLayoutBuilder(
                    controller: controller,
                    overlayChildBuilder: (context, info) => Stack(children: <Widget>[
                      Positioned(
                        left: 200,
                        top: 200,
                        child: h.probe(
                          'popup',
                          const SizedBox(width: 80, height: 60, child: ColoredBox(color: orange)),
                        ),
                      ),
                    ]),
                    child: Align(
                      alignment: Alignment.center,
                      child: h.probe(
                        'anchor',
                        const SizedBox(width: 60, height: 50, child: ColoredBox(color: blue)),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ));
    await h.frames(2);
    controller.show();
    await h.frames(2);
    controller.hide();
    await h.frames(2);
  });

  frameScenario('navigation__overlay_portal__direct_anchor', (h) async {
    final controller = OverlayPortalController();
    await h.mount((_) => Overlay(
          initialEntries: <OverlayEntry>[
            OverlayEntry(
              builder: (context) => Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 100,
                  height: 100,
                  child: OverlayPortal.overlayChildLayoutBuilder(
                    controller: controller,
                    overlayChildBuilder: (context, info) => const SizedBox.shrink(),
                    child: h.probe(
                      'anchor',
                      const SizedBox(width: 60, height: 50, child: ColoredBox(color: blue)),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ));
    await h.frames(2);
  });
  frameScenario('navigation__navigator__fade_push_timeline', (h) async {
    await _fadeTimeline(h, pop: false);
  });

  frameScenario('navigation__navigator__fade_pop_timeline', (h) async {
    await _fadeTimeline(h, pop: true);
  });

  frameScenario('navigation__hero__push_flight_navigator', (h) async {
    await _heroFlight(h, pop: false);
  });

  frameScenario('navigation__hero__pop_flight_navigator', (h) async {
    await _heroFlight(h, pop: true);
  });

  frameScenario('navigation__navigator__observer_order', (h) async {
    final key = GlobalKey<NavigatorState>();
    var log = 0.0;
    void record(double code) => log = log * 10 + code;
    h.value('log', () => log);
    await h.mount((_) => Navigator(
          key: key,
          observers: <NavigatorObserver>[_OrderObserver(record)],
          pages: <Page<void>>[
            _TestPage(key: const ValueKey<String>('root'), child: _probeBox(h, 'root', blue)),
          ],
          onDidRemovePage: (_) {},
        ));
    await h.frames(2);
    key.currentState!.push<void>(_fadeRoute(_probeBox(h, 'a', orange), 0, 0));
    await h.frames(2);
    key.currentState!.pushReplacement<void, void>(_fadeRoute(_probeBox(h, 'b', blue), 0, 0));
    await h.frames(2);
    key.currentState!.pop<void>();
    await h.frames(2);
  });

  frameScenario('navigation__pop_scope__route_back_blocked', (h) async {
    final key = GlobalKey<NavigatorState>();
    var callbacks = 0;
    var didPop = -1.0;
    h.value('callbacks', () => callbacks.toDouble());
    h.value('did_pop', () => didPop);
    await h.mount((_) => Navigator(
          key: key,
          pages: <Page<void>>[
            _TestPage(key: const ValueKey<String>('root'), child: _probeBox(h, 'root', blue)),
          ],
          onDidRemovePage: (_) {},
        ));
    await h.frames(2);
    key.currentState!.push<void>(PageRouteBuilder<void>(
      transitionDuration: Duration.zero,
      reverseTransitionDuration: Duration.zero,
      pageBuilder: (context, animation, secondaryAnimation) => PopScope<void>(
        canPop: false,
        onPopInvokedWithResult: (popped, result) {
          callbacks++;
          didPop = popped ? 1 : 0;
        },
        child: _probeBox(h, 'top', orange),
      ),
    ));
    await h.frames(2);
    // handlePopRoute does not reach a bare Navigator (no Router or WidgetsApp
    // registers it as the root back handler), so the Navigator back action is
    // maybePop; the Incular twin dispatches back through its own dispatcher.
    await key.currentState!.maybePop<void>();
    await h.frames(2);
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

class _TestRouteInformationProvider extends RouteInformationProvider
    with ChangeNotifier {
  _TestRouteInformationProvider(String path)
    : _value = RouteInformation(uri: Uri.parse(path));

  RouteInformation _value;

  @override
  RouteInformation get value => _value;

  void setPath(String path) {
    _value = RouteInformation(uri: Uri.parse(path));
    notifyListeners();
  }
}

class _TestRouteInformationParser extends RouteInformationParser<String> {
  @override
  Future<String> parseRouteInformation(RouteInformation information) async =>
      information.uri.path;

  @override
  RouteInformation restoreRouteInformation(String configuration) =>
      RouteInformation(uri: Uri.parse(configuration));
}

class _TestRouterDelegate extends RouterDelegate<String> with ChangeNotifier {
  _TestRouterDelegate(this.h);

  final FrameHarness h;
  String path = '/home';

  @override
  String get currentConfiguration => path;

  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.topLeft,
    child: h.probe(
      'route',
      SizedBox(
        width: 120,
        height: 80,
        child: ColoredBox(color: path == '/details' ? orange : blue),
      ),
    ),
  );

  @override
  Future<void> setNewRoutePath(String configuration) async {
    path = configuration;
    notifyListeners();
  }

  @override
  Future<bool> popRoute() async => false;
}

Widget _probeBox(FrameHarness h, String name, Color color) => Align(
      alignment: Alignment.topLeft,
      child: h.probe(
        name,
        SizedBox(width: 120, height: 80, child: ColoredBox(color: color)),
      ),
    );

PageRouteBuilder<void> _fadeRoute(Widget page, int ms, int reverseMs) => PageRouteBuilder<void>(
      transitionDuration: Duration(milliseconds: ms),
      reverseTransitionDuration: Duration(milliseconds: reverseMs),
      pageBuilder: (context, animation, secondaryAnimation) => page,
      transitionsBuilder: (context, animation, secondaryAnimation, child) =>
          FadeTransition(opacity: animation, child: child),
    );

Widget _heroStack(FrameHarness h, String name, double x, double y, double w, double hgt, Color color) =>
    Stack(children: <Widget>[
      Positioned(
        left: x,
        top: y,
        child: Hero(
          tag: 'photo',
          child: h.probe(name, SizedBox(width: w, height: hgt, child: ColoredBox(color: color))),
        ),
      ),
    ]);

Future<void> _fadeTimeline(FrameHarness h, {required bool pop}) async {
  final key = GlobalKey<NavigatorState>();
  await h.mount((_) => Navigator(
        key: key,
        pages: <Page<void>>[_RootPage(h)],
        onDidRemovePage: (_) {},
      ));
  await h.frames(2);
  key.currentState!.push<void>(_fadeRoute(_probeBox(h, 'top', orange), 300, 300));
  await h.frames(22);
  if (pop) {
    key.currentState!.pop<void>();
    await h.frames(22);
  }
}

Future<void> _heroFlight(FrameHarness h, {required bool pop}) async {
  final key = GlobalKey<NavigatorState>();
  await h.mount((_) => Navigator(
        key: key,
        pages: <Page<void>>[
          _TestPage(
            key: const ValueKey<String>('home'),
            child: _heroStack(h, 'hero_from', 10, 20, 40, 40, blue),
          ),
        ],
        onDidRemovePage: (_) {},
      ));
  await h.frames(2);
  key.currentState!.push<void>(_fadeRoute(_heroStack(h, 'hero_to', 100, 150, 80, 60, orange), 300, 300));
  await h.frames(22);
  if (pop) {
    key.currentState!.pop<void>();
    await h.frames(22);
  }
}

class _RootPage extends Page<void> {
  const _RootPage(this.h) : super(key: const ValueKey<String>('root'));
  final FrameHarness h;

  @override
  Route<void> createRoute(BuildContext context) => PageRouteBuilder<void>(
        settings: this,
        transitionDuration: Duration.zero,
        reverseTransitionDuration: Duration.zero,
        pageBuilder: (context, animation, secondaryAnimation) => _probeBox(h, 'below', blue),
        transitionsBuilder: (context, animation, secondaryAnimation, child) => FadeTransition(
          opacity: Tween<double>(begin: 1, end: 0.5).animate(secondaryAnimation),
          child: child,
        ),
      );
}

class _OrderObserver extends NavigatorObserver {
  _OrderObserver(this.record);
  final void Function(double code) record;

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) => record(1);

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) => record(2);

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) => record(3);
}
