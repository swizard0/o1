
use super::set::{Set, Ref};

pub struct Forest<T> {
    nodes: Set<Node<T>>,
}

struct Node<T> {
    item: T,
    parent: Option<Ref>,
}
