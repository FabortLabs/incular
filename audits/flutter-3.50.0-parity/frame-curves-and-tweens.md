# Curves and tweens recheck

Five strict numeric frame fixtures matched Flutter 3.50.0
`ab2912035e`: scratch batch `20261124-160443-853`.

- `anim_curves__curves__interior`
- `anim_curves__curve_classes__edges`
- `anim_curves__tweens__scalar`
- `anim_curves__tweens__typed`
- `anim_curves__tween_sequence__weighted`

These compare the listed sampled outputs. The independent null endpoint
and large-integer midpoint cases stay red in the dedicated
`flutter_controller_and_physics_regressions` test target.
