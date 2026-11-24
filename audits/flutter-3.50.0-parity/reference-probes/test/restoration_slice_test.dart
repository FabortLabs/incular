import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Probe extends StatefulWidget {
  const Probe({super.key});
  @override
  State<Probe> createState() => ProbeState();
}

class ProbeState extends State<Probe> with RestorationMixin {
  final draft = RestorableTextEditingController(text: 'default');
  @override
  String get restorationId => 'probe';
  @override
  void restoreState(RestorationBucket? oldBucket, bool initialRestore) {
    registerForRestoration(draft, 'draft');
  }

  void detach() => unregisterFromRestoration(draft);
  bool get hasDraft => bucket!.contains('draft');
  String? get savedDraft => bucket!.read<String>('draft');
  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
  @override
  void dispose() {
    draft.dispose();
    super.dispose();
  }
}

void main() {
  testWidgets('unregistered mutable draft stays absent', (tester) async {
    final root = RestorationBucket.empty(
      restorationId: 'root',
      debugOwner: null,
    );
    final key = GlobalKey<ProbeState>();
    try {
      await tester.pumpWidget(
        UnmanagedRestorationScope(
          bucket: root,
          child: Probe(key: key),
        ),
      );
      final state = key.currentState!;
      final editor = state.draft.value;
      editor.text = 'before';
      expect(state.hasDraft, isTrue);
      expect(state.savedDraft, 'before');
      state.detach();
      expect(state.hasDraft, isFalse);
      editor.text = 'after';
      expect(editor.text, 'after');
      expect(state.hasDraft, isFalse);
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      root.dispose();
    }
  });
}
