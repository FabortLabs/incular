import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('smart punctuation equality uses resolved defaults and copyWith preserves them', () {
    const implicit = TextInputConfiguration();
    const explicit = TextInputConfiguration(
      smartDashesType: SmartDashesType.enabled,
      smartQuotesType: SmartQuotesType.enabled,
    );
    expect(implicit, explicit);
    expect(implicit.hashCode, explicit.hashCode);

    final copied = implicit.copyWith(obscureText: true);
    expect(copied.obscureText, isTrue);
    expect(copied.smartDashesType, SmartDashesType.enabled);
    expect(copied.smartQuotesType, SmartQuotesType.enabled);
    const fresh = TextInputConfiguration(obscureText: true);
    expect(fresh.smartDashesType, SmartDashesType.disabled);
    expect(fresh.smartQuotesType, SmartQuotesType.disabled);
    expect(copied, isNot(fresh));
  });
}
