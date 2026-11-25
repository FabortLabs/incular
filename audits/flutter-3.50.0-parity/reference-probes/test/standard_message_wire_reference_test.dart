import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('nested null and UTF-8 use the pinned bytes and honor data views', () {
    const codec = StandardMessageCodec();
    const value = [null, true, '☺'];
    final bytes = codec.encodeMessage(value)!;
    final data = [12, 3, 0, 1, 7, 3, 0xe2, 0x98, 0xba];
    expect(
      bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes),
      data,
    );
    final padded = Uint8List.fromList([0xee, ...data, 0xee]);
    expect(codec.decodeMessage(ByteData.sublistView(padded, 1, 10)), value);
  });
}
