# v0.6.1-dev

### Fixes:
* Fix a bug where a flattened node made all its children invisible.

# v0.6.0

New features:
* Update egui to 0.33
* Added a `override_striped` setting to the tree view to turn on/off highlighting every second node.

Changes:
* The minimum width of the tree view is no longer persisted.  
This allows the tree view to shrink to a smaller width after restarting the program.

Fixes:
* Fix an issue where the drag external and move external action were always output even if the drag was entirely within the tree. Closes issue #28
* When dragging multiple nodes some nodes where represented multiple times in the source vector. This no longer happens. Reported by @hydra.
* Fix an issue where the `allow_drag_and_drop` tree setting did not do anything.
* Fix a panic when quickly dragging and dropping a node outside the native window.
