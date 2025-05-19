use std::ops::{Index, Range, RangeFrom, RangeInclusive};

use indexmap::IndexMap;

use crate::{NodeId, NodeState};

#[derive(Clone)]
pub(crate) struct NodeStates<NodeIdType> {
    states: IndexMap<NodeIdType, NodeState<NodeIdType>>,
}

impl<NodeIdType> NodeStates<NodeIdType> {
    pub fn new() -> Self {
        Self {
            states: IndexMap::new(),
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

    pub(crate) fn position_of_id(&self, id: NodeIdType) -> Option<usize> {
        self.states.get_index_of(&id)
    }
    pub(crate) fn insert(&mut self, node_id: NodeIdType, state: NodeState<NodeIdType>) {
        self.states.insert(node_id, state);
    }
    pub(crate) fn iter<'a>(&'a self) -> indexmap::map::Iter<'a, NodeIdType, NodeState<NodeIdType>> {
        self.states.iter()
    }
    pub(crate) fn first<'a>(&'a self) -> Option<(&'a NodeIdType, &'a NodeState<NodeIdType>)> {
        self.states.first()
    }
}

impl<NodeIdType> Index<usize> for NodeStates<NodeIdType> {
    type Output = NodeState<NodeIdType>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.states[index]
    }
}
impl<NodeIdType> Index<RangeInclusive<usize>> for NodeStates<NodeIdType> {
    type Output = indexmap::map::Slice<NodeIdType, NodeState<NodeIdType>>;

    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        &self.states[index]
    }
}

impl<NodeIdType> Index<RangeFrom<usize>> for NodeStates<NodeIdType> {
    type Output = indexmap::map::Slice<NodeIdType, NodeState<NodeIdType>>;

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.states[index]
    }
}

impl<NodeIdType> Index<Range<usize>> for NodeStates<NodeIdType> {
    type Output = indexmap::map::Slice<NodeIdType, NodeState<NodeIdType>>;

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.states[index]
    }
}

impl<'a, NodeIdType> IntoIterator for &'a NodeStates<NodeIdType> {
    type Item = (&'a NodeIdType, &'a NodeState<NodeIdType>);

    type IntoIter = indexmap::map::Iter<'a, NodeIdType, NodeState<NodeIdType>>;

    fn into_iter(self) -> Self::IntoIter {
        self.states.iter()
    }
}
