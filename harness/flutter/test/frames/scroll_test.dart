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
}

