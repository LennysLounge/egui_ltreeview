use indexmap::IndexMap;

use crate::{NodeId, NodeState};

#[derive(Clone)]
pub(crate) struct NodeStates<NodeIdType> {
    states: IndexMap<NodeIdType, NodeState>,
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
    pub(crate) fn get_mut(&mut self, id: &NodeIdType) -> Option<&mut NodeState> {
        self.states.get_mut(id)
    }
    pub(crate) fn insert(&mut self, node_id: NodeIdType, state: NodeState) {
        if self.first.is_none() {
            self.first = Some(node_id.clone());
        }
        self.states.insert(node_id.clone(), state);
    }
}
