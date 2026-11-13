import 'dart:convert';
import 'dart:io';

import 'package:analyzer/dart/analysis/utilities.dart';
import 'package:analyzer/dart/ast/ast.dart';

// Produce a lossless, reviewable candidate list. The four supplementary
// libraries require human scoping: not all their declarations affect widgets.
void main() {
  final root = Directory('C:/Manual Installs/flutter/packages/flutter/lib/src');
  const dirs = ['animation', 'physics', 'scheduler', 'widgets', 'rendering', 'painting', 'gestures', 'services'];
  final rows = <Map<String, Object?>>[];
  final errors = <String>[];
  for (final dir in dirs) {
    final files = Directory('${root.path}/$dir').listSync(recursive: true)
        .whereType<File>().where((f) => f.path.endsWith('.dart')).toList()
      ..sort((a, b) => a.path.compareTo(b.path));
    for (final file in files) {
      final relative = file.path.substring(root.path.length + 1).replaceAll('\\', '/');
      final parsed = parseString(content: file.readAsStringSync(), path: file.path, throwIfDiagnostics: false);
      if (parsed.errors.isNotEmpty) errors.add('$relative: ${parsed.errors.length} parse diagnostic(s)');
      void add(String name, String kind, int offset) {
        if (name.startsWith('_')) return;
        rows.add({'file': relative, 'line': parsed.lineInfo.getLocation(offset).lineNumber,
          'name': name, 'kind': kind,
          'scope': dirs.indexOf(dir) < 4 ? 'primary' : 'widget dependency: review relevance'});
      }
      for (final d in parsed.unit.declarations) {
        if (d is ClassDeclaration || d is MixinDeclaration || d is EnumDeclaration || d is ExtensionTypeDeclaration) {
          final named = d as NamedCompilationUnitMember;
          add(named.name.lexeme, d.runtimeType.toString().replaceAll('Impl', ''), named.name.offset);
          if (d is ClassDeclaration && named.name.lexeme == 'Curves') {
            for (final m in d.members.whereType<FieldDeclaration>()) {
              if (m.isStatic && m.fields.isConst) {
                for (final v in m.fields.variables) { add('Curves.${v.name.lexeme}', 'static constant', v.name.offset); }
              }
            }
          }
        } else if (d is FunctionDeclaration) {
          add(d.name.lexeme, d.isGetter ? 'getter' : d.isSetter ? 'setter' : 'function', d.name.offset);
        } else if (d is TopLevelVariableDeclaration) {
          for (final v in d.variables.variables) {
            add(v.name.lexeme, d.variables.isConst ? 'constant' : 'top-level variable', v.name.offset);
          }
        }
      }
    }
  }
  File('../declarations.jsonl').writeAsStringSync(rows.map(jsonEncode).join('\n') + '\n');
  File('../declaration-parse-diagnostics.txt').writeAsStringSync(errors.join('\n'));
  stdout.writeln('${rows.length} declarations; ${errors.length} files with parse diagnostics');
}
