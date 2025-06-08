use std::collections::HashMap;

use egui::{Id, Ui, Vec2};

use crate::NodeId;

#[derive(Clone, Debug)]
pub(crate) struct DragState<NodeIdType> {
    pub dragged: Vec<NodeIdType>,
    pub simplified: Vec<NodeIdType>,
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
    node_states: HashMap<NodeIdType, bool>,
    /// Wether or not the context menu was open last frame.
    pub(crate) context_menu_was_open: bool,
    /// The last node that was clicked. Used for double click detection.
    pub(crate) last_clicked_node: Option<NodeIdType>,
    /// If and what is being dragged.
    dragged: Option<DragState<NodeIdType>>,
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
            node_states: HashMap::new(),
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
    pub fn expand_parents_of(&mut self, _id: &NodeIdType) {
        println!("TreeViewState::expand_parents_of not yet implemented");
    }

    /// Expand the node and all its parent nodes.
    /// Effectively this makes the node visible in the tree.
    pub fn expand_node(&mut self, _id: &NodeIdType) {
        println!("TreeViewState::expand_node not yet implemented");
    }

    /// Set the openness state of a node.
    pub fn set_openness(&mut self, id: NodeIdType, open: bool) {
        self.node_states.insert(id.clone(), open);
    }

    pub(crate) fn toggle_openness(&mut self, id: &NodeIdType) {
        if let Some(openness) = self.node_states.get_mut(id) {
            *openness = !*openness;
        }
    }

    pub(crate) fn is_open(&self, id: &NodeIdType) -> Option<bool> {
        self.node_states.get(id).cloned()
    }

    /// Get the parent id of a node.
    #[deprecated = "The TreeViewState no longer carries this information. Refer to your own data source"]
    pub fn parent_id_of(&self, _id: &NodeIdType) -> Option<&NodeIdType> {
        None
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
    pub(crate) fn get_simplified_dragged(&self) -> Option<&Vec<NodeIdType>> {
        self.dragged.as_ref().map(|state| &state.simplified)
    }

    pub(crate) fn set_dragged(&mut self, dragged: DragState<NodeIdType>) {
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

    pub(crate) fn get_dragged(&self) -> Option<&Vec<NodeIdType>> {
        self.dragged.as_ref().map(|state| &state.dragged)
    }

    /// Is the given id part of a valid drag.
    pub(crate) fn is_dragged(&self, id: &NodeIdType) -> bool {
        self.dragged
            .as_ref()
            .is_some_and(|state| state.dragged.contains(id))
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
        self.last_clicked_node = Some(id.clone());
    }
    pub(crate) fn get_selection_cursor(&self) -> Option<&NodeIdType> {
        self.selection_cursor.as_ref()
    }
    pub(crate) fn get_selection_pivot(&self) -> Option<&NodeIdType> {
        self.selection_pivot.as_ref()
    }
    pub(crate) fn get_selection(&self) -> &Vec<NodeIdType> {
        &self.selected
    }
}
