import 'dart:convert';
import 'dart:io';
import 'package:analyzer/dart/analysis/utilities.dart';
import 'package:analyzer/dart/ast/ast.dart';

const libraries = ['animation','foundation','gestures','painting','physics','rendering','scheduler','semantics','services','widget_previews','widgets'];
typedef Row = Map<String,Object?>;
final units = <String,CompilationUnit>{};
final locals = <String,List<Row>>{};
final diagnostics = <String>[];
final external = <String>{};
final variants = <String>{};
late Directory root;

String relative(File f) => f.absolute.path.replaceAll('\\','/').substring(root.absolute.path.replaceAll('\\','/').length+1);
String target(String path, String uri) => uri.startsWith('package:flutter/')
  ? uri.substring('package:flutter/'.length)
  : relative(File.fromUri(File(root.path+'/'+path).absolute.uri.resolve(uri)));
CompilationUnit unit(String path) => units.putIfAbsent(path, () {
  final file=File(root.path+'/'+path);
  final result=parseString(content:file.readAsStringSync(),path:file.path,throwIfDiagnostics:false);
  for(final err in result.errors) { diagnostics.add(path+':'+err.offset.toString()+': '+err.message); }
  return result.unit;
});

List<Row> local(String path) => locals.putIfAbsent(path, () {
  final u=unit(path), rows=<Row>[];
  String withoutBody(AstNode node, AstNode body) {
    final source=node.toSource(), tail=body.toSource();
    return tail.isNotEmpty && source.endsWith(tail) ? source.substring(0,source.length-tail.length) : source;
  }
  String header(AstNode node) {
    final src=node.toSource(), pos=src.indexOf('{');
    return pos<0 ? src : src.substring(0,pos);
  }
  void add(String name,String kind,int offset,String sig,[String? owner]) {
    if(name.startsWith('_')) return;
    rows.add({'file':path,'line':u.lineInfo.getLocation(offset).lineNumber,'name':name,'kind':kind,
      if(owner!=null)'owner':owner,'signature':sig.replaceAll(RegExp(r'\s+'),' ').trim()});
  }
  void members(String owner,Iterable<ClassMember> items) {
    for(final m in items) {
      if(m is MethodDeclaration) {
        add(m.name.lexeme,m.isGetter?'getter':m.isSetter?'setter':m.isOperator?'operator':'method',m.name.offset,withoutBody(m,m.body),owner);
      } else if(m is ConstructorDeclaration) {
        add(m.name?.lexeme??owner,'constructor',m.name?.offset??m.returnType.offset,withoutBody(m,m.body),owner);
      } else if(m is FieldDeclaration) {
        for(final v in m.fields.variables) { add(v.name.lexeme,m.fields.isConst?'constant':'field',v.name.offset,m.toSource(),owner); }
      }
    }
  }
  for(final d in u.declarations) {
    if(d is ExtensionDeclaration) {
      final n=d.name;
      if(n==null) { variants.add(path+': unnamed extension excluded');continue; }
      if(n.lexeme.startsWith('_')) continue;
      add(n.lexeme,'ExtensionDeclaration',n.offset,header(d));
      members(n.lexeme,d.members);
    } else if(d is NamedCompilationUnitMember) {
      final n=d.name;
      if(n.lexeme.startsWith('_')) continue;
      final k=d.runtimeType.toString().replaceAll('Impl','');
      final sig=d is FunctionDeclaration ? withoutBody(d,d.functionExpression.body)
        : d is ClassDeclaration || d is EnumDeclaration || d is MixinDeclaration || d is ExtensionTypeDeclaration ? header(d):d.toSource();
      add(n.lexeme,k,n.offset,sig);
      if(d is ClassDeclaration) members(n.lexeme,d.members);
      if(d is MixinDeclaration) members(n.lexeme,d.members);
      if(d is ExtensionTypeDeclaration) members(n.lexeme,d.members);
      if(d is EnumDeclaration) {
        members(n.lexeme,d.members);
        for(final c in d.constants) { add(c.name.lexeme,'enum constant',c.name.offset,c.toSource(),n.lexeme); }
      }
    } else if(d is TopLevelVariableDeclaration) {
      for(final v in d.variables.variables) { add(v.name.lexeme,d.variables.isConst?'constant':'variable',v.name.offset,d.toSource()); }
    }
  }
  return rows;
});

List<Row> exports(String path,String library,Set<String> stack) {
  if(!stack.add(path)) return [];
  final rows=[...local(path)];
  for(final d in unit(path).directives) {
    if(d is PartDirective && d.uri.stringValue!=null) rows.addAll(local(target(path,d.uri.stringValue!)));
    if(d is! ExportDirective) continue;
    final uris=[d.uri.stringValue,...d.configurations.map((x)=>x.uri.stringValue)];
    if(uris.length>1){variants.add(library+': '+path+' conditional export union: '+d.toSource());}
    for(final uri in uris.whereType<String>()) {
      if(uri.startsWith('dart:') || (uri.startsWith('package:') && !uri.startsWith('package:flutter/'))) {
        external.add(library+': '+path+' -> '+d.toSource());
        continue;
      }
      final file=target(path,uri);
      if(RegExp(r'^(src/(material|cupertino)/|(material|cupertino)\.dart)').hasMatch(file)) continue;
      var edge=exports(file,library,stack);
      for(final c in d.combinators) {
        if(c is ShowCombinator) {
          final names=c.shownNames.map((i)=>i.name).toSet();
          edge=edge.where((r)=>names.contains(r['owner']??r['name'])).toList();
        }
        if(c is HideCombinator) {
          final names=c.hiddenNames.map((i)=>i.name).toSet();
          edge=edge.where((r)=>!names.contains(r['owner']??r['name'])).toList();
        }
      }
      rows.addAll(edge);
    }
  }
  stack.remove(path);
  return rows;
}

void main(List<String> args) {
  root=Directory(Directory(args[0]).resolveSymbolicLinksSync());
  final all=<String,Row>{};
  for(final l in libraries) {
    for(final row in exports(l+'.dart',l,{})) {
      final key=[row['file'],row['line'],row['owner'],row['kind'],row['name']].join('|');
      final by=all.putIfAbsent(key,()=>{...row,'exportedBy':<String>[]})['exportedBy'] as List<String>;
      if(!by.contains(l))by.add(l);
    }
  }
  final rows=all.values.toList()..sort((a,b){
    final f=(a['file'] as String).compareTo(b['file'] as String);
    return f==0?(a['line'] as int).compareTo(b['line'] as int):f;
  });
  final out=args[1];
  File(out+'/flutter-public-api.jsonl').writeAsStringSync(rows.map(jsonEncode).join('\n')+'\n');
  File(out+'/inventory-notes.json').writeAsStringSync(const JsonEncoder.withIndent('  ').convert({
    'entrypoints':libraries,'parsedFiles':units.length,'rows':rows.length,'diagnostics':diagnostics,
    'externalReexports':external.toList()..sort(),'variants':variants.toList()..sort(),
  })+'\n');
  print('rows='+rows.length.toString()+', parsed files='+units.length.toString()+', diagnostics='+diagnostics.length.toString());
}
