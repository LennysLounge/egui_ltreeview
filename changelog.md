# v0.6.1-dev

### Fixes:
* Fix a bug where a flattened node made all its children invisible.
* Fix a panic when dragging a dropping a node onto a custom label. Closes #42
* The ghost overlay when dragging a node would get detached from the cursor if the user scrolled while dragging
This should no longer happen.

### Changes:
* The tree state will now use temp storage if the persistence feature is not active.
Previously, if the persistence feature on egui was active it would also require the persistence feature on egui_ltreeview
to be active. That is not necessarily desired since it now also forces the NodeId to be serializable.
Now the tree state will use temp storage if the feature is not active and persistent storage once the feature is activated.
The requirement for NodeId to be serializable will therefore only show up once the feature on egui_ltreeview is used.
* Shift clicking a node when no previous selection was made will now select the clicked node instead of doing nothing. Closes #37
* Make it so that the modifiers key required for a range selection or a set selection are configurable. Closes #39

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
