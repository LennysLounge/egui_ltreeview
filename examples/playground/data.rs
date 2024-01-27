#![allow(unused)]

use std::any::Any;

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
}
pub struct File {
    pub id: Uuid,
    pub name: String,
    pub icon: bool,
}

impl Node {
    pub fn dir(name: &'static str, children: Vec<Node>) -> Self {
        Node::Directory(Directory {
            id: Uuid::new_v4(),
            name: String::from(name),
            children,
            custom_closer: true,
            icon: false,
        })
    }

    pub fn file(name: &'static str) -> Self {
        Node::File(File {
            id: Uuid::new_v4(),
            name: String::from(name),
            icon: true,
        })
    }

    pub fn id(&self) -> &Uuid {
        match self {
            Node::Directory(dir) => &dir.id,
            Node::File(file) => &file.id,
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
