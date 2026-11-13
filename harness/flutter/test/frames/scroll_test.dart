import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);
const Color green = Color(0xFF4CAF50);

void main() {
  frameScenario('scroll__controller__jump_clamp', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    h.value('max', () => controller.hasClients ? controller.position.maxScrollExtent : 0);
    await h.mount((_) => ListView.builder(
          controller: controller,
          itemExtent: 50,
          itemCount: 40,
          itemBuilder: (context, index) {
            final row = ColoredBox(color: index.isEven ? blue : green);
            return index == 0 || index == 5 ? h.probe('row$index', row) : row;
          },
        ));
    await h.frames(1);
    controller.jumpTo(125);
    await h.frames(1);
    controller.jumpTo(100000);
    await h.frames(3);
  });

  frameScenario('scroll__controller__jump_activity_notifications', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    var starts = 0;
    var updates = 0;
    var ends = 0;
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    h.value('notifications.start', () => starts.toDouble());
    h.value('notifications.update', () => updates.toDouble());
    h.value('notifications.end', () => ends.toDouble());
    await h.mount((_) => NotificationListener<ScrollNotification>(
          onNotification: (notification) {
            if (notification is ScrollStartNotification) starts++;
            if (notification is ScrollUpdateNotification) updates++;
            if (notification is ScrollEndNotification) ends++;
            return false;
          },
          child: ListView.builder(
            controller: controller,
            itemExtent: 50,
            itemCount: 40,
            itemBuilder: (context, index) => ColoredBox(color: index.isEven ? blue : green),
          ),
        ));
    await h.frames(1);
    controller.jumpTo(125);
    await h.frames(1);
  });

  frameScenario('scroll__controller__animate_retarget', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    await h.mount((_) => ListView.builder(
          controller: controller,
          itemExtent: 50,
          itemCount: 80,
          itemBuilder: (context, index) {
            final row = ColoredBox(color: index.isEven ? blue : orange);
            return index == 0 || index == 8 ? h.probe('row$index', row) : row;
          },
        ));
    await h.frames(1);
    controller.animateTo(600, duration: const Duration(milliseconds: 320), curve: Curves.fastOutSlowIn);
    await h.frames(10);
    controller.animateTo(250, duration: const Duration(milliseconds: 160), curve: Curves.linear);
    await h.frames(12);
  });

  frameScenario('scroll__page_view__fraction_initial_page', (h) async {
    final controller = PageController(initialPage: 1, viewportFraction: 0.5);
    addTearDown(controller.dispose);
    h.value('page', () => controller.hasClients ? controller.page ?? 0 : 0);
    await h.mount((_) => PageView(
          controller: controller,
          children: <Widget>[
            h.probe('page0', const ColoredBox(color: blue)),
            h.probe('page1', const ColoredBox(color: orange)),
            h.probe('page2', const ColoredBox(color: green)),
          ],
        ));
    await h.frames(3);
  });

  frameScenario('scroll__controller__animate_activity_notifications', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    var starts = 0;
    var updates = 0;
    var ends = 0;
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    h.value('notifications.start', () => starts.toDouble());
    h.value('notifications.update', () => updates.toDouble());
    h.value('notifications.end', () => ends.toDouble());
    await h.mount((_) => NotificationListener<ScrollNotification>(
          onNotification: (notification) {
            if (notification is ScrollStartNotification) starts++;
            if (notification is ScrollUpdateNotification) updates++;
            if (notification is ScrollEndNotification) ends++;
            return false;
          },
          child: ListView.builder(
            controller: controller,
            itemExtent: 50,
            itemCount: 40,
            itemBuilder: (context, index) => ColoredBox(color: index.isEven ? blue : green),
          ),
        ));
    await h.frames(1);
    controller.animateTo(300, duration: const Duration(milliseconds: 160), curve: Curves.linear);
    await h.frames(8);
  });

  frameScenario('scroll__drag__notification_order', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final counts = List<int>.filled(5, 0);
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    h.value('notifications.start', () => counts[0].toDouble());
    h.value('notifications.update', () => counts[1].toDouble());
    h.value('notifications.overscroll', () => counts[2].toDouble());
    h.value('notifications.end', () => counts[3].toDouble());
    h.value('notifications.user_scroll', () => counts[4].toDouble());
    await h.mount((_) => _countingList(controller, counts, const ClampingScrollPhysics()));
    await h.frames(1);
    await _drag(h, const Offset(400, 500), const Offset(400, 200), 5, framesPerStep: 8);
    await h.frames(20);
  });

  frameScenario('scroll__fling__notification_order', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final counts = List<int>.filled(5, 0);
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    h.value('notifications.start', () => counts[0].toDouble());
    h.value('notifications.update', () => counts[1].toDouble());
    h.value('notifications.overscroll', () => counts[2].toDouble());
    h.value('notifications.end', () => counts[3].toDouble());
    h.value('notifications.user_scroll', () => counts[4].toDouble());
    await h.mount((_) => _countingList(controller, counts, const ClampingScrollPhysics()));
    await h.frames(1);
    await _drag(h, const Offset(400, 500), const Offset(400, 200), 3, framesPerStep: 1);
    await h.frames(90);
  });

  for (final physics in ['clamping', 'bouncing']) {
    frameScenario('scroll__drag_overscroll__$physics', (h) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final counts = List<int>.filled(5, 0);
      h.value('offset', () => controller.hasClients ? controller.offset : 0);
      h.value('notifications.start', () => counts[0].toDouble());
      h.value('notifications.update', () => counts[1].toDouble());
      h.value('notifications.overscroll', () => counts[2].toDouble());
      h.value('notifications.end', () => counts[3].toDouble());
      h.value('notifications.user_scroll', () => counts[4].toDouble());
      await h.mount((_) => _countingList(
            controller,
            counts,
            physics == 'clamping' ? const ClampingScrollPhysics() : const BouncingScrollPhysics(),
          ));
      await h.frames(1);
      await _drag(h, const Offset(400, 200), const Offset(400, 400), 4, framesPerStep: 1);
      await h.frames(30);
    });
  }

  frameScenario('scroll__page_view__fling_page_changed', (h) async {
    final controller = PageController();
    addTearDown(controller.dispose);
    var changes = 0;
    var lastPage = 0;
    h.value('page', () => controller.hasClients ? controller.page ?? 0 : 0);
    h.value('page_changes', () => changes.toDouble());
    h.value('last_page', () => lastPage.toDouble());
    await h.mount((_) => PageView(
          controller: controller,
          onPageChanged: (page) {
            changes++;
            lastPage = page;
          },
          children: <Widget>[
            h.probe('page0', const ColoredBox(color: blue)),
            h.probe('page1', const ColoredBox(color: orange)),
            h.probe('page2', const ColoredBox(color: green)),
          ],
        ));
    await h.frames(1);
    await _drag(h, const Offset(600, 300), const Offset(150, 300), 5, framesPerStep: 1);
    await h.frames(60);
  });

  frameScenario('scroll__list_wheel__selected_fling', (h) async {
    final controller = FixedExtentScrollController();
    addTearDown(controller.dispose);
    var changes = 0;
    var selected = 0;
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    h.value('selected_changes', () => changes.toDouble());
    h.value('selected_index', () => selected.toDouble());
    await h.mount((_) => ListWheelScrollView(
          controller: controller,
          itemExtent: 50,
          onSelectedItemChanged: (index) {
            changes++;
            selected = index;
          },
          children: List<Widget>.generate(
            40,
            (index) {
              final row = SizedBox(
                width: 300,
                child: ColoredBox(color: index.isEven ? blue : green),
              );
              return index == 0 ? h.probe('item0', row) : row;
            },
          ),
        ));
    await h.frames(1);
    await _drag(h, const Offset(400, 450), const Offset(400, 250), 3, framesPerStep: 1);
    await h.frames(60);
  });

  frameScenario('scroll__draggable_sheet__drag_extent', (h) async {
    final sheet = DraggableScrollableController();
    addTearDown(sheet.dispose);
    h.value('size', () => sheet.isAttached ? sheet.size : 0);
    await h.mount((_) => SizedBox.expand(
          child: DraggableScrollableSheet(
            controller: sheet,
            initialChildSize: 0.5,
            minChildSize: 0.25,
            maxChildSize: 1.0,
            builder: (context, scrollController) => ListView.builder(
              controller: scrollController,
              itemCount: 40,
              itemExtent: 50,
              itemBuilder: (context, index) {
                final row = ColoredBox(color: index.isEven ? blue : green);
                return index == 0 ? h.probe('item0', row) : row;
              },
            ),
          ),
        ));
    await h.frames(1);
    await _drag(h, const Offset(400, 500), const Offset(400, 300), 4, framesPerStep: 1);
    await h.frames(40);
  });

  frameScenario('scroll__nested_scroll_view__drag_offsets', (h) async {
    final outer = ScrollController();
    addTearDown(outer.dispose);
    final key = GlobalKey<NestedScrollViewState>();
    h.value('outer', () => outer.hasClients ? outer.offset : 0);
    h.value('inner', () {
      final state = key.currentState;
      return state != null && state.innerController.hasClients ? state.innerController.offset : 0;
    });
    await h.mount((_) => NestedScrollView(
          key: key,
          controller: outer,
          headerSliverBuilder: (context, innerBoxIsScrolled) => <Widget>[
            SliverToBoxAdapter(
              child: SizedBox(height: 200, child: h.probe('header', ColoredBox(color: orange))),
            ),
          ],
          body: ListView.builder(
            itemCount: 40,
            itemExtent: 50,
            itemBuilder: (context, index) {
              final row = ColoredBox(color: index.isEven ? blue : green);
              return index == 0 ? h.probe('item0', row) : row;
            },
          ),
        ));
    await h.frames(1);
    await _drag(h, const Offset(400, 500), const Offset(400, 300), 4, framesPerStep: 1);
    await h.frames(40);
  });

  frameScenario('scroll__single_child__keep_offset_page_storage', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final bucket = PageStorageBucket();
    h.value('attached', () => controller.hasClients ? 1 : 0);
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    await h.mount((phase) => phase == 0
        ? PageStorage(
            bucket: bucket,
            child: SingleChildScrollView(
              key: const PageStorageKey<String>('scroll'),
              controller: controller,
              child: Column(
                children: List<Widget>.generate(
                  40,
                  (index) => SizedBox(
                    width: 800,
                    height: 50,
                    child: ColoredBox(color: index.isEven ? blue : green),
                  ),
                ),
              ),
            ),
          )
        : const SizedBox());
    await h.frames(1);
    controller.jumpTo(300);
    await h.frames(2);
    h.setPhase(1);
    await h.frames(2);
    h.setPhase(0);
    await h.frames(3);
  });
}

Widget _countingList(ScrollController controller, List<int> counts, ScrollPhysics physics) {
  return NotificationListener<ScrollNotification>(
    onNotification: (notification) {
      if (notification is ScrollStartNotification) counts[0]++;
      if (notification is ScrollUpdateNotification) counts[1]++;
      if (notification is OverscrollNotification) counts[2]++;
      if (notification is ScrollEndNotification) counts[3]++;
      if (notification is UserScrollNotification) counts[4]++;
      return false;
    },
    child: ListView.builder(
      controller: controller,
      physics: physics,
      itemCount: 40,
      itemExtent: 50,
      itemBuilder: (context, index) => ColoredBox(color: index.isEven ? blue : green),
    ),
  );
}

/// Touch drag from [from] to [to] in [steps] moves, [framesPerStep] frames apart.
Future<void> _drag(FrameHarness h, Offset from, Offset to, int steps,
    {required int framesPerStep}) async {
  await h.down(from);
  for (var step = 1; step <= steps; step++) {
    await h.frames(framesPerStep);
    await h.move(Offset(
      from.dx + (to.dx - from.dx) * step / steps,
      from.dy + (to.dy - from.dy) * step / steps,
    ));
  }
  await h.frames(1);
  await h.up();
}
