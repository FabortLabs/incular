# View touch slop shared by scroll and drag

Both exact Flutter 3.50.0 registrations in
`gestures/gesture_config_regression_test.dart` passed on Linux. The
pinned test view's DPR is 3, so 4 physical pixels is 4/3 logical
pixels. The mounted Incular fixture sends the actual touch down, then
moves 0, 5/3, 10/3, and 5 logical pixels over 50 ms, and releases.
The contact starts at the center of the keyed second row of a
50-pixel fixed-extent sliver list inside CustomScrollView.

The competing scroll viewport does not steal the inner detector's
update or the Draggable's start, update, and end. With the default
slop the same movement does not produce either drag. The two exact
comparisons and nine adjacent drag/drop controls passed.
