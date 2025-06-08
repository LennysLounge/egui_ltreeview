use indexmap::IndexMap;

use crate::{NodeId, NodeState};

#[derive(Clone)]
pub(crate) struct NodeStates<NodeIdType> {
    states: IndexMap<NodeIdType, NodeState<NodeIdType>>,
    first: Option<NodeIdType>,
}

impl<NodeIdType> NodeStates<NodeIdType> {
    pub fn new() -> Self {
        Self {
            states: IndexMap::new(),
            first: None,
        }
    }
}

impl<NodeIdType: NodeId> NodeStates<NodeIdType> {
    pub fn contains_key(&self, node_id: &NodeIdType) -> bool {
        self.states.contains_key(node_id)
    }

    /// Get the node state for an id.
    pub(crate) fn get(&self, id: &NodeIdType) -> Option<&NodeState<NodeIdType>> {
        self.states.get(id)
    }
    /// Get the node state for an id.
    pub(crate) fn get_mut(&mut self, id: &NodeIdType) -> Option<&mut NodeState<NodeIdType>> {
        self.states.get_mut(id)
    }
    pub(crate) fn insert(&mut self, node_id: NodeIdType, state: NodeState<NodeIdType>) {
        if self.first.is_none() {
            self.first = Some(node_id.clone());
        }
        self.states.insert(node_id.clone(), state);
    }

    pub(crate) fn is_child_of(&self, child_id: &NodeIdType, parent_id: &NodeIdType) -> bool {
        let mut current_id = child_id.clone();

        loop {
            let Some(current_node) = self.states.get(&current_id) else {
                return false;
            };
            let Some(current_parent_id) = current_node.parent_id.as_ref() else {
                return false;
            };

            if current_parent_id == parent_id {
                return true;
            }
            current_id = current_parent_id.clone();
        }
    }
}
