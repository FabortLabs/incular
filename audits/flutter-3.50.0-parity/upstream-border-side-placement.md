# Physical border constructors and omitted edges

The exact pinned `painting/border_test.dart` registrations at :18
and :27 passed. Incular's value constructors assign all four
sides, or the vertical side to left/right and the horizontal
side to top/bottom. Dart checks identical side objects; Incular
stores these side values inline and verifies every field instead.
The original identity assertions remain partial matches in the ledger.

A second pinned probe reads actual RGBA pixels at the left and
right stroke, the omitted top and bottom, and just inside the
two vertical bands. The native display-list check independently
verifies the same six samples and that the untouched default
border emits nothing. None of the horizontal strokes are implicit.
Seven focused and two adjacent native tests passed.
