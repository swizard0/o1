use super::set::{Set, Ref, ItemsTransformer};

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

    pub fn merge_aflat(self, target: &mut Forest1<T>) {
        struct Transformer;
        impl<T> ItemsTransformer<Node<T, Ref>, Node<T, Ref>> for Transformer {
            fn transform<RF>(&mut self, _ref: Ref, node: Node<T, Ref>, ref_transform: RF) -> Node<T, Ref> where RF: Fn(Ref) -> Option<Ref> {
                Node { parent: node.parent.and_then(ref_transform), ..node }
            }
        }
        target.nodes.consume(self.nodes, Transformer);
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

impl<T, R> Forest2<T, Ref2<R>> {
    pub fn merge_aflat(self, target: &mut Forest2<T, Ref2<R>>) {
        struct Transformer;
        impl<T, R> ItemsTransformer<Node<T, Ref2<R>>, Node<T, Ref2<R>>> for Transformer {
            fn transform<RF>(
                &mut self,
                _ref: Ref,
                node: Node<T, Ref2<R>>,
                ref_transform: RF,
            )
                -> Node<T, Ref2<R>>
            where RF: Fn(Ref) -> Option<Ref>
            {
                let parent = match node.parent {
                    Some(Ref2::Local(local_ref)) =>
                        ref_transform(local_ref).map(Ref2::Local),
                    Some(Ref2::External(external_ref)) =>
                        Some(Ref2::External(external_ref)),
                    None =>
                        None,
                };
                Node { parent, ..node }
            }
        }
        target.local_nodes.consume(self.local_nodes, Transformer);
    }
}

impl<T> Forest2<T, Ref> {
    pub fn merge_down(self, target: &mut Forest1<T>) {
        struct Transformer;
        impl<T> ItemsTransformer<Node<T, Ref>, Node<T, Ref2<Ref>>> for Transformer {
            fn transform<RF>(
                &mut self,
                _ref: Ref,
                node: Node<T, Ref2<Ref>>,
                ref_transform: RF,
            )
                -> Node<T, Ref>
            where RF: Fn(Ref) -> Option<Ref>
            {
                let parent = match node.parent {
                    Some(Ref2::Local(local_ref)) =>
                        ref_transform(local_ref),
                    Some(Ref2::External(external_ref)) =>
                        Some(external_ref),
                    None =>
                        None,
                };
                Node { parent, item: node.item, depth: node.depth, }
            }
        }
        target.nodes.consume(self.local_nodes, Transformer);
    }
}

impl<T, R> Forest2<T, Ref2<Ref2<R>>> {
    pub fn merge_down(self, target: &mut Forest2<T, Ref2<R>>) {
        struct Transformer;
        impl<T, R> ItemsTransformer<Node<T, Ref2<R>>, Node<T, Ref2<Ref2<R>>>> for Transformer {
            fn transform<RF>(
                &mut self,
                _ref: Ref,
                node: Node<T, Ref2<Ref2<R>>>,
                ref_transform: RF,
            )
                -> Node<T, Ref2<R>>
            where RF: Fn(Ref) -> Option<Ref>
            {
                let parent = match node.parent {
                    Some(Ref2::Local(local_ref)) =>
                        ref_transform(local_ref).map(Ref2::Local),
                    Some(Ref2::External(external_ref)) =>
                        Some(external_ref),
                    None =>
                        None,
                };
                Node { parent, item: node.item, depth: node.depth, }
            }
        }
        target.local_nodes.consume(self.local_nodes, Transformer);
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
        where T: 'a, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>;

    fn get_mut<'s, 'a: 's, A>(&'s mut self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s mut T, Self::Ref>>
        where T: 'a, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a mut T, Self::ExternalRef>>;

    fn remove<A>(&mut self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<T, Self::Ref>>
        where A: FnOnce(Self::ExternalRef) -> Option<Node<T, Self::ExternalRef>>;

    fn make_node<'s, 'a: 's, A>(&'s mut self, upper_layer_access: A, parent_ref: Self::Ref, item: T) -> Self::Ref
        where T: 'a, Self::Ref: Clone, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>
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
        where T: 'a, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>
    {
        self.nodes.get(node_ref)
            .map(|node| Node { item: &node.item, parent: node.parent, depth: node.depth, })
    }

    fn get_mut<'s, 'a: 's, A>(&'s mut self, _upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s mut T, Self::Ref>>
        where T: 'a, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a mut T, Self::ExternalRef>>
    {
        self.nodes.get_mut(node_ref)
            .map(|node| Node { item: &mut node.item, parent: node.parent, depth: node.depth, })
    }

    fn remove<A>(&mut self, _upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<T, Self::Ref>>
        where A: FnOnce(Self::ExternalRef) -> Option<Node<T, Self::ExternalRef>>
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
        where T: 'a, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a T, Self::ExternalRef>>
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

    fn get_mut<'s, 'a: 's, A>(&'s mut self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<&'s mut T, Self::Ref>>
        where T: 'a, A: FnOnce(Self::ExternalRef) -> Option<Node<&'a mut T, Self::ExternalRef>>
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

    fn remove<A>(&mut self, upper_layer_access: A, node_ref: Self::Ref) -> Option<Node<T, Self::Ref>>
        where A: FnOnce(Self::ExternalRef) -> Option<Node<T, Self::ExternalRef>>
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

    pub fn nil_mut<'a, T>() -> impl FnOnce(NonexistentRef) -> Option<Node<&'a mut T, NonexistentRef>> {
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

    pub fn cons_get_mut<'s, 'a: 's, T, R, F, A>(
        forest: &'s mut F,
        upper_layer_access: A,
    )
        -> impl FnOnce(R) -> Option<Node<&'s mut T, R>>
    where T: 'a,
          F: Forest<T, Ref = R>,
          A: FnOnce(F::ExternalRef) -> Option<Node<&'a mut T, F::ExternalRef>>,
    {
        move |node_ref| forest.get_mut(upper_layer_access, node_ref)
    }
}

// layers! { [forest2, forest1].get(node_ref) }
#[macro_export]
macro_rules! layers {
    { [$f:expr $(, $fs:expr),*].get($ref:expr) } => {
        layers!(@rec layer_access::cons_get($f, layer_access::nil()), [$($fs)*].get($ref))
    };
    { @rec $a:expr, [].get($ref:expr) } => {
        ($a)($ref)
    };
    { @rec $a:expr, [$f:expr $(, $fs:expr),*].get($ref:expr) } => {
        layers!(@rec layer_access::cons_get($f, $a), [$($fs)*].get($ref))
    };

    { [$f:expr $(, $fs:expr),*].get_mut($ref:expr) } => {
        layers!(@rec layer_access::cons_get_mut($f, layer_access::nil_mut()), [$($fs)*].get_mut($ref))
    };
    { @rec $a:expr, [].get_mut($ref:expr) } => {
        ($a)($ref)
    };
    { @rec $a:expr, [$f:expr $(, $fs:expr),*].get_mut($ref:expr) } => {
        layers!(@rec layer_access::cons_get_mut($f, $a), [$($fs)*].get_mut($ref))
    };

    { [$f:expr $(, $fs:expr),*].make_node($parent_ref:expr, $item:expr) } => {
        layers!(@rec $f, layer_access::nil(), [$($fs)*].make_node($parent_ref, $item))
    };
    { @rec $fe:expr, $a:expr, [].make_node($parent_ref:expr, $item:expr) } => {
        $fe.make_node($a, $parent_ref, $item)
    };
    { @rec $fe:expr, $a:expr, [$f:expr $(, $fs:expr),*].make_node($parent_ref:expr, $item:expr) } => {
        layers!(@rec $f, layer_access::cons_get($fe, $a), [$($fs)*].make_node($parent_ref, $item))
    };
}

pub struct TowardsRootIter<R, A> {
    cursor: Option<R>,
    layer_access: A,
}

impl<R, A> TowardsRootIter<R, A> {
    pub fn new(layer_access: A, node_ref: R) -> TowardsRootIter<R, A> {
        TowardsRootIter {
            cursor: Some(node_ref),
            layer_access,
        }
    }
}

impl<'a, T: 'a, R, A> Iterator for TowardsRootIter<R, A> where R: Clone, A: Fn(R) -> Option<Node<&'a T, R>> {
    type Item = Node<&'a T, R>;

    fn next(&mut self) -> Option<Self::Item> {
        let node_ref = self.cursor.take()?;
        let node = (self.layer_access)(node_ref.clone())?;
        self.cursor = node.parent.clone();
        Some(node)
    }
}


#[cfg(test)]
mod test {
    use super::{
        Forest,
        Forest1,
        Forest2,
        TowardsRootIter,
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

        let iter = TowardsRootIter::new(layer_access::cons_get(&forest2, layer_access::cons_get(&forest1, layer_access::nil())), child_e);
        assert_eq!(iter.map(|node| node.item).collect::<Vec<_>>(), vec![&"child_e", &"child_c", &"child_a", &"root"]);
    }

    #[test]
    fn layer_access_macro() {
        let mut forest1 = Forest1::new();
        let mut forest2 = Forest2::new();

        let root = forest1.make_root("root");
        assert_eq!(layers!([&forest1].get(root)).map(|node| node.item), Some(&"root"));

        let root2 = forest2.make_root("root2");
        assert_eq!(layers!([&forest1, &forest2].get(root2)).map(|node| node.item), Some(&"root2"));

        let root_ext = forest2.external(root);
        let child = layers!([&forest1, &mut forest2].make_node(root_ext, "child"));
        assert_eq!(layers!([&forest1, &forest2].get(child)).map(|node| node.item), Some(&"child"));

        layers!([&mut forest1, &mut forest2].get_mut(child)).map(|node| *node.item = "other child");
        assert_eq!(layers!([&forest1, &forest2].get(child)).map(|node| node.item), Some(&"other child"));
    }

    #[test]
    fn merge_forest1() {
        let mut forest1_a = Forest1::new();
        let root_a = forest1_a.make_root("root a");
        let child_a_a = layers!([&mut forest1_a].make_node(root_a, "child_a a"));
        let child_b_a = layers!([&mut forest1_a].make_node(root_a, "child_b a"));

        let mut forest1_b = Forest1::new();
        let root_b = forest1_b.make_root("root b");
        let child_a_b = layers!([&mut forest1_b].make_node(root_b, "child_a b"));
        let child_b_b = layers!([&mut forest1_b].make_node(child_a_b, "child_b b"));

    }
}
