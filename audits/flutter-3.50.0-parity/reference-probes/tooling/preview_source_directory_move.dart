// Uses the pinned flutter_tools package config and its real DirectoryWatcher.
import 'dart:async';
import 'dart:io';

import 'package:watcher/watcher.dart';

Future<void> main() async {
  final temp = await Directory.systemTemp.createTemp('preview-source-move-');
  final root = temp.path;
  final source = await Directory('$root/lib/regular').create(recursive: true);
  await Directory('$root/staged/nested').create(recursive: true);
  await File('$root/staged/nested/preview.dart')
      .writeAsString('void preview() {}');
  final watcher = DirectoryWatcher('$root/lib');
  final events = StreamIterator<WatchEvent>(watcher.events);
  final target = '${source.path}/preview';
  try {
    final added = events.moveNext().timeout(const Duration(seconds: 5));
    await watcher.ready.timeout(const Duration(seconds: 5));
    await Directory('$root/staged').rename(target);
    if (!await added ||
        events.current.type != ChangeType.ADD ||
        events.current.path != '$target/nested/preview.dart') {
      throw StateError(
        'Expected nested source addition, got ${events.current}',
      );
    }

    final removed = events.moveNext().timeout(const Duration(seconds: 5));
    await Directory(target).rename('$root/removed');
    if (!await removed ||
        events.current.type != ChangeType.REMOVE ||
        events.current.path != '$target/nested/preview.dart') {
      throw StateError('Expected nested source removal, got ${events.current}');
    }
    stdout.writeln('Directory watcher reported both nested source changes.');
  } finally {
    await events.cancel();
    await temp.delete(recursive: true);
  }
}
