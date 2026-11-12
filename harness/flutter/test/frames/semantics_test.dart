import 'dart:ui' show CheckedState, Tristate;

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

import 'recorder.dart';

void main() {
  frameScenario('semantics__properties_state', (h) async {
    final semantics = h.tester.ensureSemantics();
    var labelLength = 0.0;
    var isButton = 0.0;
    var isEnabled = 0.0;
    var isSelected = 0.0;
    var isChecked = 0.0;
    var hasTap = 0.0;
    h.value('semantics.label_length', () => labelLength);
    h.value('semantics.button', () => isButton);
    h.value('semantics.enabled', () => isEnabled);
    h.value('semantics.selected', () => isSelected);
    h.value('semantics.checked', () => isChecked);
    h.value('semantics.tap_action', () => hasTap);

    await h.mount(
      (_) => h.probe(
        'subject',
        Semantics(
          container: true,
          label: 'Ready',
          button: true,
          enabled: true,
          selected: true,
          checked: true,
          onTap: () {},
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );

    final data = h.tester
        .getSemantics(find.bySemanticsLabel('Ready'))
        .getSemanticsData();
    labelLength = data.label.length.toDouble();
    isButton = data.flagsCollection.isButton ? 1 : 0;
    isEnabled = data.flagsCollection.isEnabled == Tristate.isTrue ? 1 : 0;
    isSelected = data.flagsCollection.isSelected == Tristate.isTrue ? 1 : 0;
    isChecked = data.flagsCollection.isChecked == CheckedState.isTrue ? 1 : 0;
    hasTap = data.hasAction(SemanticsAction.tap) ? 1 : 0;
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__enabled_default', (h) async {
    final semantics = h.tester.ensureSemantics();
    var enabledState = 0.0;
    var hasTap = 0.0;
    h.value('semantics.enabled_state', () => enabledState);
    h.value('semantics.tap_action', () => hasTap);

    await h.mount(
      (_) => h.probe(
        'subject',
        Semantics(
          container: true,
          label: 'Action',
          button: true,
          onTap: () {},
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );

    final data = h.tester
        .getSemantics(find.bySemanticsLabel('Action'))
        .getSemanticsData();
    enabledState = switch (data.flagsCollection.isEnabled) {
      Tristate.isTrue => 1,
      Tristate.isFalse => 0,
      Tristate.none => 2,
    };
    hasTap = data.hasAction(SemanticsAction.tap) ? 1 : 0;
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__exclude', (h) async {
    final semantics = h.tester.ensureSemantics();
    var visibleLabel = 0.0;
    var hiddenLabel = 0.0;
    h.value('semantics.visible_label', () => visibleLabel);
    h.value('semantics.hidden_label', () => hiddenLabel);

    await h.mount(
      (_) => h.probe(
        'subject',
        Row(
          children: [
            Semantics(
              container: true,
              label: 'Visible',
              button: true,
              child: const SizedBox(
                width: 60,
                height: 40,
                child: ColoredBox(color: Color(0xFF2196F3)),
              ),
            ),
            ExcludeSemantics(
              child: Semantics(
                container: true,
                label: 'Hidden',
                button: true,
                child: const SizedBox(
                  width: 60,
                  height: 40,
                  child: ColoredBox(color: Color(0xFF4CAF50)),
                ),
              ),
            ),
          ],
        ),
      ),
    );

    final labels = <String>[];
    void visit(SemanticsNode node) {
      final label = node.getSemanticsData().label;
      if (label.isNotEmpty) labels.add(label);
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }

    final root =
        h.tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
    visit(root);
    visibleLabel = labels
        .where((label) => label == 'Visible')
        .length
        .toDouble();
    hiddenLabel = labels.where((label) => label == 'Hidden').length.toDouble();
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__merge', (h) async {
    final semantics = h.tester.ensureSemantics();
    var semanticNodeCount = 0.0;
    var mergedLabelLength = 0.0;
    var mergedLabelCount = 0.0;
    h.value('semantics.node_count', () => semanticNodeCount);
    h.value('semantics.merged_label_length', () => mergedLabelLength);
    h.value('semantics.merged_label_count', () => mergedLabelCount);

    await h.mount(
      (_) => h.probe(
        'subject',
        MergeSemantics(
          child: Row(
            children: [
              Semantics(
                container: true,
                label: 'North',
                button: true,
                child: const SizedBox(
                  width: 60,
                  height: 40,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              ),
              Semantics(
                container: true,
                label: 'South',
                button: true,
                child: const SizedBox(
                  width: 60,
                  height: 40,
                  child: ColoredBox(color: Color(0xFF4CAF50)),
                ),
              ),
            ],
          ),
        ),
      ),
    );

    final labels = <String>[];
    void visit(SemanticsNode node, {bool isRoot = false}) {
      if (!isRoot) semanticNodeCount += 1;
      final label = node.getSemanticsData().label;
      if (label.isNotEmpty) labels.add(label);
      // A merge boundary serializes as one leaf. Its descendants are retained
      // internally for merged data and action routing, but are not separate
      // nodes in the platform tree.
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }

    final root =
        h.tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
    visit(root, isRoot: true);
    const merged = 'North\nSouth';
    mergedLabelLength = labels.where((label) => label == merged).isNotEmpty
        ? merged.length.toDouble()
        : 0;
    mergedLabelCount = labels
        .where((label) => label == merged)
        .length
        .toDouble();
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__block_previous', (h) async {
    final semantics = h.tester.ensureSemantics();
    var behindCount = 0.0;
    var dialogCount = 0.0;
    h.value('semantics.behind_label', () => behindCount);
    h.value('semantics.dialog_label', () => dialogCount);

    await h.mount(
      (_) => h.probe(
        'subject',
        Row(
          children: [
            Semantics(
              container: true,
              label: 'Behind',
              button: true,
              child: const SizedBox(
                width: 60,
                height: 40,
                child: ColoredBox(color: Color(0xFF2196F3)),
              ),
            ),
            BlockSemantics(
              child: Semantics(
                container: true,
                label: 'Dialog',
                button: true,
                child: const SizedBox(
                  width: 60,
                  height: 40,
                  child: ColoredBox(color: Color(0xFF4CAF50)),
                ),
              ),
            ),
          ],
        ),
      ),
    );

    final labels = <String>[];
    void visit(SemanticsNode node) {
      final label = node.getSemanticsData().label;
      if (label.isNotEmpty) labels.add(label);
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }

    final root =
        h.tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
    visit(root);
    behindCount = labels.where((label) => label == 'Behind').length.toDouble();
    dialogCount = labels.where((label) => label == 'Dialog').length.toDouble();
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__indexed', (h) async {
    final semantics = h.tester.ensureSemantics();
    var indexInParent = -1.0;
    h.value('semantics.index_in_parent', () => indexInParent);

    await h.mount(
      (_) => h.probe(
        'subject',
        IndexedSemantics(
          index: 7,
          child: Semantics(
            label: 'Indexed item',
            button: true,
            child: const SizedBox(
              width: 120,
              height: 40,
              child: ColoredBox(color: Color(0xFF2196F3)),
            ),
          ),
        ),
      ),
    );

    void visit(SemanticsNode node) {
      if (node.indexInParent != null) {
        indexInParent = node.indexInParent!.toDouble();
      }
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }

    final root =
        h.tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
    visit(root);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__scrollable', (h) async {
    final semantics = h.tester.ensureSemantics();
    final controller = ScrollController();
    addTearDown(controller.dispose);
    var scrollNodeCount = 0.0;
    var scrollActionCount = 0.0;
    var scrollUpAction = 0.0;
    var scrollDownAction = 0.0;
    var scrollPosition = -1.0;
    var scrollExtentMax = -1.0;
    var scrollChildCount = -1.0;
    h.value('semantics.scroll_node_count', () => scrollNodeCount);
    h.value('semantics.scroll_action_count', () => scrollActionCount);
    h.value('semantics.scroll_up_action', () => scrollUpAction);
    h.value('semantics.scroll_down_action', () => scrollDownAction);
    h.value('semantics.scroll_position', () => scrollPosition);
    h.value('semantics.scroll_extent_max', () => scrollExtentMax);
    h.value('semantics.scroll_child_count', () => scrollChildCount);

    await h.mount(
      (_) => h.probe(
        'subject',
        ListView.builder(
          controller: controller,
          itemCount: 12,
          itemBuilder: (_, index) => Semantics(
            container: true,
            label: 'Row $index',
            button: true,
            child: SizedBox(
              width: 800,
              height: 100,
              child: ColoredBox(
                color: index.isEven
                    ? const Color(0xFF2196F3)
                    : const Color(0xFF4CAF50),
              ),
            ),
          ),
        ),
      ),
    );

    void sample(SemanticsNode node) {
      final data = node.getSemanticsData();
      if (data.scrollExtentMax != null) {
        scrollNodeCount += 1;
        scrollPosition = data.scrollPosition ?? -1;
        scrollExtentMax = data.scrollExtentMax ?? -1;
        scrollChildCount = data.scrollChildCount?.toDouble() ?? -1;
        scrollUpAction = data.hasAction(SemanticsAction.scrollUp) ? 1 : 0;
        scrollDownAction = data.hasAction(SemanticsAction.scrollDown) ? 1 : 0;
        scrollActionCount = scrollUpAction + scrollDownAction;
      }
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        sample(child);
        return true;
      });
    }

    final root =
        h.tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
    sample(root);
    await h.frames(1);
    controller.jumpTo(controller.position.maxScrollExtent);
    await h.frames(1);
    scrollNodeCount = 0;
    scrollActionCount = 0;
    scrollUpAction = 0;
    scrollDownAction = 0;
    scrollPosition = -1;
    scrollExtentMax = -1;
    scrollChildCount = -1;
    sample(root);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__action_dispatch', (h) async {
    final semantics = h.tester.ensureSemantics();
    var calls = 0.0;
    h.value('semantics.action_calls', () => calls);
    await h.mount(
      (_) => h.probe(
        'subject',
        Semantics(
          container: true,
          label: 'Activate',
          button: true,
          onTap: () => calls += 1,
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );

    final owner = h.tester.binding.pipelineOwner.semanticsOwner!;
    final node = h.tester.getSemantics(find.bySemanticsLabel('Activate'));
    owner.performAction(node.id, SemanticsAction.tap);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__stale_action', (h) async {
    final semantics = h.tester.ensureSemantics();
    var calls = 0.0;
    h.value('semantics.stale_action_calls', () => calls);
    await h.mount(
      (phase) => h.probe(
        'subject',
        phase == 0
            ? Semantics(
                container: true,
                label: 'Temporary action',
                button: true,
                onTap: () => calls += 1,
                child: const SizedBox(
                  width: 120,
                  height: 40,
                  child: ColoredBox(color: Color(0xFF2196F3)),
                ),
              )
            : const ColoredBox(
                color: Color(0xFF2196F3),
                child: SizedBox.expand(),
              ),
      ),
    );

    final oldId = h.tester
        .getSemantics(find.bySemanticsLabel('Temporary action'))
        .id;
    await h.frames(1);
    h.setPhase(1);
    await h.frames(1);
    h.tester.binding.pipelineOwner.semanticsOwner!.performAction(
      oldId,
      SemanticsAction.tap,
    );
    await h.frames(1);
    semantics.dispose();
  });
}
