// Authored for Incular parity; not an upstream test registration.

import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

ByteData data(int value) => ByteData(1)..setUint8(0, value);

void main() {
  test(
    'channel drain respects overflow, handler turns, and replacement',
    () async {
      final buffers = ui.ChannelBuffers();
      const channel = 'incular.audit.channel';
      final log = <(String, int)>[];
      void push(int id) => buffers.push(channel, data(id), (response) {
        log.add(
          response == null
              ? ('dropped', id)
              : ('response', response.getUint8(0)),
        );
      });

      push(0);
      push(1);
      buffers.resize(channel, 3);
      push(2);
      expect(log, [('dropped', 0)]);

      final handledFirst = Completer<void>();
      void attach(String label) =>
          buffers.setListener(channel, (message, reply) {
            final id = message!.getUint8(0);
            log.add((label, id));
            scheduleMicrotask(() {
              log.add(('microtask', id));
              reply(message);
            });
            if (id == 1) {
              buffers.clearListener(channel);
              handledFirst.complete();
            }
          });

      attach('old');
      push(3);
      expect(log, [('dropped', 0)]);
      await handledFirst.future;
      await Future<void>(() {});
      expect(log, [
        ('dropped', 0),
        ('old', 1),
        ('microtask', 1),
        ('response', 1),
      ]);
      attach('new');
      await Future<void>(() {});
      expect(log, [
        ('dropped', 0),
        ('old', 1),
        ('microtask', 1),
        ('response', 1),
        ('new', 2),
        ('microtask', 2),
        ('response', 2),
        ('new', 3),
        ('microtask', 3),
        ('response', 3),
      ]);
      push(4);
      expect(log.last, ('new', 4));
      await Future<void>(() {});
      expect(log.last, ('response', 4));
    },
  );

  test(
    'channel resize drops oldest and distinguishes empty from null',
    () async {
      final buffers = ui.ChannelBuffers();
      const startup = 'incular.audit.startup';
      const other = 'incular.audit.other';
      buffers.resize(startup, 2);
      var didResize = false;
      buffers.push(startup, data(42), (response) {
        expect(response, isNull);
        didResize = true;
        buffers.push(other, null, (_) {});
      });
      buffers.push(startup, ByteData(0), (_) {});
      buffers.resize(startup, 1);
      expect(didResize, isTrue);
      final lengths = <int?>[];
      for (final channel in [startup, other]) {
        buffers.setListener(channel, (payload, reply) {
          lengths.add(payload?.lengthInBytes);
          reply(null);
        });
      }
      await Future<void>(() {});
      expect(lengths, [0, null]);
    },
  );
}
