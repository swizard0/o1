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

    pub fn external(&self, node_ref: R) -> Ref2<R> {
        Ref2::External(node_ref)
    }
}

pub trait Forest<T> {
    type Ref;
    type ExternalRef;

    fn make_root(&mut self, item: T) -> Self::Ref {
        self.insert(Node { item, parent: None, depth: 0, })
    }

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NonexistentRef;

impl<T> Forest<T> for Forest1<T> {
    type Ref = Ref;
    type ExternalRef = NonexistentRef;

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

pub mod layer_access {
    use super::{Node, Forest, NonexistentRef};

    pub fn nil<'a, T>() -> impl Fn(NonexistentRef) -> Option<Node<&'a T, NonexistentRef>> {
        move |_| unreachable!()
    }

    pub fn cons_get<'s, 'a: 's, T, R, F, A>(
        forest: &'s F,
        upper_layer_access: A,
    )
        -> impl Fn(R) -> Option<Node<&'s T, R>>
    where T: 'a,
          F: Forest<T, Ref = R>,
          A: Fn(F::ExternalRef) -> Option<Node<&'a T, F::ExternalRef>>,
    {
        move |node_ref| forest.get(&upper_layer_access, node_ref)
    }

    // pub fn cons_mut<'s, 'a: 's, T, R, F, A>(
    //     forest: &'s mut F,
    //     upper_layer_access: A,
    // )
    //     -> impl FnMut(R) -> Option<Node<&'s mut T, R>>
    // where T: 'a,
    //       F: Forest<T, Ref = R>,
    //       A: FnMut(F::ExternalRef) -> Option<Node<&'a mut T, F::ExternalRef>>,
    // {
    //     move |node_ref| forest.get_mut(upper_layer_access, node_ref)
    // }
}

// layers! { [forest2, forest1].get(node_ref) }
#[macro_export]
macro_rules! layers {
    { [$f:expr $(, $fs:expr),*] .get($ref:expr) } => {
        layers!(@rec layer_access::cons_get(&$f, layer_access::nil()), [$($fs)*].get($ref))
    };
    { @rec $a:expr, [].get($ref:expr) } => {
        ($a)($ref)
    };
    { @rec $a:expr, [$f:expr $(, $fs:expr),*].get($ref:expr) } => {
        layers!(@rec layer_access::cons_get(&$f, $a), [$($fs)*].get($ref))
    };

    { [$f:expr $(, $fs:expr),*].make_node($parent_ref:expr, $item:expr) } => {
        layers!(@rec $f, layer_access::nil(), [$($fs)*].make_node($parent_ref, $item))
    };
    { @rec $fe:expr, $a:expr, [].make_node($parent_ref:expr, $item:expr) } => {
        $fe.make_node($a, $parent_ref, $item)
    };
    { @rec $fe:expr, $a:expr, [$f:expr $(, $fs:expr),*].make_node($parent_ref:expr, $item:expr) } => {
        layers!(@rec $f, layer_access::cons_get(&$fe, $a), [$($fs)*].make_node($parent_ref, $item))
    };
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
        Forest2,
        layer_access,
    };

