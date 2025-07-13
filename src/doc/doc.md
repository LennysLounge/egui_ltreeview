# Interaction
This section outlines the various interactions supported by the tree view.
These include mouse and keyboard interactions to navigate through the tree as well as interaction related to interacting with nodes directly.

## Keyboard navigation
Users can change the selected nodes using the arrow keys on their keyboard.
* `Arrow Up` Move the selection up one node.
* `Arrow Down` Move the selection down one node.
* `Arrow Right` Open the current directory
    * If the currently selected directory is closed then the directory will be opened.
    * Else this behaves the same as `Arrow Down`
* `Arrow Left` Close the current directory
    * If the currently selected directory is open then the directory will be closed.
    * Else moves the selection to the parent node of the currently selected node.

## Selecting nodes and multi selection
Left clicking on a node selects it. The selection can be changed by clicking on a different node or using the keyboard to navigate to a different node.

**Multi selection**  
Multi selection is only enabled if it is enabled in the [`TreeViewSettings`](TreeViewSettings::allow_multi_select). Otherwise only one node may be selected at the same time.

Multi selection can be accomplished either with the mouse or using the keyboard and follows the standard convention for multi selection.  
* `Shift + Click` / `Shift + Arrow Keys` selects a range of nodes.  
* `Control + Click` / `Control + Arrow Keys` Adds or removes individual nodes from the selection.  

**Multi selection with clicking**  

* `Click + no modifiers`: Select the clicked node and set the pivot node to the clicked node. Same as if multi selection is disabled.  
* `Click + Control`: Toggle the selection of the clicked node and set the pivot to the clicked node.  
* `Click + Shift`: Select all node between the pivot node and the clicked node inclusive. Does not update the pivot node.

**Multi selection with the keyboard**

For multi selection using the keyboard the cursor node is also visible. The cursor node is not part of the selection but is highlighted to make it possible to choose the next node for multi selection.

* `Arrow Up/Down + no modifiers`: Select the node next of/previous to the current pivot node. Set the pivot to the selected node. Same as if multi selection is disabled.  
* `Arrow Up/Down + control`: Move the cursor up or down.  
* `Space + control`: Toggle selection of the cursor node. Set the pivot to the cursor node.  
* `Arrow Up/Down + Shift`: Move the cursor up or down. Select all node between the pivot and the cursor inclusive. Does not update the pivot node.

## Activating nodes
Either `Double Clicking` or the `Enter` key will activate the current selection.

"Activating" a node does not have one specific meaning, instead it is up to the library user to implement the desired behavior by listening to the [`Action::Activate`] response from the [`TreeView`](`TreeView::show`). Only nodes that have [`NodeBuilder::activatable`] set to true will be part of the activate action.

A usual use case for activating nodes is opening a node in a new window. For example a file viewer might open the contents of a file in a new tab when activated.

## Drag and drop
If drag and drop is enabled in the [`TreeViewSettings::allow_drag_and_drop`], the tree view has support for implementing drag and drop in your data structure.

Click and hold on a node and start dragging the node above the tree. You will see a horizontal line, the drop marker, that signals where the node would be move to if the drag was released. Dragging a node into a directory will highlight the entire directory to show that the node would be inserted into the directory. Inserting in a directory will place the node last in the directory. To place the node at a specific point in the directory, move the node to the desired location and use the drop marker to place the node at the correct spot.

The tree view has no reference to the data structure used to feed the tree view and therefore it cannot actually move the node to the new position.
Instead it will return an [`Action`] that will have to be implemented by the library user. Read more about Drag or Move actions and drag and dropping to external widgets in the [`Action`] documentation.

**Invalid drag and drop and cancelling a drag and drop action**  
Sometimes a drag and drop action would not be valid depending on the semantics of the data displayed in the tree view. For example a directory containing only texture files might want to disallow dropping a txt file into this directory.

