// Flutter reference captures for the Incular pixel parity harness.
//
// Renders every scene in a 160x160 logical RepaintBoundary and writes
// `<scene>@<dpr>.png` (Skia GPU raster on Windows) to PARITY_OUT, then exits.
// The scenes must stay identical to the Incular side in
// harness/incular/tests/pixels.rs.

import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

const double side = 160;
const List<double> ratios = [1.0, 1.25, 1.5, 2.0];
late final ui.Image fitFixture;
late final Uint8List fitFixturePng;

Widget at(double left, double top, double width, double height, Widget child) =>
    Positioned(left: left, top: top, width: width, height: height, child: child);

Widget canvas(Color background, List<Widget> children) => Container(
      width: side,
      height: side,
      color: background,
      child: Stack(clipBehavior: Clip.none, children: children),
    );

const TextStyle inter = TextStyle(fontFamily: 'Inter', color: Colors.white);

class ShapePainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final fill = Paint()..color = Colors.white;
    canvas.drawPath(
      Path()
        ..moveTo(20, 20)
        ..lineTo(140, 30)
        ..lineTo(40, 140)
        ..close(),
      fill,
    );
    canvas.drawPath(
      Path()
        ..moveTo(10, 150)
        ..lineTo(150, 100),
      Paint()
        ..color = Colors.white
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2,
    );
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class TriangleClipper extends CustomClipper<Path> {
  @override
  Path getClip(Size size) => Path()
    ..moveTo(0, 0)
    ..lineTo(size.width, 0)
    ..lineTo(size.width / 2, size.height)
    ..close();

  @override
  bool shouldReclip(TriangleClipper oldClipper) => false;
}

