import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'context uses the nearest focus boundary and then its actual focus scope',
    (tester) async {
      final left = FocusScopeNode(debugLabel: 'left');
      final right = FocusScopeNode(debugLabel: 'right');
      final branch = FocusNode(debugLabel: 'branch');
      final nestedScope = FocusScopeNode(debugLabel: 'nested');
      for (final node in <FocusNode>[left, right, branch, nestedScope]) {
        addTearDown(node.dispose);
      }
      final insideBranch = GlobalKey();
      final outsideBranch = GlobalKey();
      final insideNestedScope = GlobalKey();

      Future<void> mount(FocusScopeNode parent) async {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Column(
              children: <Widget>[
                FocusScope(
                  node: left,
                  child: Column(
                    children: <Widget>[
                      Focus(
                        focusNode: branch,
                        parentNode: parent,
                        child: SizedBox(key: insideBranch),
                      ),
                      SizedBox(key: outsideBranch),
                      FocusScope(
                        node: nestedScope,
                        parentNode: right,
                        child: SizedBox(key: insideNestedScope),
                      ),
                    ],
                  ),
                ),
                FocusScope(node: right, child: const SizedBox()),
              ],
            ),
          ),
        );
      }

      await mount(right);
      expect(Focus.of(insideBranch.currentContext!), same(branch));
      expect(branch.enclosingScope, same(right));
      expect(FocusScope.of(insideBranch.currentContext!), same(right));
      expect(FocusScope.of(outsideBranch.currentContext!), same(left));
      expect(
        FocusScope.of(insideNestedScope.currentContext!),
        same(nestedScope),
      );

      await mount(left);
      expect(branch.enclosingScope, same(left));
      expect(FocusScope.of(insideBranch.currentContext!), same(left));
      expect(FocusScope.of(outsideBranch.currentContext!), same(left));
      expect(
        FocusScope.of(insideNestedScope.currentContext!),
        same(nestedScope),
      );
    },
  );
}