In this situation you might want to remove the drop marker to make it clear that this drop location is not valid. To do so call the [`DragAndDrop::remove_drop_marker`] method on the [`DragAndDrop`] structure.

## Context menus on nodes
Context menus are either configured per node or for all nodes at once.
Per node configuration uses the [`NodeBuilder::context_menu`]. If a nodes does not specify a context menu no context menu is shown.

To specify context menus for all nodes use the [`TreeView::fallback_context_menu`] method on the tree view.
Here the context menu is shown for every node in the tree. If multiple nodes are selected, this method is the only way to show a context menu for multiple nodes at once.

**A side node about sizing of the context menu**  
All nodes and the fallback share the same context menu. In egui, the size of a context menu
is determined the first time the context menu becomes visible. For this reason, you might have
to set the size of the context menu manually with `ui.set_width` if you plan on having multiple
differently sized context menus in your tree.

# Interacting with the tree view through code
## Changing selection
To change the current selection of the tree view use the [`TreeViewState::set_selected`] and [`TreeViewState::set_one_selected`] methods on the [`TreeViewState`]

## Changing open state of directory
Change the open state of any directory using the [`TreeViewState::set_openness`] method on [`TreeViewState`].
You can also query the current open state of a node with the[`TreeViewState::is_open`] method.

### Implementing a "collapse all" or "expand all" feature
The tree view does not have a build in feature to create a collapse all or expand all feature.
It tries to store as little state as possible and only stores the open state of directories that have been interacted with before. For that reason the tree itself cannot know which node ids have to be opened or closed.

The library user must implement this feature themselves. The easiest way would be to iterate over all nodes and call the [`TreeViewState::set_openness`] method in the [`TreeViewState`].

# Customization
## Controlling the size of the tree view

The tree view deliberately does not offer many options to control its size, instead the size is mostly
determined automatically.  
The width of the tree view is the largest of either:
* the remaining width of the ui using [`ui.available_size().x`](https://docs.rs/egui/latest/egui/struct.Ui.html#method.available_size)
* the minimum width via [`TreeViewSettings::min_width`] or [`TreeView::min_width`]
* the largest width of any node in the tree

If the width of the tree view is determined by the widest node in the tree you might notice a limitation of the tree view. Since only visible nodes are rendered, which node is the widest node might change depending on which nodes are visible. This can causes jittering of the width when scrolling through the tree. To create a smooth scrolling experience the tree view stores the width of the widest node it rendered to its its [`TreeViewState`] and uses this for its width calculation. A limitation of this approach is that the width of the tree view will only grow and never become smaller.

The height of the tree view is the largest of either:
* the remaining height of the ui using [`ui.available_size().y`](https://docs.rs/egui/latest/egui/struct.Ui.html#method.available_size)
* the minimum height via [`TreeViewSettings::min_height`] or [`TreeView::min_height`]
* the combined hight of all nodes in the tree

**Suggestion: Wrap the tree view in a scroll area in a side panel**  
In most cases a tree view is placed in a scroll area in a left hand side panel. This can be easily done like this:
```
egui::SidePanel::left(Id::new("tree view panel"))
    .resizable(true)
    .show(ctx, |ui| {
        ScrollArea::both().show(ui, |ui| {
            TreeView::new(Id::new("tree view"))::show(ui, |builder|{
                // build your tree here
            })
        });
    });
```

**Suggestion: Control the maximum size of the tree view using a scroll area and a group**  
If you want to have the tree as a smaller part of a more complicated panel you can
control the size using the scroll area and wrap it inside a group for better separation.
```
ui.group(|ui| {
    ScrollArea::both()
        .max_height(200.0)
        .max_width(200.0)
        .show(ui, |ui| {
            TreeView::new(Id::new("tree view")).show(ui, |builder| {
                    // build your tree here
                },
            );
        });
});
```
## Customizing the tree view itself
## Customizing nodes

# Performance
## Implementing `NodeConfig` for better performance
## Culling hidden nodes

