# Unreleased

Fixes:
* Fix an issue where the drag external and move external action were always output even if the drag was entirely within the tree. Closes issue #28
* When dragging multiple nodes some nodes where represented multiple times in the source vector. This no longer happens. Reported by @hydra.
* Fix an issue where the `allow_drag_and_drop` tree setting did not do anything.
* Fix a panic when quickly dragging and dropping a node outside the native window.

New Features:
* Added a `override_striped` setting to the tree view to turn on/off highlighting every second node.
