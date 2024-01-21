#![allow(unused)]

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
}
pub struct File {
    pub id: Uuid,
    pub name: String,
}

impl Node {
    pub fn dir(name: &'static str, children: Vec<Node>) -> Self {
        Node::Directory(Directory {
            id: Uuid::new_v4(),
            name: String::from(name),
            children,
        })
    }
    pub fn file(name: &'static str) -> Self {
        Node::File(File {
            id: Uuid::new_v4(),
            name: String::from(name),
        })
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
                    Node::dir("baz", vec![Node::file("Benjamin"), Node::file("Charlotte")]),
                ],
            ),
            Node::file("Daniel"),
            Node::file("Emma"),
            Node::dir("bar", vec![Node::file("Finn"), Node::file("Grayson")]),
        ],
    )
}
