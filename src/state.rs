use egui::{Id, Ui, Vec2};

use crate::{node_states::NodeStates, NodeId};

#[derive(Clone, Debug)]
pub(crate) enum Dragged<NodeIdType> {
    One(NodeIdType),
    Selection,
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
    /// The position of this node in the tree.
    pub position: usize,
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
    /// If and what is being dragged.
    dragged: Option<Dragged<NodeIdType>>,
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
        self.selection_pivot = selected.first().cloned();
        self.selected = selected;
    }

    /// Set a single node to be selected.
    pub fn set_one_selected(&mut self, selected: NodeIdType) {
        self.selection_pivot = Some(selected.clone());
        self.selected.clear();
        self.selected.push(selected);
    }

    /// Expand all parent nodes of the node with the given id.
    pub fn expand_parents_of(&mut self, id: &NodeIdType) {
        if let Some(parent_id) = self.parent_id_of(id) {
            self.expand_node(&parent_id.clone());
        }
    }

    /// Expand the node and all its parent nodes.
    /// Effectively this makes the node visible in the tree.
    pub fn expand_node(&mut self, id: &NodeIdType) {
        let mut current_id = id.clone();
        while let Some(node_state) = self.node_state_of_mut(&current_id) {
            node_state.open = true;
            current_id = match node_state.parent_id.as_ref() {
                Some(id) => id.clone(),
                None => break,
            }
        }
    }

    /// Set the openness state of a node.
    pub fn set_openness(&mut self, id: &NodeIdType, open: bool) {
        if let Some(node_state) = self.node_state_of_mut(id) {
            node_state.open = open;
        }
    }

    /// Get the parent id of a node.
    pub fn parent_id_of(&self, id: &NodeIdType) -> Option<&NodeIdType> {
        self.node_state_of(id)
            .and_then(|node_state| node_state.parent_id.as_ref())
    }

    pub(crate) fn node_states(&self) -> &NodeStates<NodeIdType> {
        &self.node_states
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

    pub(crate) fn prune_selection_to_known_ids(&mut self) {
        self.selected.retain(|id| self.node_states.contains_key(id));
    }
    pub(crate) fn prune_selection_to_single_id(&mut self) {
        if self.selected.len() > 1 {
            let new_selection = self.selected[0].clone();
            self.set_one_selected(new_selection);
        }
    }
    pub(crate) fn get_dragged(&self) -> Vec<NodeIdType> {
        match &self.dragged {
            Some(Dragged::One(id)) => vec![id.clone()],
            Some(Dragged::Selection) => self.selected.clone(),
            None => Vec::new(),
        }
    }
    pub(crate) fn set_dragged(&mut self, dragged: Dragged<NodeIdType>) {
        self.dragged = Some(dragged);
    }
    pub(crate) fn reset_dragged(&mut self) {
        self.dragged = None;
    }
    pub(crate) fn set_pivot(&mut self, id: Option<NodeIdType>) {
        self.selection_pivot = id;
    }
    pub(crate) fn set_cursor(&mut self, id: Option<NodeIdType>) {
        self.selection_cursor = id;
    }
    pub(crate) fn toggle_selected(&mut self, id: &NodeIdType) {
        if self.selected.contains(id) {
            self.selected.retain(|selected_id| selected_id != id);
        } else {
            self.selected.push(id.clone());
        }
    }
    /// Set which nodes are selected in the tree
    pub(crate) fn set_selected_dont_change_pivot(&mut self, selected: Vec<NodeIdType>) {
        self.selected = selected;
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
            selection_cursor,
            node_states,
            last_clicked_node,
            selection_pivot,
            ..
        } = self;
        (
            node_states,
            PartialTreeViewState {
                selected,
                dragged,
                secondary_selection,
                selection_cursor,
                last_clicked_node,
                selection_pivot,
            },
        )
    }
}

/// Represents the state of the tree view.
///
/// This holds which node is selected and the open/close
/// state of the directories.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct PartialTreeViewState<'a, NodeIdType> {
    /// Id of the node that was selected.
    selected: &'a Vec<NodeIdType>,
    /// Information about the dragged node.
    dragged: &'a Option<Dragged<NodeIdType>>,
    /// Id of the node that was right clicked.
    secondary_selection: &'a Option<NodeIdType>,
    /// The element where the selection curosr is at the moment.
    selection_cursor: &'a Option<NodeIdType>,
    /// The last node that was clicked. Used for double click detection.
    last_clicked_node: &'a mut Option<NodeIdType>,
    /// The pivot element used for selection.
    selection_pivot: &'a Option<NodeIdType>,
}
impl<NodeIdType: NodeId> PartialTreeViewState<'_, NodeIdType> {
    /// Is the given id part of a valid drag.
    pub(crate) fn is_dragged(&self, id: &NodeIdType) -> bool {
        match self.dragged {
            Some(Dragged::One(dragged_id)) => dragged_id == id,
            Some(Dragged::Selection) => self.selected.contains(id),
            None => false,
        }
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
    pub(crate) fn is_selection_cursor(&self, id: &NodeIdType) -> bool {
        self.selection_cursor
            .as_ref()
            .is_some_and(|cursor_id| cursor_id == id)
    }
    pub(crate) fn is_selection_pivot(&self, id: &NodeIdType) -> bool {
        self.selection_pivot
            .as_ref()
            .is_some_and(|pivot_id| pivot_id == id)
    }
    pub(crate) fn was_clicked_last(&self, id: &NodeIdType) -> bool {
        self.last_clicked_node
            .as_ref()
            .is_some_and(|last| last == id)
    }
    pub(crate) fn set_last_clicked(&mut self, id: &NodeIdType) {
        *self.last_clicked_node = Some(id.clone());
    }
    pub(crate) fn get_selection_cursor(&self) -> Option<&NodeIdType> {
        self.selection_cursor.as_ref()
    }
    pub(crate) fn get_selection_pivot(&self) -> Option<&NodeIdType> {
        self.selection_pivot.as_ref()
    }
}
