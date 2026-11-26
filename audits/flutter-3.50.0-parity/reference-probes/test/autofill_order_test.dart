import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Field extends StatefulWidget {
  const _Field(this.hint, {super.key});

  final String hint;

  @override
  State<_Field> createState() => _FieldState();
}

class _FieldState extends State<_Field> {
  final controller = TextEditingController();
  final focus = FocusNode();

  @override
  void dispose() {
    controller.dispose();
    focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return EditableText(
      controller: controller,
      focusNode: focus,
      autofillHints: [widget.hint],
      style: const TextStyle(),
      cursorColor: const Color(0xff000000),
      backgroundCursorColor: const Color(0xff777777),
    );
  }
}

List<String> hints(GlobalKey<AutofillGroupState> key) => key
    .currentState!
    .autofillClients
    .map(
      (client) => client
          .textInputConfiguration
          .autofillConfiguration
          .autofillHints
          .single,
    )
    .toList();

void main() {
  testWidgets(
    'autofill order survives visual reorder and appends a reparented client',
    (tester) async {
      final outer = GlobalKey<AutofillGroupState>();
      final inner = GlobalKey<AutofillGroupState>();
      final moving = _Field('moving', key: GlobalKey());
      const first = _Field('first', key: ValueKey('first'));
      const second = _Field('second', key: ValueKey('second'));

      Widget screen(int phase) => WidgetsApp(
        color: const Color(0xff000000),
        builder: (_, _) => AutofillGroup(
          key: outer,
          child: Column(
            children: [
              if (phase == 2) moving,
              if (phase == 0) first else second,
              if (phase == 0) second else first,
              AutofillGroup(
                key: inner,
                child: phase < 2 ? moving : const SizedBox(),
              ),
            ],
          ),
        ),
      );
      await tester.pumpWidget(screen(0));
      expect(hints(outer), ['first', 'second']);
      expect(hints(inner), ['moving']);
      await tester.pumpWidget(screen(1));
      expect(hints(outer), ['first', 'second']);
      await tester.pumpWidget(screen(2));
      expect(hints(outer), ['first', 'second', 'moving']);
      expect(hints(inner), isEmpty);
    },
  );

  testWidgets(
    'removing a globally keyed inner group does not finish its outer context',
    (tester) async {
      final inner = GlobalKey();
      Widget screen(bool exists) => WidgetsApp(
        color: const Color(0xff000000),
        builder: (_, _) => AutofillGroup(
          child: exists
              ? AutofillGroup(
                  key: inner,
                  onDisposeAction: AutofillContextAction.cancel,
                  child: const _Field('inside'),
                )
              : const _Field('outside'),
        ),
      );
      await tester.pumpWidget(screen(true));
      tester.testTextInput.log.clear();
      await tester.pumpWidget(screen(false));
      expect(
        tester.testTextInput.log.where(
          (call) => call.method == 'TextInput.finishAutofillContext',
        ),
        isEmpty,
      );
    },
  );
}
