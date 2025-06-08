use crate::{node_states::NodeStates, NodeBuilder, NodeId, NodeState};

pub(crate) struct BuilderState<'a, NodeIdType> {
    nodes: &'a mut NodeStates<NodeIdType>,

    node_count: usize,
    last_node_id_added: Option<NodeIdType>,
}
impl<'a, NodeIdType: NodeId> BuilderState<'a, NodeIdType> {
    pub fn new(nodes: &'a mut NodeStates<NodeIdType>) -> Self {
        Self {
            nodes,
            node_count: 0,
            last_node_id_added: None,
        }
    }

    pub fn update_and_insert_node<'ui>(
        &mut self,
        node: &NodeBuilder<'ui, NodeIdType>,
    ) -> bool {
        let is_open;
        let last_node_state = self.nodes.get_mut(&node.id);
        if let Some(last_node_state) = last_node_state {
            is_open = last_node_state.open;
            *last_node_state = NodeState {
                open: is_open,
            };
        } else {
            is_open = node.default_open;
            self.nodes.insert(
                node.id.clone(),
                NodeState {
                    open: is_open,
                },
            );
        }

        self.last_node_id_added = Some(node.id.clone());
        self.node_count += 1;
        is_open
    }

    pub fn toggle_open(&mut self, id: &NodeIdType) {
        let Some(node_state) = self.nodes.get_mut(id) else {
            return;
        };
        node_state.open = !node_state.open;
    }
}
