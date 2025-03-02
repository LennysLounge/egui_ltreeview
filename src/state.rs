use egui::{Id, Key, Modifiers, Pos2, Ui, Vec2};

use crate::{NodeId, TreeViewId};

/// State of the dragged node.
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct DragState<NodeIdType> {
    /// Id of the dragged node.
    pub node_id: NodeIdType,
    /// Offset of the drag overlay to the pointer.
    pub drag_row_offset: Vec2,
    /// Position of the pointer when the drag started.
    pub drag_start_pos: Pos2,
    /// A drag only becomes valid after it has been dragged for
    /// a short distance.
    pub drag_valid: bool,
}
/// State of each node in the tree.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeState<NodeIdType> {
    /// Id of this node.
    pub id: NodeIdType,
    /// The parent node of this node.
    pub parent_id: Option<NodeIdType>,
    /// Wether the node is open or not.
    pub open: bool,
    /// Wether the node is visible or not.
    pub visible: bool,
    /// Wether this node is a valid target for drag and drop.
    pub drop_allowed: bool,
    /// Wether this node is a directory.
    pub dir: bool,
}

/// Represents the state of the tree view.
///
/// This holds which node is selected and the open/close
/// state of the directories.
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeViewState<NodeIdType> {
    /// Id of the node that was selected.
    selected: Vec<NodeIdType>,
    /// The pivot element used for selection.
    selection_pivot: Option<NodeIdType>,
    /// Information about the dragged node.
    pub(crate) dragged: Option<DragState<NodeIdType>>,
    /// Id of the node that was right clicked.
    pub(crate) secondary_selection: Option<NodeIdType>,
    /// The rectangle the tree view occupied.
    pub(crate) size: Vec2,
    /// Open states of the dirs in this tree.
    pub(crate) node_states: Vec<NodeState<NodeIdType>>,
}
impl<NodeIdType> Default for TreeViewState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            selection_pivot: None,
            dragged: Default::default(),
            secondary_selection: Default::default(),
            size: Vec2::default(),
            node_states: Vec::new(),
        }
    }
}
impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: NodeId,
{
    pub fn load(ui: &mut Ui, id: Id) -> Option<Self> {
        ui.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ui: &mut Ui, id: Id) {
        ui.data_mut(|d| d.insert_persisted(id, self));
    }
}

impl<NodeIdType: TreeViewId> TreeViewState<NodeIdType> {
    /// Return the list of selected nodes
    pub fn selected(&self) -> &Vec<NodeIdType> {
        &self.selected
    }

    /// Set which nodes are selected in the tree
    pub fn set_selected(&mut self, selected: Vec<NodeIdType>) {
        self.selection_pivot = selected.first().map(|o| *o);
        self.selected = selected;
    }

    /// Expand all parent nodes of the node with the given id.
    pub fn expand_parents_of(&mut self, id: NodeIdType) {
        if let Some(parent_id) = self.parent_id_of(id) {
            self.expand_node(parent_id);
        }
    }

    /// Expand the node and all its parent nodes.
    /// Effectively this makes the node visible in the tree.
    pub fn expand_node(&mut self, mut id: NodeIdType) {
        loop {
            if let Some(node_state) = self.node_state_of_mut(&id) {
                node_state.open = true;
                id = match node_state.parent_id {
                    Some(id) => id,
                    None => break,
                }
            } else {
                break;
            }
        }
    }

    /// Get the parent id of a node.
    pub fn parent_id_of(&self, id: NodeIdType) -> Option<NodeIdType> {
        self.node_state_of(&id)
            .and_then(|node_state| node_state.parent_id)
    }

    /// Get the node state for an id.
    pub(crate) fn node_state_of(&self, id: &NodeIdType) -> Option<&NodeState<NodeIdType>> {
        self.node_states.iter().find(|ns| &ns.id == id)
    }
    /// Get the node state for an id.
    pub(crate) fn node_state_of_mut(
        &mut self,
        id: &NodeIdType,
    ) -> Option<&mut NodeState<NodeIdType>> {
        self.node_states.iter_mut().find(|ns| &ns.id == id)
    }

