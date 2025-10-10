# v0.6.0

Fixes:
* Fix an issue where the drag external and move external action were always output even if the drag was entirely within the tree. Closes issue #28
* When dragging multiple nodes some nodes where represented multiple times in the source vector. This no longer happens. Reported by @hydra.
* Fix an issue where the `allow_drag_and_drop` tree setting did not do anything.
* Fix a panic when quickly dragging and dropping a node outside the native window.

Changes:
* The minimum width of the tree view is no longer persisted.  
This allows the tree view to shrink to a smaller width after restarting the program.

The width of the tree view will grow to fit the largest node it has to render and then never return to a smaller size until the 


New features:
* Added a `override_striped` setting to the tree view to turn on/off highlighting every second node.
