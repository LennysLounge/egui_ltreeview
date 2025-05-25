use egui::Pos2;

use crate::{node_states::NodeStates, NodeBuilder, NodeId, NodeResponse, NodeState};

#[derive(Clone)]
struct DirectoryState<NodeIdType> {
    /// Id of the directory node.
    id: NodeIdType,
    /// If directory is expanded
    is_open: bool,
}
pub struct IndentState<NodeIdType> {
    /// Id of the node that created this indent
    source_node: NodeIdType,
    /// Anchor for the indent hint at the source directory
    anchor: f32,
    /// Positions of child nodes for the indent hint.
    positions: Vec<Pos2>,
}

pub(crate) struct BuilderState<'a, NodeIdType> {
    nodes: &'a mut NodeStates<NodeIdType>,
    stack: Vec<DirectoryState<NodeIdType>>,
    indents: Vec<IndentState<NodeIdType>>,
    node_count: usize,
    last_node_id_added: Option<NodeIdType>,
}
impl<'a, NodeIdType: NodeId> BuilderState<'a, NodeIdType> {
    pub fn new(nodes: &'a mut NodeStates<NodeIdType>) -> Self {
        Self {
            nodes,
            stack: Vec::new(),
            indents: Vec::new(),
            node_count: 0,
            last_node_id_added: None,
        }
    }

    pub fn update_and_insert_node<'ui>(
        &mut self,
        mut node: NodeBuilder<'ui, NodeIdType>,
    ) -> NodeBuilder<'ui, NodeIdType> {
        let parent_id = self.parent_id();
        let parent_dir_is_open = self.parent_dir_is_open();

        node.set_indent(self.get_indent());
        let last_node_state = self.nodes.get_mut(&node.id);
        if let Some(last_node_state) = last_node_state {
            node.set_is_open(last_node_state.open);
            *last_node_state = NodeState {
                id: node.id,
                parent_id: parent_id,
                open: node.is_open,
                visible: parent_dir_is_open && !node.flatten,
                drop_allowed: node.drop_allowed,
                dir: node.is_dir,
                activatable: node.activatable,
                position: self.node_count,
                previous: self.last_node_id_added,
                next: None,
            };
        } else {
            node.set_is_open(node.default_open);
            self.nodes.insert(
                node.id,
                NodeState {
                    id: node.id,
                    parent_id: parent_id,
                    open: node.is_open,
                    visible: parent_dir_is_open && !node.flatten,
                    drop_allowed: node.drop_allowed,
                    dir: node.is_dir,
                    activatable: node.activatable,
                    position: self.node_count,
                    previous: self.last_node_id_added,
                    next: None,
                },
            );
        }

        if let Some(last_node_id_added) = self.last_node_id_added {
            self.nodes
                .get_mut(&last_node_id_added)
                .expect("The previous added node id should always point to a node in the map")
                .next = Some(node.id);
        }
        self.last_node_id_added = Some(node.id);
        self.node_count += 1;
        node
    }

    pub fn insert_node_response(
        &mut self,
        node: &NodeBuilder<NodeIdType>,
        node_response: Option<NodeResponse>,
    ) {
        if let Some(NodeResponse {
            range: _,
            rects: Some(node_rects),
        }) = &node_response
        {
            self.push_child_node_position(
                node_rects
                    .closer
                    .or(node_rects.icon)
                    .unwrap_or(node_rects.label)
                    .left_center(),
            );
        }

        if node.is_dir {
            if let Some(node_response) = node_response {
                let anchor = node_response.range.center();
                self.indents.push(IndentState {
                    source_node: node.id,
                    anchor,
                    positions: Vec::new(),
                });
            }
            self.stack.push(DirectoryState {
                is_open: self.parent_dir_is_open() && node.is_open,
                id: node.id,
            });
        }
    }

    pub fn close_dir(&mut self) -> Option<(f32, Vec<Pos2>, usize)> {
        let closed_dir = self.stack.pop()?;
        let indent = self
            .indents
            .pop_if(|indent| indent.source_node == closed_dir.id)?;
        Some((indent.anchor, indent.positions, self.indents.len()))
    }

    fn push_child_node_position(&mut self, pos: Pos2) {
        if let Some(indent) = self.indents.last_mut() {
            indent.positions.push(pos);
        }
    }

    /// Get the current parent id if any.
    pub fn parent_id(&self) -> Option<NodeIdType> {
        self.parent_dir().map(|state| state.id)
    }
    fn parent_dir(&self) -> Option<&DirectoryState<NodeIdType>> {
        if self.stack.is_empty() {
            None
        } else {
            self.stack.last()
        }
    }
    pub fn parent_dir_is_open(&self) -> bool {
        self.parent_dir().is_none_or(|dir| dir.is_open)
    }
    pub fn get_indent(&self) -> usize {
        self.indents.len()
    }
}
