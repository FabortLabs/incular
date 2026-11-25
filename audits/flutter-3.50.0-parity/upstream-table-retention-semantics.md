
## Row reveal after a retained layout change

An authored Flutter probe confirms an upstream cache issue: in a 120px
viewport, growing the first row from 50px to 100px moves the second
40px row down by 50px and preserves its semantic ID. Its retained
ShowOnScreen action leaves offset 0. A fresh Table at the same geometry
reveals the second row and reaches offset 20. Flutter registered the
closure against the first row rectangle.

Incular deliberately reveals the **current** bounds, reaching offset
20 for both the retained and fresh Table. The runtime test verifies
that the old synthetic ID cannot dispatch after removal. Existing
PageView reveal tests still pass. This authored comparison does not
alter the pinned original coverage count.
