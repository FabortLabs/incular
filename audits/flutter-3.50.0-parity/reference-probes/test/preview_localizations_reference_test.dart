import 'package:flutter/foundation.dart';
import 'package:flutter/widget_previews.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Greeting {
  const _Greeting(this.message);
  final String message;
}

class _GreetingDelegate extends LocalizationsDelegate<_Greeting> {
  const _GreetingDelegate();

  @override
  bool isSupported(Locale locale) =>
      const {'en', 'es', 'ar'}.contains(locale.languageCode);

  @override
  Future<_Greeting> load(Locale locale) => SynchronousFuture(
    _Greeting(
      const {'en': 'Hello', 'es': 'Hola', 'ar': 'مرحبا'}[locale.languageCode]!,
    ),
  );

  @override
  bool shouldReload(_GreetingDelegate old) => false;
}

class _Direction extends DefaultWidgetsLocalizations {
  const _Direction(this.textDirection);

  @override
  final TextDirection textDirection;
}

class _DirectionDelegate extends LocalizationsDelegate<WidgetsLocalizations> {
  const _DirectionDelegate();

  @override
  bool isSupported(Locale locale) => true;

  @override
  Future<WidgetsLocalizations> load(Locale locale) => SynchronousFuture(
    _Direction(
      locale.languageCode == 'ar' ? TextDirection.rtl : TextDirection.ltr,
    ),
  );

  @override
  bool shouldReload(_DirectionDelegate old) => false;
}

const _locales = [Locale('en'), Locale('es'), Locale('ar')];
const _delegates = <LocalizationsDelegate<Object?>>[
  _GreetingDelegate(),
  _DirectionDelegate(),
];

LocalizationsResolver _makeResolver(PreviewLocalizationsData data) =>
    LocalizationsResolver(
      supportedLocales: data.supportedLocales,
      locale: data.locale,
      localeListResolutionCallback: data.localeListResolutionCallback,
      localeResolutionCallback: data.localeResolutionCallback,
      localizationsDelegates: data.localizationsDelegates,
    );

void main() {
  test('basic fallback distinguishes deferred language and region matches', () {
    const enUs = Locale('en', 'US');
    const zh = Locale('zh');
    const zhCn = Locale('zh', 'CN');
    const zhTw = Locale.fromSubtags(
      languageCode: 'zh',
      scriptCode: 'Hant',
      countryCode: 'TW',
    );
    const zhHk = Locale.fromSubtags(
      languageCode: 'zh',
      scriptCode: 'Hant',
      countryCode: 'HK',
    );
    const zhMo = Locale.fromSubtags(
      languageCode: 'zh',
      scriptCode: 'Hant',
      countryCode: 'MO',
    );
    const de = Locale('de');
    const frTn = Locale('fr', 'TN');
    final cases = <(List<Locale>, List<Locale>, Locale)>[
      ([zh, zhTw], [zhCn, zhTw], zhTw),
      ([zh, enUs], [zhCn, enUs], zhCn),
      ([de, zh, enUs], [zhCn, enUs], enUs),
      ([de, zh, Locale('it'), enUs], [enUs, zhCn], zhCn),
      ([de, Locale('ar', 'TN')], [enUs, frTn], frTn),
      ([zhHk], [zhTw, zhMo], zhTw),
      ([], [Locale('fr', 'CA'), enUs], Locale('fr', 'CA')),
    ];
    for (final (preferred, supported, expected) in cases) {
      expect(basicLocaleListResolution(preferred, supported), expected);
    }
  });

  testWidgets('preview locale and resources update below a retained child', (
    tester,
  ) async {
    final platform = tester.binding.platformDispatcher;
    platform.localesTestValue = [_locales.first];
    addTearDown(platform.clearLocalesTestValue);
    var calls = 0;
    final preview = Preview(
      localizations: () {
        calls++;
        return const PreviewLocalizationsData(
          supportedLocales: _locales,
          localizationsDelegates: _delegates,
        );
      },
    );
    expect(calls, 0);
    final resolver = _makeResolver(preview.localizations!());
    addTearDown(resolver.dispose);
    const key = ValueKey('preview child');
    final log = <(Locale, String, TextDirection)>[];
    await tester.pumpWidget(
      ListenableBuilder(
        listenable: resolver,
        child: Builder(
          builder: (context) {
            log.add((
              Localizations.localeOf(context),
              Localizations.of<_Greeting>(context, _Greeting)!.message,
              Directionality.of(context),
            ));
            return const SizedBox(key: key);
          },
        ),
        builder: (context, child) => Localizations(
          locale: resolver.locale,
          delegates: resolver.localizationsDelegates.toList(),
          child: child!,
        ),
      ),
    );
    final retained = tester.element(find.byKey(key));
    for (final locale in _locales.skip(1)) {
      platform.localesTestValue = [locale];
      await tester.pump();
      expect(tester.element(find.byKey(key)), same(retained));
    }
    expect(log, [
      (_locales[0], 'Hello', TextDirection.ltr),
      (_locales[1], 'Hola', TextDirection.ltr),
      (_locales[2], 'مرحبا', TextDirection.rtl),
    ]);
    expect(calls, 1);
  });

  testWidgets('preview explicit locale uses list then single locale override', (
    tester,
  ) async {
    final platform = tester.binding.platformDispatcher;
    platform.localesTestValue = [_locales.first];
    addTearDown(platform.clearLocalesTestValue);
    final calls = <(String, Locale?)>[];
    var overrideList = false;
    final resolver = _makeResolver(
      PreviewLocalizationsData(
        locale: _locales[1],
        supportedLocales: _locales,
        localizationsDelegates: _delegates,
        localeListResolutionCallback: (preferred, supported) {
          calls.add(('list', preferred?.first));
          return overrideList ? preferred?.first : null;
        },
        localeResolutionCallback: (preferred, supported) {
          calls.add(('single', preferred));
          return _locales[2];
        },
      ),
    );
    addTearDown(resolver.dispose);
    calls.clear();
    expect(resolver.locale, _locales[2]);
    expect(calls, [('list', _locales[1]), ('single', _locales[1])]);
    overrideList = true;
    calls.clear();
    expect(resolver.locale, _locales[1]);
    expect(calls, [('list', _locales[1])]);
    expect(const PreviewLocalizationsData().supportedLocales, const [
      Locale('en', 'US'),
    ]);
  });
}
