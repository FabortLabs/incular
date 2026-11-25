import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('integer scalars are packed and float64 aligns to eight bytes', () {
    final writer = WriteBuffer(startCapacity: 1);
    writer.putUint8(7);
    writer.putUint16(0x0102, endian: Endian.big);
    writer.putUint32(0x03040506, endian: Endian.little);
    writer.putFloat64(0.5, endian: Endian.big);
    final bytes = writer.done();
    expect(bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes), [
      7,
      1,
      2,
      6,
      5,
      4,
      3,
      0,
      0x3f,
      0xe0,
      0,
      0,
      0,
      0,
      0,
      0,
    ]);
    final reader = ReadBuffer(bytes);
    expect(reader.getUint8(), 7);
    expect(reader.getUint16(endian: Endian.big), 0x0102);
    expect(reader.getUint32(endian: Endian.little), 0x03040506);
    expect(reader.getFloat64(endian: Endian.big), 0.5);
    expect(reader.hasRemaining, isFalse);
  });
}
