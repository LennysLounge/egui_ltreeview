use std::{any::Any, ops::ControlFlow};

use egui_ltreeview::{DropPosition, TreeViewBuilder};
use uuid::Uuid;

use crate::data::{
    Directory, File, NodeVisitor, NodeVisitorMut, TreeNode, Visitable, VisitableNode,
};

pub struct PrintTreeListing {
    pub depth: usize,
}
impl NodeVisitor for PrintTreeListing {
    fn visit_dir(&mut self, dir: &Directory) -> ControlFlow<()> {
        println!(
            "{:>depth$} {}\t{}",
            "",
            dir.name,
            dir.id,
            depth = self.depth
        );
        self.depth += 4;
        ControlFlow::Continue(())
    }

    fn leave_dir(&mut self, _dir: &Directory) -> ControlFlow<()> {
        self.depth -= 4;
        ControlFlow::Continue(())
    }

    fn visit_file(&mut self, file: &File) -> ControlFlow<()> {
        println!(
            "{:>depth$} {}\t{}",
            "",
            file.name,
            file.id,
            depth = self.depth
        );
        ControlFlow::Continue(())
    }
}

pub struct TreeViewVisitor<'a> {
    pub builder: TreeViewBuilder<'a, Uuid>,
}
impl NodeVisitor for TreeViewVisitor<'_> {
    fn visit_dir(&mut self, dir: &Directory) -> ControlFlow<()> {
        let res = self.builder.dir(&dir.id, |ui| {
            ui.label(&dir.name);
        });
        if let Some(res) = res {
            res.context_menu(|ui| {
                ui.label("Contex menu of a dir");
            });
        }
        ControlFlow::Continue(())
    }

    fn leave_dir(&mut self, _dir: &Directory) -> ControlFlow<()> {
        self.builder.close_dir();
        ControlFlow::Continue(())
    }

    fn visit_file(&mut self, file: &File) -> ControlFlow<()> {
        let res = self.builder.leaf(&file.id, |ui| {
            ui.label(&file.name);
        });
        if let Some(res) = res {
            res.context_menu(|ui| {
                ui.label("Contex menu of a leaf");
            });
        }
        ControlFlow::Continue(())
    }
}

pub struct RemoveNodeVisitor {
    pub id: Uuid,
    pub removed_node: Option<TreeNode>,
}
impl RemoveNodeVisitor {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            removed_node: None,
        }
    }
    pub fn remove_from<V: Visitable>(mut self, visitable: &mut V) -> Option<TreeNode> {
        visitable.walk_mut(&mut self);
        self.removed_node
    }
}
impl NodeVisitorMut for RemoveNodeVisitor {
    fn visit_dir(&mut self, dir: &mut Directory) -> ControlFlow<()> {
        if let Some(index) = dir.nodes.iter().position(|n| &self.id == n.id()) {
            self.removed_node = Some(dir.nodes.remove(index));
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

pub struct InsertNodeVisitor {
    pub target_id: Uuid,
    pub position: DropPosition<Uuid>,
    // Option so we can leave an empty spot without moving any part of the parent struct.
    pub node: Option<TreeNode>,
}
impl NodeVisitorMut for InsertNodeVisitor {
    fn visit_dir(&mut self, dir: &mut Directory) -> ControlFlow<()> {
        if dir.id == self.target_id {
            let node = self.node.take().expect("Node should not be empty");
            match self.position {
                DropPosition::First => dir.nodes.insert(0, node),
                DropPosition::Last => dir.nodes.push(node),
                DropPosition::After(id) => {
                    if let Some(index) = dir.nodes.iter().position(|n| n.id() == &id) {
                        dir.nodes.insert(index + 1, node);
                    }
                }
                DropPosition::Before(id) => {
                    if let Some(index) = dir.nodes.iter().position(|n| n.id() == &id) {
                        dir.nodes.insert(index, node);
                    }
                }
            }
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

pub struct DropAllowedVisitor<'a> {
    pub drag_node: &'a dyn Any,
    pub drop_allowed: bool,
}
impl<'a> DropAllowedVisitor<'a> {
    pub fn new(drag_node: &'a dyn Any) -> Self {
        Self {
            drag_node,
            drop_allowed: false,
        }
    }
    pub fn test<V>(mut self, node: &V) -> bool
    where
        V: Visitable + ?Sized,
    {
        node.enter(&mut self);
        self.drop_allowed
    }
}
impl<'a> NodeVisitor for DropAllowedVisitor<'a> {
    fn visit_dir(&mut self, dir: &Directory) -> ControlFlow<()> {
        if let Some(dropped) = self.drag_node.downcast_ref::<Directory>() {
            if dir.a_allowed {
                self.drop_allowed = true;
            } else {
                self.drop_allowed = !dropped.name.to_lowercase().contains("a");
            }
        }
        if let Some(dropped) = self.drag_node.downcast_ref::<File>() {
            if dir.a_allowed {
                self.drop_allowed = true;
            } else {
                self.drop_allowed = !dropped.name.to_lowercase().contains("a");
            }
        }
        ControlFlow::Break(())
    }

    fn visit_file(&mut self, _file: &File) -> ControlFlow<()> {
        self.drop_allowed = false;
        ControlFlow::Break(())
    }
}

pub struct SearchVisitor<'a, T> {
    id: Uuid,
    action: Box<dyn FnMut(&dyn VisitableNode) -> T + 'a>,
    pub output: Option<T>,
}
impl<'a, T> SearchVisitor<'a, T> {
    pub fn new(id: Uuid, action: impl FnMut(&dyn VisitableNode) -> T + 'a) -> Self {
        Self {
            id,
            action: Box::new(action),
            output: None,
        }
    }
    pub fn search_in<V>(mut self, node: &V) -> Option<T>
    where
        V: Visitable,
    {
        node.walk(&mut self);
        self.output
    }
}
impl<'a, T> NodeVisitor for SearchVisitor<'a, T> {
    fn leave_dir(&mut self, dir: &Directory) -> ControlFlow<()> {
        if dir.id != self.id {
            return ControlFlow::Continue(());
        }
        self.output = Some((self.action)(dir));
        ControlFlow::Break(())
    }
    fn visit_file(&mut self, file: &File) -> ControlFlow<()> {
        if file.id != self.id {
            return ControlFlow::Continue(());
        }
        self.output = Some((self.action)(file));
        ControlFlow::Break(())
    }
}
