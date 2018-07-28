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

pub trait Forest<T> {
    type Ref;
    type ExternalRef;

    fn make_root(&mut self, item: T) -> Self::Ref;
    fn make_node(&mut self, parent_ref: Self::Ref, item: T) -> Self::Ref;

    fn access<'s, A>(&'s self, upper_layer_access: A, node_ref: Self::Ref) -> Option<(&'s T, Option<Self::Ref>)>
        where A: Fn(Self::ExternalRef) -> Option<(&'s T, Option<Self::ExternalRef>)>;
}

impl<T> Forest<T> for Forest1<T> {
    type Ref = Ref;
    type ExternalRef = ();

    fn make_root(&mut self, item: T) -> Self::Ref {
        self.nodes.insert(Node { item, parent: None, })
    }

    fn make_node(&mut self, parent_ref: Self::Ref, item: T) -> Self::Ref {
        self.nodes.insert(Node { item, parent: Some(parent_ref), })
    }

    fn access<'s, A>(&'s self, _upper_layer_access: A, node_ref: Self::Ref) -> Option<(&'s T, Option<Self::Ref>)>
        where A: Fn(Self::ExternalRef) -> Option<(&'s T, Option<Self::ExternalRef>)>
    {
        self.nodes.get(node_ref)
            .map(|node| (&node.item, node.parent))
    }
}

impl<T, F> Forest<T> for Forest2<T, F> where F: Forest<T>, F::Ref: Clone {
    type Ref = Ref2<F::Ref>;
    type ExternalRef = F::Ref;

    fn make_root(&mut self, item: T) -> Self::Ref {
        Ref2::Local(self.local_nodes.insert(Node { item, parent: None, }))
    }

    fn make_node(&mut self, parent_ref: Self::Ref, item: T) -> Self::Ref {
        Ref2::Local(self.local_nodes.insert(Node { item, parent: Some(parent_ref), }))
    }

    fn access<'s, A>(&'s self, upper_layer_access: A, node_ref: Self::Ref) -> Option<(&'s T, Option<Self::Ref>)>
        where A: Fn(Self::ExternalRef) -> Option<(&'s T, Option<Self::ExternalRef>)>
    {
        match node_ref {
            Ref2::Local(local_node_ref) =>
                self.local_nodes.get(local_node_ref).map(|node| (&node.item, node.parent.clone())),
            Ref2::External(external_node_ref) =>
                upper_layer_access(external_node_ref)
                    .map(|(value, parent)| (value, parent.map(Ref2::External))),
        }
    }
}

pub fn get1<'a, T>(node_ref: Ref, forest: &'a Forest1<T>) -> Option<&'a T> {
    forest.access(|()| unreachable!(), node_ref).map(|p| p.0)
}

pub fn get2<'a, T>(node_ref: Ref2<Ref>, forest2: &'a Forest2<T, Forest1<T>>, forest1: &'a Forest1<T>) -> Option<&'a T> {
    forest2.access(|node_ref| forest1.access(|()| unreachable!(), node_ref), node_ref).map(|p| p.0)
}

pub fn get3<'a, T>(
    node_ref: Ref2<Ref2<Ref>>,
    forest3: &'a Forest2<T, Forest2<T, Forest1<T>>>,
    forest2: &'a Forest2<T, Forest1<T>>,
    forest1: &'a Forest1<T>,
)
    -> Option<&'a T>
{
    forest3.access(
        |node_ref| forest2.access(|node_ref| forest1.access(|()| unreachable!(), node_ref), node_ref),
        node_ref,
    ).map(|p| p.0)
}

pub fn get4<'a, T>(
    node_ref: Ref2<Ref2<Ref2<Ref>>>,
    forest4: &'a Forest2<T, Forest2<T, Forest2<T, Forest1<T>>>>,
    forest3: &'a Forest2<T, Forest2<T, Forest1<T>>>,
    forest2: &'a Forest2<T, Forest1<T>>,
    forest1: &'a Forest1<T>,
)
    -> Option<&'a T>
{
    forest4.access(
        |node_ref| forest3.access(
            |node_ref| forest2.access(|node_ref| forest1.access(|()| unreachable!(), node_ref), node_ref),
            node_ref,
         ),
        node_ref,
    ).map(|p| p.0)
}
