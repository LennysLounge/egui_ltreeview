This is a tree view widget for egui

This tree view widget implements all the common features of a tree view to get you
up and running as fast as possible.

**Features**:
* Directory and leaf nodes
* Node selection
* Select multiple nodes
* Keyboard navigation using arrow keys
* Frontend for Drag and Drop support
* Agnostic to the implementation of your data.

# Crate feature flags
* `persistence` Adds serde to [`NodeId`] and enabled the `persistence` feature of egui.

# Getting started
```
let id = ui.make_persistent_id("Names tree view");
TreeView::new(id).show(ui, |builder| {
    builder.dir(0, "Root");
    builder.leaf(1, "Ava");
    builder.leaf(2, "Benjamin");
    builder.leaf(3, "Charlotte");
    builder.close_dir();
});
```
Create a new [`TreeView`] with its unique id and show it for the current ui.
Use the [`builder`](TreeViewBuilder) in the callback to add directories and leaves
to the tree. The nodes of the tree must have a unqiue id which implements the [`NodeId`] trait.

# Customizing the tree view
To change the basic settings of the tree view you can use the [`TreeViewSettings`] to customize the tree view
or use the convienience methods on [`TreeView`] directly.

Check out [`TreeViewSettings`] for all settings possible on the tree view.

```
TreeView::new(id)
    .with_settings(TreeViewSettings{
        override_indent: Some(15),
        fill_space_horizontal: true,
        fill_space_vertical: true,
        ..Default::default()
    })
    .max_height(200)
    .show(ui, |builder| {
    ...
});
```

# Customizing nodes, directories and leaves
To customize nodes, directories, and leaves you can use the [`NodeBuilder`] before adding the node
to the [`builder`](TreeViewBuilder).
Here you can add an icon to the node that is shown infront of the label. For directories you can also
show a custom closer. It is also possible to configure the context menu for this node specifically. More
about context menus in the context menu section.

Look at [`NodeBuilder`] for all configuration options of a node.
```
TreeView::new(id).show(ui, |builder| {
    builder.node(NodeBuilder::dir(0)
        .default_open(false)
        .label("Root")
        .icon(|ui| {
            egui::Image::new(egui::include_image!("settings.png"))
                .tint(ui.visuals().widgets.noninteractive.fg_stroke.color)
                .paint_at(ui, ui.max_rect());
        }));
    // other leaves or directories
    builder.close_dir(); // dont forget to close the root directory at the end.
});
```
# Multi select
The tree view supports selecting multiple nodes at once. This behavior was modeled after the
windows file exploror and supports all the common keyboard navigation behaviors.

Clicking on a node selects this node. Shift clicking will select all nodes between the previously selected
node (the pivot node) and the newly clicked node. Control clicking (command click on mac) will add the
clicked node to the selection or remove it from the selection if it was already selected.

You can use the arrow keys to move the selection through the tree. If you hold either shift or control(command on mac)
while navigating with the arrow keys you will move a cursor through the tree instead. How nodes are selected in this
mode depends on the configuration of shift and control being held down.
* **shift only** this will select all nodes between the pivot node and the cursor.
* **control only** Only moves the cursor. Pressing space will either select or deselect the current node underneath the cursor
* **shift and control** Every node the cursor reaches is added to the selection.

You can disable multi selection by setting [`allow_multi_select`](TreeView::allow_multi_selection) to
false in the [`TreeView`] or the [`TreeViewSettings`].

# Context menus
You can add a context menu to a node by specifying it in the [`NodeBuilder`].
```
treebuilder.node(NodeBuilder::leaf(0)
    .context_menu(|ui|{
        ui.label("i am the context menu for this node")
    }));
```
If a node was right clicked but did not configure a context menu then the [`fallback context menu`](TreeView::fallback_context_menu)
will be used.

The [`fallback context menu`](TreeView::fallback_context_menu) in the [`TreeView`] also serves as the context menu
for right clicking on multiple nodes in a multi selection. Here the list of all nodes that belong to this context menu is passed in

```
TreeView::new(id)
    .fallback_context_menu(|ui, nodes| {
        for node in nodes{
            ui.label(format!("selected node: {}", node));
        }
    })
    .show(ui, |builder| {
        builder.dir(0, "Root");
        builder.leaf(1, "Ava");
        builder.leaf(2, "Benjamin");
        builder.leaf(3, "Charlotte");
        builder.close_dir();
    });
```


**A side node about sizing of the context menu:**  
All nodes and the fallback share the same context menu. In egui, the size of a context menu
is determined the first time the context menu becomes visible. For this reason, you might have
to set the size of the context menu manually with `ui.set_width` if you plan on having multiple
differently sized contxt menues in your tree.

# Drag and drop
The tree supports the frontend for creating drag and drop actions in your tree.
Since this crate is agnostic to the implementation of the data used to create the tree, it will
create a list of [`Action`]s as part of its response. It is up to the user to implement these actions
correctly for the drag and drop to work.

An [`Action`] can contain:
* A `Drag` action shows that a node has been dragged but not yet dropped.  
* A `Move` action shows that the node has been dropped

A node can control if it wants to be a valid target of a `Drag` or `Move` action by setting
its [`drop_allowed`](NodeBuilder::drop_allowed) property.
