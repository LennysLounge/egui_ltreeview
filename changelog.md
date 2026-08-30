# v0.9.1-dev

### New features:
* Added an accessory to a node.  
An accessory is an area to the right of a nodes label and right aligned. Its primarily used to add
buttons or extra information to a row. For example a 3d editor might have a "hide" button next to an object to hide this object in the rendered view.
A text editor might use the accessory to show an indicator that the file has unsaved changes.
  * To add an accessory to a node simply use the `NodeBuilder::accessory` method or implement the `NodeConfig::accessory` directly.
  * In the render order the accessory is always rendered before the label of the node. This is to make the label correctly truncate its text when there is not enough space to show the full label.
* Added options to control if the tree view will expand to take the available width or height
  * `fill_available_width` controls if the tree will automatically expand to take up the available width. Default is true.
  * `fill_available_height` controls if the tree will automatically expand to take up the available height. Default is true.
* Added a `max_width` option to the tree view.
  
### Fixes:

### Changes:
* The default label for a `NodeBuilder::label` now also truncates the label in adition to making it not selectable. The label is only truncated if the maximum width of the tree is small enough that the label cannot be shown in its full length.  
This does not apply to custom labels using `NodeBuilder::label_ui`, there you will have to add truncation yourself.

# v0.9.0

### New features:

### Fixes:
* When opening the fallback context menu for a selected node, it will now be supplied with the full list of selected ids instead of just the id of the clicked node. Closes #52
* Fix a compile error in the persistence_with_tree example. Thanks to @XCemaXX for pointing this out.

### Changes:
* Update egui to 0.36
* Disabled the default egui features to not force those onto a downstream user. Closes #50

# v0.8.0

### Changes:
* Update egui to 0.35
  * Node ids must now also implement the `Debug` trait. This is because of a change in egui where inputs into an `Id` must now also implement `Debug`

# v0.7.1

### Fixes:
* Double clicking to activate and to toggle a directory now work again. Closes #45
* Triggering a context menu on a single node that does not define its own context menu now correctly 
calls the fallback context menu again. Closes #46
* Fixes a crash when the tree view is inside a collapsible header and scrolled when the collapsable header is collapsed. Closes #47
* Showing the same nodes in two tree would previously cause an id collision when opening the context menu of either of the nodes.
This is now fixed and id of the source tree view is correctly passed as a salt. Closes #48

### Changes:

# v0.7.0

### New features:

### Fixes:
* Opening a context menu on the empty space bellow the tree will now open the fallback context menu for no nodes. Closes #38

### Changes:
* Update egui to 0.34
* When clicking on the empty space bellow the tree, the selection will now be cleared. Closes #38

# v0.6.1

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
