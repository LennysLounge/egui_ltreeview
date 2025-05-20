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
            self.first = Some(node_id);
        }
        self.states.insert(node_id, state);
    }
    pub(crate) fn iter<'a>(&'a self) -> indexmap::map::Iter<'a, NodeIdType, NodeState<NodeIdType>> {
        self.states.iter()
    }
    pub(crate) fn first<'a>(&'a self) -> Option<(&'a NodeIdType, &'a NodeState<NodeIdType>)> {
        self.states.first()
    }
    pub(crate) fn iter_from_to<'a>(
        &'a self,
        from: &NodeIdType,
        to: &NodeIdType,
    ) -> IterFromTo<'a, NodeIdType> {
        IterFromTo::new(self, *from, *to)
    }

    pub(crate) fn is_child_of(&self, child_id: &NodeIdType, parent_id: &NodeIdType) -> bool {
        let mut current_id = *child_id;

        loop {
            let Some(current_node) = self.states.get(&current_id) else {
                return false;
            };
            let Some(current_parent_id) = current_node.parent_id else {
                return false;
            };

            if current_parent_id == *parent_id {
                return true;
            }
            current_id = current_parent_id;
        }
    }

    pub(crate) fn find_previously_visible(
        &self,
        start_id: &NodeIdType,
    ) -> Option<&NodeState<NodeIdType>> {
        let mut current_id = self.states.get(start_id)?.previous?;
        loop {
            let state = self.states.get(&current_id)?;
            if state.visible {
                return Some(state);
            }
            current_id = state.previous?;
        }
    }
    pub(crate) fn find_next_visible(
        &self,
        start_id: &NodeIdType,
    ) -> Option<&NodeState<NodeIdType>> {
        let mut current_id = self.states.get(start_id)?.next?;
        loop {
            let state = self.states.get(&current_id)?;
            if state.visible {
                return Some(state);
            }
            current_id = state.next?;
        }
    }
}

pub(crate) enum IterFromTo<'a, NodeIdType> {
    Invalid,
    Valid {
        nodes: &'a NodeStates<NodeIdType>,
        current: NodeIdType,
        to: NodeIdType,
    },
}
impl<'a, NodeIdType: NodeId> Iterator for IterFromTo<'a, NodeIdType> {
    type Item = &'a NodeState<NodeIdType>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IterFromTo::Invalid => None,
            IterFromTo::Valid { nodes, current, to } => {
                let state = nodes.get(&current)?;
                match state.next {
                    Some(next) => {
                        if current == to {
                            *self = Self::Invalid
                        } else {
                            *current = next
                        }
                    }
                    None => *self = Self::Invalid,
                }
                Some(state)
            }
        }
    }
}
impl<'a, NodeIdType: NodeId> IterFromTo<'a, NodeIdType> {
    pub fn new(nodes: &'a NodeStates<NodeIdType>, a: NodeIdType, b: NodeIdType) -> Self {
        let a_state = nodes.get(&a);
        let b_state = nodes.get(&b);

        let (from, to) = match a_state.zip(b_state) {
            Some((a, b)) => {
                if a.position < b.position {
                    (a.id, b.id)
                } else {
                    (b.id, a.id)
                }
            }
            None => return Self::Invalid,
        };
        Self::Valid {
            nodes,
            current: from,
            to,
        }
    }
}
