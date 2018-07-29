use super::set::{Set, Ref};

pub struct Node<T, R> {
    pub item: T,
    pub parent: Option<R>,
    pub depth: usize,
}

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

    fn make_root(&mut self, item: T) -> Ref {
        self.insert(Node { item, parent: None, depth: 0, })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ref2<R> {
    Local(Ref),
    External(R),
}

pub struct Forest2<T, R> {
    local_nodes: Set<Node<T, Ref2<R>>>,
}

impl<T, R> Forest2<T, R> {
    pub fn new() -> Forest2<T, R> {
        Forest2 {
            local_nodes: Set::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Forest2<T, R> {
        Forest2 {
            local_nodes: Set::with_capacity(capacity),
        }
    }

    fn make_root(&mut self, item: T) -> Ref2<R> {
        Ref2::Local(self.local_nodes.insert(Node { item, parent: None, depth: 0, }))
    }
}

trait Forest<T> {
    type Ref;
    type ExternalRef;

    fn insert(&mut self, node: Node<T, Self::Ref>) -> Self::Ref;

    fn get<'s, 'a: 's, A>(&'s self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s T, Self::Ref>>
        where T: 'a, A: Fn(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>;

    fn get_mut<'s, 'a: 's, A>(&'s mut self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s mut T, Self::Ref>>
        where T: 'a, A: FnMut(Self::ExternalRef) -> Option<Node<&'a mut T, Self::ExternalRef>>;

    fn remove<A>(&mut self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<T, Self::Ref>>
        where A: FnMut(Self::ExternalRef) -> Option<Node<T, Self::ExternalRef>>;

    fn make_node<'s, 'a: 's, A>(&'s mut self, upper_layer_access: A, parent_ref: Self::Ref, item: T) -> Self::Ref
        where T: 'a, Self::Ref: Clone, A: Fn(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>
    {
        if let Some(parent_depth) = self.get(upper_layer_access, parent_ref.clone()).map(|node| node.depth) {
            self.insert(Node { item, parent: Some(parent_ref), depth: parent_depth + 1, })
        } else {
            self.insert(Node { item, parent: None, depth: 0, })
        }
    }
}

impl<T> Forest<T> for Forest1<T> {
    type Ref = Ref;
    type ExternalRef = ();

    fn insert(&mut self, node: Node<T, Self::Ref>) -> Self::Ref {
        self.nodes.insert(node)
    }

    fn get<'s, 'a: 's, A>(&'s self, _upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s T, Self::Ref>>
        where T: 'a, A: Fn(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>
    {
        self.nodes.get(node_ref)
            .map(|node| Node { item: &node.item, parent: node.parent, depth: node.depth, })
    }

    fn get_mut<'s, 'a: 's, A>(&'s mut self, _upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s mut T, Self::Ref>>
        where T: 'a, A: FnMut(Self::ExternalRef) -> Option<Node<&'a mut T, Self::ExternalRef>>
    {
        self.nodes.get_mut(node_ref)
            .map(|node| Node { item: &mut node.item, parent: node.parent, depth: node.depth, })
    }

    fn remove<A>(&mut self, _upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<T, Self::Ref>>
        where A: FnMut(Self::ExternalRef) -> Option<Node<T, Self::ExternalRef>>
    {
        self.nodes.remove(node_ref)
    }
}

impl<T, R> Forest<T> for Forest2<T, R> where R: Clone {
    type Ref = Ref2<R>;
    type ExternalRef = R;

    fn insert(&mut self, node: Node<T, Self::Ref>) -> Self::Ref {
        Ref2::Local(self.local_nodes.insert(node))
    }

    fn get<'s, 'a: 's, A>(&'s self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s T, Self::Ref>>
        where T: 'a, A: Fn(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>
    {
        match node_ref {
            Ref2::Local(local_node_ref) => {
                self.local_nodes.get(local_node_ref)
                    .map(|node| Node {
                        item: &node.item,
                        parent: node.parent.clone(),
                        depth: node.depth,
                    })
            },
            Ref2::External(external_node_ref) => {
                upper_layer_access(external_node_ref)
                    .map(|node| Node {
                        item: node.item,
                        parent: node.parent.map(Ref2::External),
                        depth: node.depth,
                    })
            },
        }
    }

    fn get_mut<'s, 'a: 's, A>(&'s mut self, mut upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s mut T, Self::Ref>>
        where T: 'a, A: FnMut(Self::ExternalRef) -> Option<Node<&'a mut T, Self::ExternalRef>>
    {
        match node_ref {
            Ref2::Local(local_node_ref) => {
                self.local_nodes.get_mut(local_node_ref)
                    .map(|node| Node {
                        item: &mut node.item,
                        parent: node.parent.clone(),
                        depth: node.depth,
                    })
            },
            Ref2::External(external_node_ref) => {
                upper_layer_access(external_node_ref)
                    .map(|node| Node {
                        item: node.item,
                        parent: node.parent.map(Ref2::External),
                        depth: node.depth,
                    })
            },
        }
    }

    fn remove<A>(&mut self, mut upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<T, Self::Ref>>
        where A: FnMut(Self::ExternalRef) -> Option<Node<T, Self::ExternalRef>>
    {
        match node_ref {
            Ref2::Local(local_node_ref) =>
                self.local_nodes.remove(local_node_ref),
            Ref2::External(external_node_ref) => {
                upper_layer_access(external_node_ref)
                    .map(|node| Node {
                        item: node.item,
                        parent: node.parent.map(Ref2::External),
                        depth: node.depth,
                    })
            },
        }
    }
}

pub fn nil<'a, T, R>() -> impl Fn(R) -> Option<(&'a T, Option<R>)> {
    move |_| unreachable!()
}

pub fn car_get<'s, 'a: 's, T, F, A>(
    forest: &'s F,
    upper_layer_access: A,
)
    -> impl Fn(F::Ref) -> Option<Node<&'s T, F::Ref>>
    where T: 'a,
          F: Forest<T>,
          A: Fn(F::ExternalRef) -> Option<Node<&'a T, F::ExternalRef>>,
{
    move |node_ref| forest.get(&upper_layer_access, node_ref)
}


// pub fn access1<'a, T, R>() -> impl Fn(R) -> Option<(&'a T, Option<R>)> {
//     move |_| unreachable!()
// }

// pub fn access2<'a, T: 'a, F, A>(
//     forest: &'a F,
//     upper_layer_access: A,
// )
//     -> impl Fn(F::Ref) -> Option<(&'a T, Option<F::Ref>)>
//     where F: Forest<T>,
//           A: Fn(F::ExternalRef) -> Option<(&'a T, Option<F::ExternalRef>)>,
// {
//     move |node_ref| forest.access(&upper_layer_access, node_ref)
// }

// pub struct TowardsRootIter<R, A> {
//     cursor: Option<R>,
//     layer_access: A,
// }

// pub fn towards_root_iter<'a, T: 'a, R, A, F>(
//     forest1: &'a F,
//     upper_layer_access: A,
//     node_ref: R,
// )
//     -> TowardsRootIter<R, impl Fn(R) -> Option<(&'a T, Option<R>)>>
//     where F: Forest<T, Ref = R>,
//           A: Fn(F::ExternalRef) -> Option<(&'a T, Option<F::ExternalRef>)>,
// {
//     TowardsRootIter {
//         cursor: Some(node_ref),
//         layer_access: move |node_ref| forest1.access(&upper_layer_access, node_ref),
//     }
// }

// impl<'a, T: 'a, R, A> Iterator for TowardsRootIter<R, A> where R: Clone, A: Fn(R) -> Option<(&'a T, Option<R>)> {
//     type Item = (R, &'a T);

//     fn next(&mut self) -> Option<Self::Item> {
//         let node_ref = self.cursor.take()?;
//         let (item, parent) = (self.layer_access)(node_ref.clone())?;
//         self.cursor = parent;
//         Some((node_ref, item))
//     }
// }


#[cfg(test)]
mod test {
    use super::{
        Forest,
        Forest1,
        access1,
        towards_root_iter,
    };

    #[test]
    fn forest1() {
        let mut forest1 = Forest1::new();
        let root = forest1.make_root("root");
        let child_a = forest1.make_node(root, "child_a");
        assert_eq!(forest1.parent(access1(), child_a), Some(root));
        let child_b = forest1.make_node(root, "child_b");
        let child_c = forest1.make_node(child_a, "child_c");

        assert_eq!(forest1.get(access1(), root), Some(&"root"));
        assert_eq!(forest1.get(access1(), child_a), Some(&"child_a"));
        assert_eq!(forest1.get(access1(), child_b), Some(&"child_b"));
        assert_eq!(forest1.get(access1(), child_c), Some(&"child_c"));

        let rev_path: Vec<_> = towards_root_iter(&forest1, access1(), child_c).map(|p| p.1).collect();
        assert_eq!(rev_path, vec![&"child_c", &"child_a", &"root"]);
    }
}
