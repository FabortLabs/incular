import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('removing a retained ancestor unregisters the nested leader', () {
    final link = LayerLink();
    final leader = LeaderLayer(link: link);
    final keepLeader = LayerHandle<LeaderLayer>(leader);
    final branch = ContainerLayer()..append(leader);
    final keepBranch = LayerHandle<ContainerLayer>(branch);
    final root = ContainerLayer()..append(branch);
    final owner = Object();

    root.attach(owner);
    expect(link.leader, same(leader));
    expect(leader.attached, isTrue);

    branch.remove();
    expect(leader.attached, isFalse);
    expect(link.leader, isNull);
    expect(keepLeader.layer, same(leader));
    expect(branch.firstChild, same(leader));

    root.append(branch);
    expect(link.leader, same(leader));
    root.detach();
    expect(link.leader, isNull);
    expect(leader.attached, isFalse);

    root.attach(owner);
    expect(link.leader, same(leader));
    root.detach();
    root.removeAllChildren();
    keepBranch.layer = null;
    keepLeader.layer = null;
  });
}
