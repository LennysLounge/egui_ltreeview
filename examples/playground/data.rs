#![allow(unused)]

use std::any::Any;

use egui_ltreeview::DirPosition;
use uuid::Uuid;

fn main() {}

pub enum Node {
    Directory(Directory),
    File(File),
}
pub struct Directory {
    pub id: Uuid,
    pub name: String,
    pub children: Vec<Node>,
    pub custom_closer: bool,
    pub icon: bool,
    pub activatable: bool,
}
pub struct File {
    pub id: Uuid,
    pub name: String,
    pub icon: bool,
    pub activatable: bool,
}

impl Node {
    pub fn dir(name: &'static str, children: Vec<Node>) -> Self {
        Node::Directory(Directory {
            id: Uuid::new_v4(),
            name: String::from(name),
            children,
            custom_closer: true,
            icon: false,
            activatable: false,
        })
    }

    pub fn file(name: &'static str) -> Self {
        Node::File(File {
            id: Uuid::new_v4(),
            name: String::from(name),
            icon: true,
            activatable: true,
        })
    }

    pub fn id(&self) -> &Uuid {
        match self {
            Node::Directory(dir) => &dir.id,
            Node::File(file) => &file.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Node::Directory(directory) => &directory.name,
            Node::File(file) => &file.name,
        }
    }

    pub fn find_mut(&mut self, id: &Uuid, action: &mut dyn FnMut(&mut Node)) {
        if self.id() == id {
            (action)(self);
        } else {
            match self {
                Node::Directory(dir) => {
                    for node in dir.children.iter_mut() {
                        node.find_mut(id, action);
                    }
                }
                Node::File(_) => (),
            }
        }
    }

    pub fn remove(&mut self, id: &Uuid) -> Option<Node> {
        match self {
            Node::Directory(dir) => {
                if let Some(index) = dir.children.iter().position(|n| n.id() == id) {
                    Some(dir.children.remove(index))
                } else {
                    for node in dir.children.iter_mut() {
                        let r = node.remove(id);
                        if r.is_some() {
                            return r;
                        }
                    }
                    None
                }
            }
            Node::File(_) => None,
        }
    }

    pub fn insert(
        &mut self,
        id: &Uuid,
        position: DirPosition<Uuid>,
        value: Node,
    ) -> Result<(), Node> {
        match self {
            Node::Directory(dir) => {
                if dir.id == *id {
                    match position {
                        DirPosition::First => dir.children.insert(0, value),
                        DirPosition::Last => dir.children.push(value),
                        DirPosition::After(after_id) => {
                            if let Some(index) =
                                dir.children.iter().position(|n| *n.id() == after_id)
                            {
                                dir.children.insert(index + 1, value);
                            }
                        }
                        DirPosition::Before(before_id) => {
                            if let Some(index) =
                                dir.children.iter().position(|n| *n.id() == before_id)
                            {
                                dir.children.insert(index, value);
                            }
                        }
                    }
                    Ok(())
                } else {
                    let mut value = Err(value);
                    for node in dir.children.iter_mut() {
                        if let Err(v) = value {
                            value = node.insert(id, position, v);
                        }
                    }
                    value
                }
            }
            _ => Err(value),
        }
    }
}
pub fn make_tree() -> Node {
    Node::dir(
        "Root",
        vec![
            Node::dir(
                "Foo",
                vec![
                    Node::file("Ava"),
                    Node::dir("bar", vec![Node::file("Benjamin"), Node::file("Charlotte")]),
                ],
            ),
            Node::file("Daniel"),
            Node::file("Emma"),
            Node::dir("bar", vec![Node::file("Finn"), Node::file("Grayson")]),
        ],
    )
}
