use super::set::{Set, Ref};

pub struct Forest1<T> {
    nodes: Set<Node<T, Ref>>,
}

impl<T> Forest1<T> {
    pub fn new() -> Forest1<T> {
        Forest1 {
            nodes: Set::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Forest1<T> {
        Forest1 {
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ref2<R> {
    Local(Ref),
    External(R),
}

pub struct Forest2<T, F> where F: Forest<T> {
    local_nodes: Set<Node<T, Ref2<F::Ref>>>,
}

impl<T, F> Forest2<T, F> where F: Forest<T> {
    pub fn new() -> Forest2<T, F> {
        Forest2 {
            local_nodes: Set::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Forest2<T, F> {
        Forest2 {
            local_nodes: Set::with_capacity(capacity),
        }
    }
}

struct Node<T, R> {
    item: T,
    parent: Option<R>,
}

pub struct Bottom;
pub struct Level<T, R> {
    top: T,
    rest: R,
}

pub trait LevelLayer: Sized {
    type T;
    type R;

    fn disasm(self) -> (Self::T, Self::R);
}

impl LevelLayer for Bottom {
    type T = Bottom;
    type R = Bottom;

    fn disasm(self) -> (Self::T, Self::R) {
        (Bottom, Bottom)
    }
}

impl<T, R> LevelLayer for Level<T, R> {
    type T = T;
    type R = R;

    fn disasm(self) -> (Self::T, Self::R) {
        (self.top, self.rest)
    }
}

pub trait Forest<T> {
    type Ref;
    type ExternalForest;

    fn make_root(&mut self, item: T) -> Self::Ref;
    fn make_node(&mut self, parent_ref: Self::Ref, item: T) -> Self::Ref;

    fn get<'s, 'f: 's, L>(&'s self, layer: L, node_ref: Self::Ref) -> Option<&'s T>
        where L: LevelLayer<T = &'f Self::ExternalForest>, L::R: LevelLayer, Self::ExternalForest: 'f;
}

impl<T> Forest<T> for Forest1<T> {
    type Ref = Ref;
    type ExternalForest = Bottom;

    fn make_root(&mut self, item: T) -> Self::Ref {
        self.nodes.insert(Node { item, parent: None, })
    }

    fn make_node(&mut self, parent_ref: Self::Ref, item: T) -> Self::Ref {
        self.nodes.insert(Node { item, parent: Some(parent_ref), })
    }

    fn get<'s, 'f: 's, L>(&'s self, _layer: L, node_ref: Self::Ref) -> Option<&'s T>
        where L: LevelLayer<T = &'f Self::ExternalForest>, L::R: LevelLayer, Self::ExternalForest: 'f
    {
        self.nodes.get(node_ref)
            .map(|node| &node.item)
    }
}

impl<T, F> Forest<T> for Forest2<T, F> where F: Forest<T> {
    type Ref = Ref2<F::Ref>;
    type ExternalForest = F;

    fn make_root(&mut self, item: T) -> Self::Ref {
        Ref2::Local(self.local_nodes.insert(Node { item, parent: None, }))
    }

    fn make_node(&mut self, parent_ref: Self::Ref, item: T) -> Self::Ref {
        Ref2::Local(self.local_nodes.insert(Node { item, parent: Some(parent_ref), }))
    }

    fn get<'s, 'f: 's, L>(&'s self, layer: L, node_ref: Self::Ref) -> Option<&'s T>
        where L: LevelLayer<T = &'f Self::ExternalForest>, L::R: LevelLayer, Self::ExternalForest: 'f
    {
        match node_ref {
            Ref2::Local(local_node_ref) =>
                self.local_nodes.get(local_node_ref).map(|node| &node.item),
            Ref2::External(external_node_ref) => {
                let (external_forest, next_layer) = layer.disasm();
                external_forest.get(next_layer, external_node_ref)
            },
        }
    }
}
