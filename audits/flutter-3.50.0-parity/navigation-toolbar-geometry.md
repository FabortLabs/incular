# NavigationToolbar slot geometry

This is a neutral widgets reference probe, not a Material/Cupertino app bar or an upstream-registration count. All five authored Flutter tests passed against the pinned 3.50.0 checkout. The focused Incular target has one pass and four intentional failures.

- A 300x60 toolbar with 80px leading and 20px trailing centers a 40px middle at x=130 in Flutter, vs x=160 in Incular. Flutter stretches leading to 60px high at y=0; Incular keeps 24px at y=18. The trailing rectangle matches.
- Equal 50px side widths give x=130 in both. This is a positive positioning control, not confirmation of the unequal case.
- Uncentered LTR uses default gap 16 and explicit gap 24 in Flutter (middle x=96/104). Both values give x=160 in Incular, including explicit configuration through its typed builder.
- In the mirrored uncentered RTL case, side x positions match but the middle starts at x=164 in Flutter vs 100 in Incular. Leading also fails to stretch.
- When leading/trailing slots encroach, Flutter reserves both middle gaps and constrains width. For [(leading,middle,trailing)=(200,100,20),(20,100,140),(180,100,100)], middle (x,width) is [(216,48),(44,100),(196,0)] in Flutter vs [(200,80),(40,100),(180,20)] in Incular.

Reference: run-reference-probe.ps1 -Test navigation_toolbar_test.dart. Incular: cargo test --locked -p incular-widgets --test flutter350_navigation_toolbar_parity -- --nocapture.
