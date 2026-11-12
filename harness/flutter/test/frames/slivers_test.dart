import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);
const Color green = Color(0xFF4CAF50);

Widget rowBox(FrameHarness h, String probe, double height, Color color) => SizedBox(
      height: height,
      child: h.probe(probe, ColoredBox(color: color)),
    );

class FixedHeaderDelegate extends SliverPersistentHeaderDelegate {
  FixedHeaderDelegate(this.h, this.name, this.color, this.extent);
  final FrameHarness h;
  final String name;
  final Color color;
  final double extent;
  @override
  double get minExtent => extent;
  @override
  double get maxExtent => extent;
  @override
  Widget build(BuildContext context, double shrinkOffset, bool overlapsContent) =>
      SizedBox.expand(child: h.probe(name, ColoredBox(color: color)));
  @override
  bool shouldRebuild(covariant FixedHeaderDelegate oldDelegate) => false;
}

void main() {
  frameScenario('slivers__box_padding_fill_remaining__static_and_scroll', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    h.value('offset', () => controller.hasClients ? controller.offset : 0.0);
    await h.mount((_) => CustomScrollView(
          controller: controller,
          slivers: <Widget>[
            SliverToBoxAdapter(child: rowBox(h, 'adapter', 45, blue)),
            SliverPadding(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
              sliver: SliverList(
                delegate: SliverChildListDelegate(<Widget>[
                  rowBox(h, 'padded0', 36, orange),
                  rowBox(h, 'padded1', 52, green),
                  rowBox(h, 'padded2', 40, blue),
                ]),
              ),
            ),
            SliverFillRemaining(
              hasScrollBody: false,
              child: h.probe('fill_remaining', const ColoredBox(color: green)),
            ),
          ],
        ));
    await h.frames(2);
    await h.down(const Offset(400, 500));
    await h.frames(1);
    await h.move(const Offset(400, 430));
    await h.frames(2);
    await h.up();
    await h.frames(2);
  });

  frameScenario('slivers__varied_extent_list__indices', (h) async {
    await h.mount((_) => CustomScrollView(slivers: <Widget>[
          SliverVariedExtentList(
            itemExtentBuilder: (index, _) => 30 + index * 7.0,
            delegate: SliverChildBuilderDelegate(
              (context, index) => h.probe('varied$index', ColoredBox(color: index.isEven ? green : blue)),
              childCount: 4,
            ),
          ),
        ]));
    await h.frames(2);
  });

  frameScenario('slivers__fixed_prototype_extent_lists__indices', (h) async {
    await h.mount((_) => CustomScrollView(slivers: <Widget>[
          SliverFixedExtentList(
            itemExtent: 38,
            delegate: SliverChildBuilderDelegate(
              (context, index) => h.probe('fixed$index', ColoredBox(color: index.isEven ? blue : orange)),
              childCount: 4,
            ),
          ),
          SliverToBoxAdapter(child: rowBox(h, 'between', 24, green)),
          SliverPrototypeExtentList(
            prototypeItem: const SizedBox(height: 44),
            delegate: SliverChildBuilderDelegate(
              (context, index) => h.probe('prototype$index', ColoredBox(color: index.isEven ? orange : green)),
              childCount: 4,
            ),
          ),
        ]));
    await h.frames(4);
  });

  frameScenario('slivers__grid_and_fill_viewport__geometry', (h) async {
    await h.mount((_) => CustomScrollView(slivers: <Widget>[
          SliverGrid(
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 3,
              mainAxisExtent: 54,
              mainAxisSpacing: 6,
              crossAxisSpacing: 8,
            ),
            delegate: SliverChildBuilderDelegate(
              (context, index) => h.probe('grid$index', ColoredBox(color: index.isEven ? blue : orange)),
              childCount: 6,
            ),
          ),
          SliverFillViewport(
            viewportFraction: 0.75,
            padEnds: false,
            delegate: SliverChildListDelegate(<Widget>[
              h.probe('page0', const ColoredBox(color: green)),
              h.probe('page1', const ColoredBox(color: blue)),
            ]),
          ),
        ]));
    await h.frames(3);
  });

  frameScenario('slivers__pinned_header_and_main_axis_group__scroll', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    h.value('offset', () => controller.hasClients ? controller.offset : 0.0);
    await h.mount((_) => CustomScrollView(
          controller: controller,
          slivers: <Widget>[
            SliverMainAxisGroup(slivers: <Widget>[
              SliverPersistentHeader(
                pinned: true,
                delegate: FixedHeaderDelegate(h, 'pinned', blue, 70),
              ),
              SliverFixedExtentList(
                itemExtent: 46,
                delegate: SliverChildBuilderDelegate(
                  (context, index) => rowBox(h, 'row$index', 46, index.isEven ? orange : green),
                  childCount: 20,
                ),
              ),
            ]),
            SliverToBoxAdapter(child: rowBox(h, 'after_group', 50, blue)),
          ],
        ));
    await h.frames(1);
    controller.jumpTo(135);
    await h.frames(4);
  });

  frameScenario('slivers__cross_axis_group__lanes', (h) async {
    await h.mount((_) => CustomScrollView(slivers: <Widget>[
          SliverCrossAxisGroup(slivers: <Widget>[
            SliverToBoxAdapter(child: SizedBox(height: 100, child: h.probe('lane_left', const ColoredBox(color: blue)))),
            SliverToBoxAdapter(child: SizedBox(height: 100, child: h.probe('lane_right', const ColoredBox(color: orange)))),
          ]),
          SliverToBoxAdapter(child: rowBox(h, 'after_lanes', 50, green)),
        ]));
    await h.frames(2);
  });

  frameScenario('slivers__pinned_header_sliver__child_extent', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    h.value('offset', () => controller.hasClients ? controller.offset : 0.0);
    await h.mount((_) => CustomScrollView(
          controller: controller,
          slivers: <Widget>[
            PinnedHeaderSliver(
              child: SizedBox(height: 60, child: h.probe('pinned_child', const ColoredBox(color: blue))),
            ),
            SliverFixedExtentList(
              itemExtent: 42,
              delegate: SliverChildBuilderDelegate(
                (context, index) => rowBox(h, 'body$index', 42, index.isEven ? green : orange),
                childCount: 20,
              ),
            ),
          ],
        ));
    controller.jumpTo(80);
    await h.frames(3);
  });

  frameScenario('slivers__custom_scroll_view__cache_extent', (h) async {
    await h.mount((_) => CustomScrollView(
          cacheExtent: 180,
          slivers: <Widget>[
            SliverFixedExtentList(
              itemExtent: 40,
              delegate: SliverChildBuilderDelegate(
                (context, index) => h.probe('cache$index', ColoredBox(color: index.isEven ? blue : orange)),
                childCount: 30,
              ),
            ),
          ],
        ));
    await h.frames(2);
  });
}

