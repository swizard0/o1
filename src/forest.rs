
use super::set::{Set, Ref};

pub struct Forest<T> {
    nodes: Set<Node<T>>,
}

impl<T> Forest<T> {
    pub fn new() -> Forest<T> {
        Forest {
            nodes: Set::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Forest<T> {
        Forest {
            nodes: Set::with_capacity(capacity),
        }
    }

    pub fn make_root(&mut self, item: T) -> Ref {
        self.nodes.insert(Node { item, parent: None, })
    }

    pub fn make_node(&mut self, parent_ref: Ref, item: T) -> Ref {
        self.nodes.insert(Node { item, parent: Some(parent_ref), })
    }

    pub fn remove(&mut self, node_ref: Ref) -> Option<T> {
        self.nodes.remove(node_ref)
            .map(|node| node.item)
    }

    pub fn get(&self, node_ref: Ref) -> Option<&T> {
        self.nodes.get(node_ref)
            .map(|node| &node.item)
    }

    pub fn get_mut(&mut self, node_ref: Ref) -> Option<&mut T> {
        self.nodes.get_mut(node_ref)
            .map(|node| &mut node.item)
    }

    pub fn parent(&self, node_ref: Ref) -> Option<Ref> {
        self.nodes.get(node_ref)
            .and_then(|node| node.parent)
    }
}

struct Node<T> {
    item: T,
    parent: Option<Ref>,
}
