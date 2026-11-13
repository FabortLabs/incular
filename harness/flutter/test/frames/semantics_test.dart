import 'dart:ui' show CheckedState, Tristate;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import 'recorder.dart';

SemanticsOwner semanticsOwnerOf(TestWidgetsFlutterBinding binding) {
  SemanticsOwner? found;
  void visit(PipelineOwner owner) {
    if (owner.semanticsOwner case final SemanticsOwner semanticsOwner?
        when semanticsOwner.rootSemanticsNode != null) {
      found ??= semanticsOwner;
    }
    owner.visitChildren(visit);
  }

  visit(binding.rootPipelineOwner);
  return found!;
}

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
        semanticsOwnerOf(h.tester.binding).rootSemanticsNode!;
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
        semanticsOwnerOf(h.tester.binding).rootSemanticsNode!;
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
        semanticsOwnerOf(h.tester.binding).rootSemanticsNode!;
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
        semanticsOwnerOf(h.tester.binding).rootSemanticsNode!;
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
        semanticsOwnerOf(h.tester.binding).rootSemanticsNode!;
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

    final owner = semanticsOwnerOf(h.tester.binding);
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
    semanticsOwnerOf(h.tester.binding).performAction(
      oldId,
      SemanticsAction.tap,
    );
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__container_label_only', (h) async {
    final semantics = h.tester.ensureSemantics();
    var tapNodeCount = 0.0;
    var tapAction = 0.0;
    var tapIsButton = 0.0;
    var labelOnlyNodeCount = 0.0;
    h.value('semantics.tap_node_count', () => tapNodeCount);
    h.value('semantics.tap_action', () => tapAction);
    h.value('semantics.tap_is_button', () => tapIsButton);
    h.value('semantics.label_only_node_count', () => labelOnlyNodeCount);
    await h.mount(
      (_) => h.probe(
        'subject',
        Row(
          children: [
            Semantics(
              container: true,
              label: 'Plain',
              onTap: () {},
              child: const SizedBox(
                width: 120,
                height: 40,
                child: ColoredBox(color: Color(0xFF2196F3)),
              ),
            ),
            Semantics(
              container: true,
              label: 'Label only',
              child: const SizedBox(
                width: 120,
                height: 40,
                child: ColoredBox(color: Color(0xFF4CAF50)),
              ),
            ),
          ],
        ),
      ),
    );
    void visit(SemanticsNode node) {
      final data = node.getSemanticsData();
      if (data.label == 'Plain') {
        tapNodeCount += 1;
        if (data.hasAction(SemanticsAction.tap)) tapAction = 1;
        if (data.flagsCollection.isButton) tapIsButton = 1;
      }
      if (data.label == 'Label only') labelOnlyNodeCount += 1;
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__opacity_zero', (h) async {
    final semantics = h.tester.ensureSemantics();
    var fadedCount = 0.0;
    h.value('semantics.faded_node_count', () => fadedCount);
    await h.mount(
      (_) => h.probe(
        'subject',
        Opacity(
          opacity: 0,
          child: Semantics(
            container: true,
            label: 'Faded',
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
      if (node.getSemanticsData().label == 'Faded') fadedCount += 1;
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__offstage_visibility', (h) async {
    final semantics = h.tester.ensureSemantics();
    var offstageCount = 0.0;
    var invisibleCount = 0.0;
    var maintainedCount = 0.0;
    h.value('semantics.offstage_node_count', () => offstageCount);
    h.value('semantics.invisible_node_count', () => invisibleCount);
    h.value('semantics.maintained_node_count', () => maintainedCount);
    Widget labelled(String label, Color color) => Semantics(
      container: true,
      label: label,
      button: true,
      child: SizedBox(
        width: 60,
        height: 40,
        child: ColoredBox(color: color),
      ),
    );
    await h.mount(
      (_) => h.probe(
        'subject',
        Row(
          children: [
            Offstage(
              offstage: true,
              child: labelled('Offstage', const Color(0xFF2196F3)),
            ),
            Visibility(
              visible: false,
              child: labelled('Invisible', const Color(0xFF4CAF50)),
            ),
            Visibility(
              visible: false,
              maintainState: true,
              maintainAnimation: true,
              maintainSize: true,
              maintainSemantics: true,
              child: labelled('Maintained', const Color(0xFFF44336)),
            ),
          ],
        ),
      ),
    );
    void visit(SemanticsNode node) {
      final label = node.getSemanticsData().label;
      if (label == 'Offstage') offstageCount += 1;
      if (label == 'Invisible') invisibleCount += 1;
      if (label == 'Maintained') maintainedCount += 1;
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__value_hint_tooltip', (h) async {
    final semantics = h.tester.ensureSemantics();
    var valueLength = 0.0;
    var hintLength = 0.0;
    var tooltipLength = 0.0;
    h.value('semantics.value_length', () => valueLength);
    h.value('semantics.hint_length', () => hintLength);
    h.value('semantics.tooltip_length', () => tooltipLength);
    await h.mount(
      (_) => h.probe(
        'subject',
        Semantics(
          container: true,
          label: 'Slider',
          value: '50%',
          hint: 'Adjusts volume',
          tooltip: 'Volume',
          button: true,
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );
    final data = h.tester
        .getSemantics(find.bySemanticsLabel('Slider'))
        .getSemanticsData();
    valueLength = data.value.length.toDouble();
    hintLength = data.hint.length.toDouble();
    tooltipLength = data.tooltip.length.toDouble();
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__increase_decrease', (h) async {
    final semantics = h.tester.ensureSemantics();
    var increaseCalls = 0.0;
    var decreaseCalls = 0.0;
    var hasIncrease = 0.0;
    var hasDecrease = 0.0;
    h.value('semantics.increase_calls', () => increaseCalls);
    h.value('semantics.decrease_calls', () => decreaseCalls);
    h.value('semantics.has_increase', () => hasIncrease);
    h.value('semantics.has_decrease', () => hasDecrease);
    await h.mount(
      (_) => h.probe(
        'subject',
        Semantics(
          container: true,
          label: 'Stepper',
          value: '5',
          increasedValue: '6',
          decreasedValue: '4',
          slider: true,
          onIncrease: () => increaseCalls += 1,
          onDecrease: () => decreaseCalls += 1,
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );
    final stepper = h.tester.getSemantics(find.bySemanticsLabel('Stepper'));
    final data = stepper.getSemanticsData();
    hasIncrease = data.hasAction(SemanticsAction.increase) ? 1 : 0;
    hasDecrease = data.hasAction(SemanticsAction.decrease) ? 1 : 0;
    await h.frames(1);
    final id = stepper.id;
    semanticsOwnerOf(h.tester.binding).performAction(
      id,
      SemanticsAction.increase,
    );
    semanticsOwnerOf(h.tester.binding).performAction(
      id,
      SemanticsAction.decrease,
    );
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__roles_flags', (h) async {
    final semantics = h.tester.ensureSemantics();
    var headerCount = 0.0;
    var linkCount = 0.0;
    var imageCount = 0.0;
    var toggledCount = 0.0;
    var hiddenCount = 0.0;
    h.value('semantics.header_count', () => headerCount);
    h.value('semantics.link_count', () => linkCount);
    h.value('semantics.image_count', () => imageCount);
    h.value('semantics.toggled_count', () => toggledCount);
    h.value('semantics.hidden_count', () => hiddenCount);
    Widget box(Color color) => SizedBox(
      width: 60,
      height: 40,
      child: ColoredBox(color: color),
    );
    await h.mount(
      (_) => h.probe(
        'subject',
        Column(
          children: [
            Semantics(
              container: true,
              header: true,
              label: 'Heading',
              child: box(const Color(0xFF2196F3)),
            ),
            Semantics(
              container: true,
              link: true,
              label: 'Link',
              child: box(const Color(0xFF4CAF50)),
            ),
            Semantics(
              container: true,
              image: true,
              label: 'Picture',
              child: box(const Color(0xFFFF9800)),
            ),
            Semantics(
              container: true,
              button: true,
              toggled: true,
              label: 'Toggle',
              child: box(const Color(0xFF9C27B0)),
            ),
            Semantics(
              container: true,
              button: true,
              hidden: true,
              label: 'Concealed',
              child: box(const Color(0xFFF44336)),
            ),
          ],
        ),
      ),
    );
    void visit(SemanticsNode node) {
      final data = node.getSemanticsData();
      if (data.flagsCollection.isHeader) headerCount += 1;
      if (data.flagsCollection.isLink) linkCount += 1;
      if (data.flagsCollection.isImage) imageCount += 1;
      if (data.flagsCollection.isToggled == Tristate.isTrue) toggledCount += 1;
      if (data.flagsCollection.isHidden) hiddenCount += 1;
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__gesture_tap', (h) async {
    final semantics = h.tester.ensureSemantics();
    var tapNodeCount = 0.0;
    var tapCalls = 0;
    h.value('semantics.tap_node_count', () => tapNodeCount);
    h.value('semantics.tap_calls', () => tapCalls.toDouble());
    await h.mount(
      (_) => h.probe(
        'subject',
        GestureDetector(
          onTap: () => tapCalls += 1,
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );
    int? tapId;
    void visit(SemanticsNode node) {
      if (node.getSemanticsData().hasAction(SemanticsAction.tap)) {
        tapNodeCount += 1;
        tapId ??= node.id;
      }
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    if (tapId != null) {
      semanticsOwnerOf(h.tester.binding).performAction(
        tapId!,
        SemanticsAction.tap,
      );
    }
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__live_region', (h) async {
    final semantics = h.tester.ensureSemantics();
    var liveRegionCount = 0.0;
    h.value('semantics.live_region_count', () => liveRegionCount);
    await h.mount(
      (_) => h.probe(
        'subject',
        Semantics(
          container: true,
          liveRegion: true,
          label: 'Status',
          child: const SizedBox(
            width: 120,
            height: 40,
            child: ColoredBox(color: Color(0xFF2196F3)),
          ),
        ),
      ),
    );
    void visit(SemanticsNode node) {
      if (node.getSemanticsData().flagsCollection.isLiveRegion) {
        liveRegionCount += 1;
      }
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__horizontal_scroll', (h) async {
    final semantics = h.tester.ensureSemantics();
    final controller = ScrollController();
    addTearDown(controller.dispose);
    var scrollActionCount = 0.0;
    var scrollPosition = -1.0;
    var scrollExtentMax = -1.0;
    h.value('semantics.scroll_action_count', () => scrollActionCount);
    h.value('semantics.scroll_position', () => scrollPosition);
    h.value('semantics.scroll_extent_max', () => scrollExtentMax);
    await h.mount(
      (_) => h.probe(
        'subject',
        SizedBox(
          height: 100,
          child: ListView.builder(
            scrollDirection: Axis.horizontal,
            controller: controller,
            itemCount: 12,
            itemBuilder: (_, index) => Semantics(
              container: true,
              label: 'Column $index',
              button: true,
              child: SizedBox(
                width: 100,
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
      ),
    );
    void sample(SemanticsNode node) {
      final data = node.getSemanticsData();
      if (data.scrollExtentMax != null) {
        scrollPosition = data.scrollPosition ?? -1;
        scrollExtentMax = data.scrollExtentMax ?? -1;
        scrollActionCount = 0;
        if (data.hasAction(SemanticsAction.scrollLeft)) scrollActionCount += 1;
        if (data.hasAction(SemanticsAction.scrollRight)) {
          scrollActionCount += 1;
        }
      }
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        sample(child);
        return true;
      });
    }
    final root = semanticsOwnerOf(h.tester.binding).rootSemanticsNode!;
    sample(root);
    await h.frames(1);
    controller.jumpTo(controller.position.maxScrollExtent);
    await h.frames(1);
    sample(root);
    await h.frames(1);
    semantics.dispose();
  });

  frameScenario('semantics__set_text', (h) async {
    final semantics = h.tester.ensureSemantics();
    final controller = TextEditingController();
    final focusNode = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focusNode.dispose);
    h.value('semantics.text_length', () => controller.text.length.toDouble());
    var setTextNodes = 0.0;
    h.value('semantics.set_text_nodes', () => setTextNodes);
    await h.mount(
      (_) => h.probe(
        'subject',
        SizedBox(
          width: 200,
          height: 48,
          child: EditableText(
            controller: controller,
            focusNode: focusNode,
            style: const TextStyle(
              fontFamily: 'Inter',
              fontSize: 14,
              color: Color(0xFF000000),
            ),
            cursorColor: const Color(0xFF000000),
            backgroundCursorColor: const Color(0xFF9E9E9E),
          ),
        ),
      ),
    );
    int? id;
    void visit(SemanticsNode node) {
      if (node.getSemanticsData().hasAction(SemanticsAction.setText)) {
        setTextNodes += 1;
        id ??= node.id;
      }
      if (node.mergeAllDescendantsIntoThisNode) return;
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }
    visit(semanticsOwnerOf(h.tester.binding).rootSemanticsNode!);
    await h.frames(1);
    if (id != null) {
      semanticsOwnerOf(h.tester.binding).performAction(
        id!,
        SemanticsAction.setText,
        'hello',
      );
    }
    await h.frames(1);
    semantics.dispose();
  });
}
