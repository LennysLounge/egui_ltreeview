use std::{
    collections::HashSet,
    ops::{Index, Range, RangeFrom, RangeInclusive},
};

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

    pub(crate) fn position_of_id(&self, id: NodeIdType) -> Option<usize> {
        self.states.get_index_of(&id)
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
    pub(crate) fn iter_child_nodes_of<'a>(
        &'a self,
        node_id: &NodeIdType,
    ) -> ChildIter<'a, NodeIdType> {
        ChildIter::new(self, node_id)
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

/// Iterator over all child nodes of a directory node.
pub(crate) struct ChildIter<'a, NodeIdType> {
    valid_directories: HashSet<NodeIdType>,
    nodes: &'a NodeStates<NodeIdType>,
    next: Option<NodeIdType>,
}
impl<'a, NodeIdType: NodeId> Iterator for ChildIter<'a, NodeIdType> {
    type Item = &'a NodeState<NodeIdType>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_node = self.next.and_then(|next_id| self.nodes.get(&next_id))?;
        let is_child_node = next_node
            .parent_id
            .is_some_and(|parent_id| self.valid_directories.contains(&parent_id));
        if !is_child_node {
            self.next = None;
            return None;
        }
        if next_node.dir {
            self.valid_directories.insert(next_node.id);
        }
        self.next = next_node.next;
        Some(next_node)
    }
}
impl<'a, NodeIdType: NodeId> ChildIter<'a, NodeIdType> {
    pub fn new(nodes: &'a NodeStates<NodeIdType>, start_id: &NodeIdType) -> Self {
        let next = nodes.get(start_id).and_then(|node_state| node_state.next);
        let mut valid_directories = HashSet::new();
        valid_directories.insert(*start_id);
        Self {
            valid_directories,
            nodes,
            next,
        }
    }
}
