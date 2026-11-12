// Flutter reference captures for the Incular pixel parity harness.
//
// Renders every scene in a 160x160 logical RepaintBoundary and writes
// `<scene>@<dpr>.png` (Skia GPU raster on Windows) to PARITY_OUT, then exits.
// The scenes must stay identical to the Incular side in
// harness/incular/tests/pixels.rs.

import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

const double side = 160;
const List<double> ratios = [1.0, 1.25, 1.5, 2.0];

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

final Map<String, Widget Function()> scenes = {
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

void main() => runApp(const Host());