final Map<String, Widget Function()> scenes = {
  'image_default_fit': () => canvas(Colors.black, [
        at(30, 30, 100, 100, RawImage(image: fitFixture, width: 100, height: 100)),
      ]),
  'decoration_image_default_fit': () => canvas(Colors.black, [
        at(30, 30, 100, 100, Container(
          decoration: BoxDecoration(image: DecorationImage(image: MemoryImage(fitFixturePng))),
        )),
      ]),
  // A box-relative gradient on a box away from the origin (paint audit:
  // positionDependentBox, gradient geometry).
  'linear_gradient_offset_box': () => canvas(Colors.black, [
        at(30, 30, 100, 100, Container(
          decoration: const BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [Color(0xFFFF0000), Color(0xFF0000FF)],
            ),
          ),
        )),
      ]),
  // A static RawScrollbar thumb at rest with thumbVisibility (scroll audit:
  // thumb geometry; fade timing is not observable in a still capture).
  'raw_scrollbar_thumb': () {
    final controller = ScrollController();
    return canvas(Colors.black, [
      Positioned.fill(
        child: RawScrollbar(
          controller: controller,
          thumbVisibility: true,
          thumbColor: Colors.white,
          thickness: 6,
          // Without this the desktop ScrollBehavior adds a second (hidden at
          // rest) RawScrollbar.
          child: ScrollConfiguration(
            behavior: const ScrollBehavior().copyWith(scrollbars: false),
            child: ListView.builder(
            controller: controller,
            itemCount: 20,
            itemExtent: 40,
            itemBuilder: (context, index) => ColoredBox(
              color: index.isEven ? const Color(0xFF303030) : const Color(0xFF505050),
            ),
          ),
          ),
        ),
      ),
    ]);
  },
  // Line placement per textAlign (text audit: no line-box accessor exists on
  // either side). The last row wraps so justify has a non-final line.
  'text_align': () => canvas(Colors.black, [
        at(8, 8, 144, 20, Text('ab', textAlign: TextAlign.start, style: inter.copyWith(fontSize: 14))),
        at(8, 32, 144, 20, Text('ab', textAlign: TextAlign.center, style: inter.copyWith(fontSize: 14))),
        at(8, 56, 144, 20, Text('ab', textAlign: TextAlign.end, style: inter.copyWith(fontSize: 14))),
        at(8, 80, 144, 72, Text('aa bb cc dd ee ff gg hh ii jj kk', textAlign: TextAlign.justify, style: inter.copyWith(fontSize: 14))),
      ]),
  'circle': () => canvas(Colors.black, [
        at(20, 20, 120, 120,
            Container(decoration: const BoxDecoration(color: Colors.white, shape: BoxShape.circle))),
      ]),
  'rrect': () => canvas(Colors.black, [
        at(20, 40, 120, 80,
            Container(
                decoration: BoxDecoration(
                    color: Colors.white, borderRadius: BorderRadius.circular(24)))),
      ]),
  'path': () => canvas(Colors.black, [
        Positioned.fill(child: CustomPaint(painter: ShapePainter())),
      ]),
  'text': () => canvas(Colors.black, [
        at(8, 8, 152, 20, const Text('Hamburgefonstiv 0123', softWrap: false, style: inter)),
        at(8, 40, 152, 30,
            Text('Quick brown fox', softWrap: false, style: inter.copyWith(fontSize: 16))),
        at(8, 80, 152, 40,
            Text('Ag 0123', softWrap: false, style: inter.copyWith(fontSize: 24))),
        at(8, 124, 152, 30,
            Text('Wavy 12.5', softWrap: false, style: inter.copyWith(fontSize: 12.5))),
      ]),
  'rich_text_spans': () => canvas(Colors.black, [
        at(8, 30, 144, 40, RichText(
          softWrap: false,
          text: TextSpan(style: inter.copyWith(fontSize: 20), children: const [
            TextSpan(text: 'RED ', style: TextStyle(color: Color(0xFFFF0000))),
            TextSpan(text: 'BLUE', style: TextStyle(color: Color(0xFF0000FF))),
          ]),
        )),
      ]),
  'size_transition_clip': () => canvas(Colors.black, [
        Positioned(
          left: 20,
          top: 20,
          child: SizeTransition(
            sizeFactor: const AlwaysStoppedAnimation<double>(0.5),
            child: const SizedBox(width: 120, height: 100, child: ColoredBox(color: Colors.white)),
          ),
        ),
      ]),
  'physical_shape_triangle': () => canvas(Colors.black, [
        at(20, 20, 120, 120, PhysicalShape(
          clipper: TriangleClipper(), elevation: 8, color: Colors.white,
          child: const Align(alignment: Alignment.topLeft,
            child: SizedBox(width: 40, height: 40, child: ColoredBox(color: Colors.red))),
        )),
      ]),
  'physical_model_circle': () => canvas(Colors.black, [
        at(20, 20, 120, 120, const PhysicalModel(
          shape: BoxShape.circle,
          color: Colors.blue,
          child: Align(alignment: Alignment.center,
            child: SizedBox(width: 60, height: 60, child: ColoredBox(color: Colors.red))),
        )),
      ]),
  'overlay_portal_ancestor_clip': () => canvas(Colors.black, [
        Positioned.fill(child: Overlay(initialEntries: [
          OverlayEntry(builder: (_) => Align(
            alignment: Alignment.topLeft,
            child: ClipRect(child: SizedBox(width: 80, height: 80,
              child: OverlayPortal(
                controller: OverlayPortalController()..show(),
                overlayChildBuilder: (_) => Stack(children: [
                  at(110, 110, 40, 40, const ColoredBox(color: Colors.orange)),
                ]),
                child: const ColoredBox(color: Colors.blue),
              ),
            )),
          )),
        ])),
      ]),
  'overlay_portal_no_clip': () => canvas(Colors.black, [
        Positioned.fill(child: Overlay(initialEntries: [
          OverlayEntry(builder: (_) => Align(
            alignment: Alignment.topLeft,
            child: SizedBox(width: 80, height: 80,
              child: OverlayPortal(
                controller: OverlayPortalController()..show(),
                overlayChildBuilder: (_) => Stack(children: [
                  at(110, 110, 40, 40, const ColoredBox(color: Colors.orange)),
                ]),
                child: const ColoredBox(color: Colors.blue),
              ),
            ),
          )),
        ])),
      ]),
  'clip_r_superellipse': () => canvas(Colors.black, [
        at(20, 40, 120, 80, ClipRSuperellipse(
          borderRadius: BorderRadius.circular(32),
          child: const SizedBox.expand(child: ColoredBox(color: Colors.white)),
        )),
      ]),
  'color_filter_matrix_bias': () => canvas(Colors.black, [
        at(40, 40, 80, 80, const ColorFiltered(
          colorFilter: ui.ColorFilter.matrix(<double>[
            -1, 0, 0, 0, 255, 0, -1, 0, 0, 255,
            0, 0, -1, 0, 255, 0, 0, 0, 1, 0,
          ]),
          child: ColoredBox(color: Colors.white),
        )),
      ]),
  'text_dark': () => canvas(Colors.white, [
        at(8, 8, 152, 20, Text('Hamburgefonstiv 0123', softWrap: false, style: inter.copyWith(color: Colors.black))),
        at(8, 40, 152, 30,
            Text('Quick brown fox', softWrap: false, style: inter.copyWith(fontSize: 16, color: Colors.black))),
        at(8, 80, 152, 40,
            Text('Ag 0123', softWrap: false, style: inter.copyWith(fontSize: 24, color: Colors.black))),
      ]),
  'text_gray': () => canvas(Colors.black, [
        at(8, 8, 152, 20, Text('Hamburgefonstiv 0123', softWrap: false, style: inter.copyWith(color: const Color(0xFF808080)))),
        at(8, 40, 152, 30,
            Text('Quick brown fox', softWrap: false, style: inter.copyWith(fontSize: 16, color: const Color(0xFF808080)))),
        at(8, 80, 152, 40,
            Text('Ag 0123', softWrap: false, style: inter.copyWith(fontSize: 24, color: const Color(0xFF808080)))),
      ]),
  'shadow': () => canvas(Colors.white, [
        at(40, 40, 80, 80,
            Container(
                decoration: BoxDecoration(
              color: const Color(0xFF2196F3),
              borderRadius: BorderRadius.circular(8),
              boxShadow: const [
                BoxShadow(color: Color(0x8A000000), blurRadius: 16, offset: Offset(0, 4)),
              ],
            ))),
      ]),
  'blur': () => ImageFiltered(
        imageFilter: ui.ImageFilter.blur(sigmaX: 4, sigmaY: 4),
        child: canvas(Colors.black, [
          at(50, 50, 60, 60, Container(color: Colors.white)),
        ]),
      ),
  'blend': () => canvas(Colors.black, [
        at(20, 20, 80, 80, Container(color: const Color(0xFFFF0000))),
        at(60, 60, 80, 80, Opacity(opacity: 0.5, child: Container(color: const Color(0xFF0000FF)))),
        at(40, 90, 100, 40, Container(color: const Color(0x80FFFFFF))),
        at(100, 10, 50, 50, Container(color: const Color(0x4000FF00))),
      ]),
  'hairline': () => canvas(Colors.black, [
        at(10.3, 10.7, 50.5, 30.25,
            Container(decoration: BoxDecoration(border: Border.all(color: Colors.white, width: 1)))),
        at(80.5, 20.25, 40, 40, Container(color: Colors.white)),
        at(10.25, 70.5, 140, 0.5, Container(color: Colors.white)),
        at(10, 90.4, 140, 1, Container(color: Colors.white)),
        at(20.6, 110, 0.75, 40, Container(color: Colors.white)),
      ]),
  // A backdrop blur clipped to a rounded rectangle: the corners outside
  // the rounded shape keep the sharp stripes.
  'backdrop_rrect': () => canvas(Colors.black, [
        for (var x = 10.0; x < 160; x += 40) at(x, 0, 20, 160, Container(color: Colors.white)),
        at(20, 20, 120, 120,
            ClipRRect(
              borderRadius: BorderRadius.circular(40),
              child: BackdropFilter(
                filter: ui.ImageFilter.blur(sigmaX: 6, sigmaY: 6),
                child: const SizedBox.expand(),
              ),
            )),
      ]),
};

