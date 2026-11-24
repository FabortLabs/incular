import 'dart:convert';
import 'dart:io';
import 'package:analyzer/dart/analysis/utilities.dart';
import 'package:analyzer/dart/ast/ast.dart';
import 'package:analyzer/dart/ast/visitor.dart';

const libraryDirs = [
  'animation', 'foundation', 'gestures', 'painting', 'physics', 'rendering',
  'scheduler', 'semantics', 'services', 'widgets',
];
const previewRoots = [
  'packages/flutter_tools/test/commands.shard/hermetic/widget_preview',
  'packages/flutter_tools/test/commands.shard/permeable/widget_preview',
  'dev/integration_tests/widget_preview_scaffold/test',
];
const otherPreviewDirs = [
  'packages/flutter_tools/test/general.shard/migrations',
  'packages/flutter_tools/test/integration.shard',
];
const registrations = {
  'test', 'testWidgets', 'testGoldens', 'testGolden', 'testWithoutContext',
  'testWithContext', 'testUsingContext', 'testWidgetsWithLeakTracking',
  'testExecutable', 'testGesture', 'frameScenario',
};
const groups = {'group', 'groupWithContext', 'groupWithoutContext'};
typedef Row = Map<String,Object?>;

class ScenarioVisitor extends RecursiveAstVisitor<void> {
  ScenarioVisitor(this.source, this.library, this.lineFor);
  final String source;
  final String library;
  final int Function(int) lineFor;
  final rows = <Row>[];
  final groupLabels = <String>[];
  int dynamicNames = 0;

  @override
  void visitMethodInvocation(MethodInvocation node) {
    final method = node.methodName.name;
    final args = node.argumentList.arguments;
    if (args.isEmpty) {
      super.visitMethodInvocation(node);
      return;
    }
    final first = args.first;
    final label = first is StringLiteral ? first.stringValue : null;
    final value = label ?? first.toSource();
    if (groups.contains(method)) {
      groupLabels.add(value);
      super.visitMethodInvocation(node);
      groupLabels.removeLast();
      return;
    }
    if (registrations.contains(method)) {
      final options = <String, String>{
        for (final a in args.whereType<NamedExpression>())
          a.name.label.name: a.expression.toSource(),
      };
      if (label == null) dynamicNames++;
      rows.add({
        'library':library, 'file':source, 'line':lineFor(node.methodName.offset),
        'registration':method, 'description':value,
        if (groupLabels.isNotEmpty) 'groups':[...groupLabels],
        if (label == null) 'dynamicDescription':true,
        if (options.isNotEmpty) 'options':options,
        'verification':'not-compared-at-3.50.0',
      });
    }
    super.visitMethodInvocation(node);
  }
}

void main(List<String> args) {
  final flutter = Directory(args[0]).resolveSymbolicLinksSync();
  final checkout = Directory(args[1]).resolveSymbolicLinksSync();
  final out = Directory(args[2]);
  final seen = <String>{};
  final diagnostics = <String>[];
  final framework = <Row>[];
  final harness = <Row>[];
  final filesByLibrary = <String, int>{};
  final rowsByLibrary = <String, int>{};
  var dynamicNames = 0;
  void visitRoot(String base, String relative, String library, List<Row> rows,
    {bool Function(File)? include, Set<String>? extensions}) {
    final dir = Directory(base+'/'+relative);
    if (!dir.existsSync()) throw StateError('Missing test root '+relative);
    final sourceFiles = dir.listSync(recursive:true).whereType<File>().where(
      (f) => (extensions ?? {'_test.dart'}).any((end)=>f.path.endsWith(end)) &&
        (include == null || include(f)),
    ).toList()..sort((a,b)=>a.path.compareTo(b.path));
    for (final file in sourceFiles) {
      final path = file.absolute.path.replaceAll('\\','/').substring(base.replaceAll('\\','/').length+1);
      if (!seen.add(path)) continue;
      filesByLibrary.update(library,(n)=>n+1,ifAbsent:()=>1);
      final result = parseString(content:file.readAsStringSync(),path:path,throwIfDiagnostics:false);
      for (final err in result.errors) { diagnostics.add(path+':'+err.offset.toString()+': '+err.message); }
      final v = ScenarioVisitor(path,library,(offset)=>result.lineInfo.getLocation(offset).lineNumber);
      result.unit.accept(v);
      rows.addAll(v.rows);
      rowsByLibrary.update(library,(n)=>n+v.rows.length,ifAbsent:()=>v.rows.length);
      dynamicNames += v.dynamicNames;
    }
  }
  for (final l in libraryDirs) {
    visitRoot(flutter,'packages/flutter/test/'+l,l,framework);
  }
  for (final dir in previewRoots) { visitRoot(flutter,dir,'widget_previews',framework); }
  for (final dir in otherPreviewDirs) {
    visitRoot(flutter,dir,'widget_previews',framework,include:(f)=>f.uri.pathSegments.last.contains('widget_preview'));
  }
  visitRoot(checkout,'harness/flutter/test','incular_harness',harness);
  void save(String path, List<Row> rows) {
    File(out.path+'/'+path).writeAsStringSync(rows.map(jsonEncode).join('\n')+'\n');
  }
  save('upstream-scenarios.jsonl',framework);
  save('existing-harness-scenarios.jsonl',harness);
  File(out.path+'/scenario-notes.json').writeAsStringSync(const JsonEncoder.withIndent('  ').convert({
    'filesByLibrary':filesByLibrary,'rowsByLibrary':rowsByLibrary,'dynamicDescriptions':dynamicNames,
    'diagnostics':diagnostics,'registrations':registrations.toList()..sort(),
    'notes':[
      'Rows represent captured registration sites; loops/variants can generate more run-time cases.',
      'Custom helpers, skipped cases, static conditional branches and engine or package-level tests need separate review.',
      'Material/Cupertino test directories excluded; a neutral scenario may use a design-system fixture.',
      'Incular harness scenarios were inventoried, not re-run against this Flutter snapshot.',
    ],
  })+'\n');
  print('Flutter registration sites: '+framework.length.toString()+'; existing harness: '+harness.length.toString());
}
