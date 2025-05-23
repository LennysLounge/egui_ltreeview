use egui::{Id, Key, Modifiers, Pos2, Ui, Vec2};

use crate::{node_states::NodeStates, NodeId};

/// State of the dragged node.
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct DragState<NodeIdType> {
    /// Id of the dragged nodes.
    pub node_ids: Vec<NodeIdType>,
    /// Position of the pointer when the drag started.
    pub drag_start_pos: Pos2,
    /// A drag only becomes valid after it has been dragged for
    /// a short distance.
    pub drag_valid: bool,
}
/// State of each node in the tree.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct NodeState<NodeIdType> {
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
    /// Wether this node can be activated.
    pub activatable: bool,
    /// The position of this node in the tree.
    pub position: usize,
    /// The node id of the previous node.
    pub previous: Option<NodeIdType>,
    /// The node id of the next node.
    pub next: Option<NodeIdType>,
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
    /// The element where the selection curosr is at the moment.
    selection_cursor: Option<NodeIdType>,
    /// Information about the dragged node.
    pub(crate) dragged: Option<DragState<NodeIdType>>,
    /// Id of the node that was right clicked.
    pub(crate) secondary_selection: Option<NodeIdType>,
    /// The rectangle the tree view occupied.
    pub(crate) size: Vec2,
    /// Open states of the dirs in this tree.
    node_states: NodeStates<NodeIdType>,
    /// Wether or not the context menu was open last frame.
    pub(crate) context_menu_was_open: bool,
    /// The last node that was clicked. Used for double click detection.
    pub(crate) last_clicked_node: Option<NodeIdType>,
}

impl<NodeIdType> Default for TreeViewState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            selection_pivot: None,
            selection_cursor: None,
            dragged: Default::default(),
            secondary_selection: Default::default(),
            size: Vec2::default(),
            node_states: NodeStates::new(),
            context_menu_was_open: false,
            last_clicked_node: None,
        }
    }
}
impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: NodeId + Send + Sync + 'static,
{
    /// Load a [`TreeViewState`] from memory.
    pub fn load(ui: &mut Ui, id: Id) -> Option<Self> {
        ui.data_mut(|d| d.get_persisted(id))
    }
    /// Store this [`TreeViewState`] to memory.
    pub fn store(self, ui: &mut Ui, id: Id) {
        ui.data_mut(|d| d.insert_persisted(id, self));
    }
}

impl<NodeIdType: NodeId> TreeViewState<NodeIdType> {
    /// Return the list of selected nodes
    pub fn selected(&self) -> &Vec<NodeIdType> {
        &self.selected
    }

    /// Set which nodes are selected in the tree
    pub fn set_selected(&mut self, selected: Vec<NodeIdType>) {
        self.selection_pivot = selected.first().copied();
        self.selected = selected;
    }

    /// Set a single node to be selected.
    pub fn set_one_selected(&mut self, selected: NodeIdType) {
        self.selection_pivot = Some(selected);
        self.selected.clear();
        self.selected.push(selected);
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
        while let Some(node_state) = self.node_state_of_mut(&id) {
            node_state.open = true;
            id = match node_state.parent_id {
                Some(id) => id,
                None => break,
            }
        }
    }

    /// Get the parent id of a node.
    pub fn parent_id_of(&self, id: NodeIdType) -> Option<NodeIdType> {
        self.node_state_of(&id)
            .and_then(|node_state| node_state.parent_id)
    }

    pub(crate) fn node_states(&self) -> &NodeStates<NodeIdType> {
        &self.node_states
    }

    pub(crate) fn selection_cursor(&self) -> Option<NodeIdType> {
        self.selection_cursor
    }

    /// Get the node state for an id.
    pub(crate) fn node_state_of(&self, id: &NodeIdType) -> Option<&NodeState<NodeIdType>> {
        self.node_states.get(id)
    }
    /// Get the node state for an id.
    pub(crate) fn node_state_of_mut(
        &mut self,
        id: &NodeIdType,
    ) -> Option<&mut NodeState<NodeIdType>> {
        self.node_states.get_mut(id)
    }