    /// Is the current drag valid.
    /// `false` if no drag is currently registered.
    pub(crate) fn drag_valid(&self) -> bool {
        self.dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid)
    }
    /// Is the given id part of a valid drag.
    pub(crate) fn is_dragged(&self, id: &NodeIdType) -> bool {
        self.dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid && &drag_state.node_id == id)
    }

    pub(crate) fn is_selected(&self, id: &NodeIdType) -> bool {
        self.selected.contains(id)
    }

    pub(crate) fn is_secondary_selected(&self, id: &NodeIdType) -> bool {
        self.secondary_selection.as_ref().is_some_and(|n| n == id)
    }

    pub(crate) fn handle_click(&mut self, clicked_id: NodeIdType, modifiers: Modifiers) {
        if modifiers.command_only() {
            self.selected.push(clicked_id);
            self.selection_pivot = Some(clicked_id);
        } else if modifiers.shift_only() {
            if let Some(selection_pivot) = self.selection_pivot {
                self.selected.clear();

                let clicked_pos = self
                    .node_states
                    .iter()
                    .position(|node| node.id == clicked_id)
                    .unwrap();
                let pivot_pos = self
                    .node_states
                    .iter()
                    .position(|node| node.id == selection_pivot)
                    .unwrap();
                self.node_states[clicked_pos.min(pivot_pos)..=clicked_pos.max(pivot_pos)]
                    .iter()
                    .for_each(|node| self.selected.push(node.id));
            } else {
                self.selected.clear();
                self.selected.push(clicked_id);
                self.selection_pivot = Some(clicked_id);
            }
        } else {
            self.selected.clear();
            self.selected.push(clicked_id);
            self.selection_pivot = Some(clicked_id);
        }
    }

    pub(crate) fn handle_key(&mut self, key: &Key, _modifier: &Modifiers) {
        match key {
            Key::ArrowUp => 'arm: {
                let Some(pivot_id) = self.selection_pivot else {
                    break 'arm;
                };

                let pivot_pos = self
                    .node_states
                    .iter()
                    .position(|ns| ns.id == pivot_id)
                    .unwrap();
                if let Some(prev_node) = self.node_states[0..pivot_pos]
                    .iter()
                    .rev()
                    .find(|node| node.visible)
                {
                    self.selected.clear();
                    self.selected.push(prev_node.id);
                    self.selection_pivot = Some(prev_node.id);
                }
            }
            Key::ArrowDown => 'arm: {
                let Some(pivot_id) = self.selection_pivot else {
                    break 'arm;
                };

                let pivot_pos = self
                    .node_states
                    .iter()
                    .position(|ns| ns.id == pivot_id)
                    .unwrap();
                if let Some(prev_node) = self.node_states[(pivot_pos + 1)..]
                    .iter()
                    .find(|node| node.visible)
                {
                    self.selected.clear();
                    self.selected.push(prev_node.id);
                    self.selection_pivot = Some(prev_node.id);
                }
            }
            Key::ArrowLeft => 'arm: {
                if self.selected.len() != 1 {
                    break 'arm;
                }
                let node = self
                    .node_states
                    .iter_mut()
                    .find(|n| n.id == self.selected[0])
                    .unwrap();
                if node.open && node.dir {
                    node.open = false;
                } else {
                    if let Some(parent_id) = node.parent_id {
                        self.selected.clear();
                        self.selected.push(parent_id);
                        self.selection_pivot = Some(parent_id);
                    }
                }
            }
            Key::ArrowRight => 'arm: {
                if self.selected.len() != 1 {
                    break 'arm;
                }
                let node = self
                    .node_states
                    .iter_mut()
                    .find(|n| n.id == self.selected[0])
                    .unwrap();
                if !node.open && node.dir {
                    node.open = true;
                } else {
                    let node_id = node.id;
                    let first_child_node = self
                        .node_states
                        .iter()
                        .find(|n| n.parent_id == Some(node_id));
                    if let Some(first_child_node) = first_child_node {
                        self.selected.clear();
                        self.selected.push(first_child_node.id);
                        self.selection_pivot = Some(first_child_node.id);
                    }
                }
            }
            _ => (),
        }
    }
}