    #[test]
    fn layer_access() {
        let mut forest1 = Forest1::new();

        let root = forest1.make_root("root");
        assert_eq!(forest1.get(layer_access::nil(), root).map(|node| node.item), Some(&"root"));
        assert_eq!(forest1.get(layer_access::nil(), root).map(|node| node.parent), Some(None));
        assert_eq!(forest1.get(layer_access::nil(), root).map(|node| node.depth), Some(0));

        let child_a = forest1.make_node(layer_access::nil(), root, "child_a");
        assert_eq!(forest1.get(layer_access::nil(), child_a).map(|node| node.item), Some(&"child_a"));
        assert_eq!(forest1.get(layer_access::nil(), child_a).map(|node| node.parent), Some(Some(root)));
        assert_eq!(forest1.get(layer_access::nil(), child_a).map(|node| node.depth), Some(1));

        let child_b = forest1.make_node(layer_access::nil(), root, "child_b");
        assert_eq!(forest1.get(layer_access::nil(), child_b).map(|node| node.item), Some(&"child_b"));
        assert_eq!(forest1.get(layer_access::nil(), child_b).map(|node| node.parent), Some(Some(root)));
        assert_eq!(forest1.get(layer_access::nil(), child_b).map(|node| node.depth), Some(1));

        let child_c = forest1.make_node(layer_access::nil(), child_a, "child_c");
        assert_eq!(forest1.get(layer_access::nil(), child_c).map(|node| node.item), Some(&"child_c"));
        assert_eq!(forest1.get(layer_access::nil(), child_c).map(|node| node.parent), Some(Some(child_a)));
        assert_eq!(forest1.get(layer_access::nil(), child_c).map(|node| node.depth), Some(2));

        let mut forest2 = Forest2::new();

        let root2 = forest2.make_root("root2");
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), root2).map(|node| node.item), Some(&"root2"));
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), root2).map(|node| node.parent), Some(None));
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), root2).map(|node| node.depth), Some(0));

        let child_d = forest2.make_node(layer_access::cons_get(&forest1, layer_access::nil()), root2, "child_d");
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_d).map(|node| node.item), Some(&"child_d"));
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_d).map(|node| node.parent), Some(Some(root2)));
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_d).map(|node| node.depth), Some(1));

        let child_c_ext = forest2.external(child_c);
        let child_e = forest2.make_node(layer_access::cons_get(&forest1, layer_access::nil()), child_c_ext, "child_e");
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_e).map(|node| node.item), Some(&"child_e"));
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_e).map(|node| node.parent), Some(Some(child_c_ext)));
        assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_e).map(|node| node.depth), Some(3));
    }

    #[test]
    fn layer_access_macro() {
        let mut forest1 = Forest1::new();
        let mut forest2 = Forest2::new();

        let root = forest1.make_root("root");
        assert_eq!(layers!([forest1].get(root)).map(|node| node.item), Some(&"root"));

        let root2 = forest2.make_root("root2");
        assert_eq!(layers!([forest1, forest2].get(root2)).map(|node| node.item), Some(&"root2"));

        let root_ext = forest2.external(root);
        let child = layers!([forest1, forest2].make_node(root_ext, "child"));
        assert_eq!(layers!([forest1, forest2].get(child)).map(|node| node.item), Some(&"child"));


        // assert_eq!(forest1.get(layer_access::nil(), root).map(|node| node.parent), Some(None));
        // assert_eq!(forest1.get(layer_access::nil(), root).map(|node| node.depth), Some(0));

        // let child_a = forest1.make_node(layer_access::nil(), root, "child_a");
        // assert_eq!(forest1.get(layer_access::nil(), child_a).map(|node| node.item), Some(&"child_a"));
        // assert_eq!(forest1.get(layer_access::nil(), child_a).map(|node| node.parent), Some(Some(root)));
        // assert_eq!(forest1.get(layer_access::nil(), child_a).map(|node| node.depth), Some(1));

        // let child_b = forest1.make_node(layer_access::nil(), root, "child_b");
        // assert_eq!(forest1.get(layer_access::nil(), child_b).map(|node| node.item), Some(&"child_b"));
        // assert_eq!(forest1.get(layer_access::nil(), child_b).map(|node| node.parent), Some(Some(root)));
        // assert_eq!(forest1.get(layer_access::nil(), child_b).map(|node| node.depth), Some(1));

        // let child_c = forest1.make_node(layer_access::nil(), child_a, "child_c");
        // assert_eq!(forest1.get(layer_access::nil(), child_c).map(|node| node.item), Some(&"child_c"));
        // assert_eq!(forest1.get(layer_access::nil(), child_c).map(|node| node.parent), Some(Some(child_a)));
        // assert_eq!(forest1.get(layer_access::nil(), child_c).map(|node| node.depth), Some(2));

        // let mut forest2 = Forest2::new();

        // let root2 = forest2.make_root("root2");
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), root2).map(|node| node.item), Some(&"root2"));
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), root2).map(|node| node.parent), Some(None));
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), root2).map(|node| node.depth), Some(0));

        // let child_d = forest2.make_node(layer_access::cons_get(&forest1, layer_access::nil()), root2, "child_d");
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_d).map(|node| node.item), Some(&"child_d"));
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_d).map(|node| node.parent), Some(Some(root2)));
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_d).map(|node| node.depth), Some(1));

        // let child_c_ext = forest2.external(child_c);
        // let child_e = forest2.make_node(layer_access::cons_get(&forest1, layer_access::nil()), child_c_ext, "child_e");
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_e).map(|node| node.item), Some(&"child_e"));
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_e).map(|node| node.parent), Some(Some(child_c_ext)));
        // assert_eq!(forest2.get(layer_access::cons_get(&forest1, layer_access::nil()), child_e).map(|node| node.depth), Some(3));
    }
}