final GlobalKey boundary = GlobalKey();

class Host extends StatefulWidget {
  const Host({super.key});
  @override
  State<Host> createState() => HostState();
}

class HostState extends State<Host> {
  String scene = scenes.keys.first;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => run());
  }

  Future<void> run() async {
    final out = Directory(Platform.environment['PARITY_OUT'] ?? '../out/pixels/flutter');
    out.createSync(recursive: true);
    await precacheImage(MemoryImage(fitFixturePng), context);
    for (final name in scenes.keys) {
      setState(() => scene = name);
      await WidgetsBinding.instance.endOfFrame;
      await WidgetsBinding.instance.endOfFrame;
      final render = boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
      for (final ratio in ratios) {
        final image = await render.toImage(pixelRatio: ratio);
        final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
        File('${out.path}/$name@$ratio.png').writeAsBytesSync(bytes!.buffer.asUint8List());
        image.dispose();
      }
    }
    stdout.writeln('parity: wrote ${scenes.length * ratios.length} captures to ${out.path}');
    exit(0);
  }

  @override
  Widget build(BuildContext context) => Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: RepaintBoundary(
            key: boundary,
            child: SizedBox(width: side, height: side, child: scenes[scene]!()),
          ),
        ),
      );
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  canvas.drawRect(const Rect.fromLTWH(0, 0, 40, 20), ui.Paint()..color = Colors.white);
  fitFixture = await recorder.endRecording().toImage(40, 20);
  fitFixturePng = (await fitFixture.toByteData(format: ui.ImageByteFormat.png))!.buffer.asUint8List();
  runApp(const Host());
}