    /// Is the current drag valid.
    /// `false` if no drag is currently registered.
    pub(crate) fn drag_valid(&self) -> bool {
        self.dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid)
    }

    pub(crate) fn is_selected(&self, id: &NodeIdType) -> bool {
        self.selected.contains(id)
    }

    pub(crate) fn handle_click(
        &mut self,
        clicked_id: NodeIdType,
        modifiers: Modifiers,
        allow_multi_select: bool,
    ) {
        if modifiers.command_only() && allow_multi_select {
            if self.selected.contains(&clicked_id) {
                self.selected.retain(|id| id != &clicked_id);
            } else {
                self.selected.push(clicked_id);
            }
            self.selection_pivot = Some(clicked_id);
            self.selection_cursor = None;
        } else if modifiers.shift_only() && allow_multi_select {
            if let Some(selection_pivot) = self.selection_pivot {
                self.selected.clear();
                self.node_states
                    .iter_from_to(&clicked_id, &selection_pivot)
                    .for_each(|ns| self.selected.push(ns.id));
            } else {
                self.selected.clear();
                self.selected.push(clicked_id);
                self.selection_pivot = Some(clicked_id);
            }
            self.selection_cursor = None;
        } else {
            self.selected.clear();
            self.selected.push(clicked_id);
            self.selection_pivot = Some(clicked_id);
            self.selection_cursor = None;
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        key: &Key,
        modifiers: &Modifiers,
        allow_multi_select: bool,
    ) {
        match key {
            Key::ArrowUp | Key::ArrowDown => 'arm: {
                let Some(pivot_id) = self.selection_pivot else {
                    break 'arm;
                };
                let Some(current_cursor_id) = self.selection_cursor.or(self.selection_pivot) else {
                    break 'arm;
                };
                let new_cursor = match key {
                    Key::ArrowUp => self.node_states.find_previously_visible(&current_cursor_id),
                    Key::ArrowDown => self.node_states.find_next_visible(&current_cursor_id),
                    _ => unreachable!(),
                };
                if let Some(new_cursor) = new_cursor {
                    if modifiers.shift_only() && allow_multi_select {
                        self.selection_cursor = Some(new_cursor.id);
                        self.selected.clear();
                        self.node_states
                            .iter_from_to(&new_cursor.id, &pivot_id)
                            .for_each(|ns| self.selected.push(ns.id));
                    } else if modifiers.command_only() && allow_multi_select {
                        self.selection_cursor = Some(new_cursor.id);
                    } else if modifiers.shift && modifiers.command && allow_multi_select {
                        if !self.selected.contains(&new_cursor.id) {
                            self.selected.push(new_cursor.id);
                        }
                        self.selection_cursor = Some(new_cursor.id);
                    } else {
                        self.selected.clear();
                        self.selected.push(new_cursor.id);
                        self.selection_pivot = Some(new_cursor.id);
                        self.selection_cursor = None;
                    }
                }
            }
            Key::Space => 'arm: {
                let Some(cursor_id) = self.selection_cursor else {
                    break 'arm;
                };
                if self.selected.contains(&cursor_id) {
                    self.selected.retain(|id| id != &cursor_id);
                    self.selection_pivot = Some(cursor_id);
                } else {
                    self.selected.push(cursor_id);
                    self.selection_pivot = Some(cursor_id);
                }
            }
            Key::ArrowLeft => 'arm: {
                if self.selected.len() != 1 {
                    break 'arm;
                }
                let selected_node = self.selected[0];
                let node = self.node_state_of_mut(&selected_node).unwrap();
                if node.open && node.dir && node.visible {
                    node.open = false;
                } else {
                    let node_id = node.id;
                    if let Some(parent_node_id) =
                        self.first_visible_parent_of(node_id).map(|n| n.id)
                    {
                        self.selected.clear();
                        self.selected.push(parent_node_id);
                        self.selection_pivot = Some(parent_node_id);
                    }
                }
            }
            Key::ArrowRight => 'arm: {
                if self.selected.len() != 1 {
                    break 'arm;
                }
                let selected_node = self.selected[0];
                let node = self.node_state_of_mut(&selected_node).unwrap();
                if node.dir {
                    if !node.open {
                        node.open = true;
                    } else {
                        let node_id = node.id;
                        let next_visible = self.node_states.find_next_visible(&node_id);
                        if let Some(next_visible) = next_visible {
                            if self.node_states.is_child_of(&next_visible.id, &node_id) {
                                self.set_one_selected(next_visible.id);
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    fn first_visible_parent_of(&self, id: NodeIdType) -> Option<&NodeState<NodeIdType>> {
        let mut next_parent = self.node_state_of(&id).and_then(|n| n.parent_id);
        while let Some(next_parent_id) = next_parent {
            let node = self.node_state_of(&next_parent_id).unwrap();
            if node.visible {
                return Some(node);
            }
            next_parent = node.parent_id;
        }
        None
    }

    pub(crate) fn prune_selection_to_known_ids(&mut self) {
        self.selected.retain(|id| self.node_states.contains_key(id));
    }
    pub(crate) fn prune_selection_to_single_id(&mut self) {
        if self.selected.len() > 1 {
            let new_selection = self.selected[0];
            self.set_one_selected(new_selection);
        }
    }

    pub(crate) fn split<'a>(
        &'a mut self,
    ) -> (
        &'a mut NodeStates<NodeIdType>,
        PartialTreeViewState<'a, NodeIdType>,
    ) {
        let TreeViewState {
            selected,
            dragged,
            secondary_selection,
            node_states,
            ..
        } = self;
        (
            node_states,
            PartialTreeViewState {
                selected,
                dragged,
                secondary_selection,
            },
        )
    }
}

/// Represents the state of the tree view.
///
/// This holds which node is selected and the open/close
/// state of the directories.
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct PartialTreeViewState<'a, NodeIdType> {
    /// Id of the node that was selected.
    selected: &'a Vec<NodeIdType>,
    /// Information about the dragged node.
    pub(crate) dragged: &'a Option<DragState<NodeIdType>>,
    /// Id of the node that was right clicked.
    pub(crate) secondary_selection: &'a Option<NodeIdType>,
}
impl<NodeIdType: NodeId> PartialTreeViewState<'_, NodeIdType> {
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
            .is_some_and(|drag_state| drag_state.drag_valid && drag_state.node_ids.contains(id))
    }

    pub(crate) fn is_selected(&self, id: &NodeIdType) -> bool {
        self.selected.contains(id)
    }

    pub(crate) fn is_secondary_selected(&self, id: &NodeIdType) -> bool {
        self.secondary_selection.as_ref().is_some_and(|n| n == id)
    }
    pub(crate) fn selected_count(&self) -> usize {
        self.selected.len()
    }
}
