use super::{
    set::{
        Set,
        Ref,
        SetsInitMerger,
        SetsInProgressMerger,
    },
    merge::{
        MergeState,
        InitMerger,
        InProgressMerger,
    },
};

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

    pub fn merge_aflat(self, target: Forest1<T>) -> Forest1InitMerger<T> {
        Forest1InitMerger(target.nodes.merge(self.nodes))
    }

    pub fn local_iter(&self) -> impl Iterator<Item = (Ref, &T)> {
        self.nodes.iter().map(|(set_ref, node)| (set_ref, &node.item))
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

    pub fn local_iter(&self) -> impl Iterator<Item = (Ref2<R>, &T)> {
        self.local_nodes.iter()
            .map(|(set_ref, node)| (Ref2::Local(set_ref), &node.item))
    }
}

impl<T, R> Forest2<T, R> {
    pub fn merge_aflat(self, target: Forest2<T, R>) -> Forest2AflatInitMerger<T, R> {
        Forest2AflatInitMerger(target.local_nodes.merge(self.local_nodes))
    }
}

impl<T> Forest2<T, Ref> {
    pub fn merge_down(self, target: Forest1<T>) -> Forest2Down1InitMerger<T> {
        Forest2Down1InitMerger(target.nodes.merge(self.local_nodes))
    }
}

impl<T, R> Forest2<T, Ref2<R>> {
    pub fn merge_down(self, target: Forest2<T, R>) -> Forest2Down2InitMerger<T, R> {
        Forest2Down2InitMerger(target.local_nodes.merge(self.local_nodes))
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

#[macro_export]
macro_rules! layers {
    { [$f:expr $(, $fs:expr),*].get($ref:expr) } => {
        layers!(@rec ::forest::layer_access::cons_get($f, ::forest::layer_access::nil()), [$($fs)*].get($ref))
    };
    { @rec $a:expr, [].get($ref:expr) } => {
        ($a)($ref)
    };
    { @rec $a:expr, [$f:expr $(, $fs:expr),*].get($ref:expr) } => {
        layers!(@rec ::forest::layer_access::cons_get($f, $a), [$($fs)*].get($ref))
    };

    { [$f:expr $(, $fs:expr),*].get_mut($ref:expr) } => {
        layers!(@rec ::forest::layer_access::cons_get_mut($f, ::forest::layer_access::nil_mut()), [$($fs)*].get_mut($ref))
    };
    { @rec $a:expr, [].get_mut($ref:expr) } => {
        ($a)($ref)
    };
    { @rec $a:expr, [$f:expr $(, $fs:expr),*].get_mut($ref:expr) } => {
        layers!(@rec ::forest::layer_access::cons_get_mut($f, $a), [$($fs)*].get_mut($ref))
    };

    { [$f:expr $(, $fs:expr),*].make_node($parent_ref:expr, $item:expr) } => {
        layers!(@rec $f, ::forest::layer_access::nil(), [$($fs)*].make_node($parent_ref, $item))
    };
    { @rec $fe:expr, $a:expr, [].make_node($parent_ref:expr, $item:expr) } => {
        $fe.make_node($a, $parent_ref, $item)
    };
    { @rec $fe:expr, $a:expr, [$f:expr $(, $fs:expr),*].make_node($parent_ref:expr, $item:expr) } => {
        layers!(@rec $f, ::forest::layer_access::cons_get($fe, $a), [$($fs)*].make_node($parent_ref, $item))
    };

    { [$f:expr $(, $fs:expr),*].towards_root_iter($ref:expr) } => {
        layers!(@rec ::forest::layer_access::cons_get($f, ::forest::layer_access::nil()), [$($fs)*].towards_root_iter($ref))
    };
    { @rec $a:expr, [].towards_root_iter($ref:expr) } => {
        TowardsRootIter::new($a, $ref)
    };
    { @rec $a:expr, [$f:expr $(, $fs:expr),*].towards_root_iter($ref:expr) } => {
        layers!(@rec ::forest::layer_access::cons_get($f, $a), [$($fs)*].towards_root_iter($ref))
    };

    { [$f:expr $(, $fs:expr),*].iter() } => {
        layers!(@rec $f.local_iter(), [$($fs)*].iter())
    };
    { @rec $i:expr, [].iter() } => {
        $i
    };
    { @rec $i:expr, [$f:expr $(, $fs:expr),*].iter() } => {
        layers!(@rec $i.map(|(set_ref, item)| (::forest::Ref2::External(set_ref), item)).chain($f.local_iter()), [$($fs)*].iter())
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

pub struct Forest1InitMerger<T>(SetsInitMerger<Node<T, Ref>, Node<T, Ref>>);

pub struct Forest1InProgressMerger<T> {
    inner_merger: SetsInProgressMerger<Node<T, Ref>, Node<T, Ref>>,
    parent: Option<Ref>,
    depth: usize,
}

impl<T> InitMerger<Ref, Ref, T, Forest1InProgressMerger<T>, Forest1<T>, Forest1<T>> for Forest1InitMerger<T> {
    fn ref_transform(&self, source_ref: Ref) -> Option<Ref> {
        self.0.ref_transform(source_ref)
    }

    fn merge_start(self) -> MergeState<Ref, T, Forest1InProgressMerger<T>, Forest1<T>, Forest1<T>> {
        Forest1InProgressMerger::make_state(self.0.merge_start())
    }
}

impl<T> Forest1InProgressMerger<T> {
    fn make_state(
        inner_state: MergeState<Ref, Node<T, Ref>, SetsInProgressMerger<Node<T, Ref>, Node<T, Ref>>, Set<Node<T, Ref>>, Set<Node<T, Ref>>>,
    ) -> MergeState<Ref, T, Forest1InProgressMerger<T>, Forest1<T>, Forest1<T>>
    {
        match inner_state {
            MergeState::Continue { item_ref, item: node, next, } =>
                MergeState::Continue {
                    item_ref,
                    item: node.item,
                    next: Forest1InProgressMerger {
                        inner_merger: next,
                        parent: node.parent,
                        depth: node.depth,
                    },
                },
            MergeState::Finish { merged, empty, } =>
                MergeState::Finish {
                    merged: Forest1 { nodes: merged, },
                    empty: Forest1 { nodes: empty, },
                },
        }
    }
}

impl<T> InProgressMerger<Ref, Ref, T, T, Forest1InProgressMerger<T>, Forest1<T>, Forest1<T>> for Forest1InProgressMerger<T> {
    fn ref_transform(&self, source_ref: Ref) -> Option<Ref> {
        self.inner_merger.ref_transform(source_ref)
    }

    fn proceed(self, transformed_item: T) -> MergeState<Ref, T, Forest1InProgressMerger<T>, Forest1<T>, Forest1<T>> {
        let node = Node {
            item: transformed_item,
            parent: self.parent.and_then(|parent_ref| self.inner_merger.ref_transform(parent_ref)),
            depth: self.depth,
        };
        Forest1InProgressMerger::make_state(self.inner_merger.proceed(node))
    }
}

pub struct Forest2AflatInitMerger<T, R>(SetsInitMerger<Node<T, Ref2<R>>, Node<T, Ref2<R>>>);

pub struct Forest2AflatInProgressMerger<T, R> {
    inner_merger: SetsInProgressMerger<Node<T, Ref2<R>>, Node<T, Ref2<R>>>,
    parent: Option<Ref2<R>>,
    depth: usize,
}

impl<T, R> InitMerger<Ref2<R>, Ref2<R>, T, Forest2AflatInProgressMerger<T, R>, Forest2<T, R>, Forest2<T, R>> for Forest2AflatInitMerger<T, R> {
    fn ref_transform(&self, source_ref: Ref2<R>) -> Option<Ref2<R>> {
        match source_ref {
            Ref2::Local(local_ref) =>
                self.0.ref_transform(local_ref).map(Ref2::Local),
            Ref2::External(external_ref) =>
                Some(Ref2::External(external_ref)),
        }
    }

    fn merge_start(self) -> MergeState<Ref2<R>, T, Forest2AflatInProgressMerger<T, R>, Forest2<T, R>, Forest2<T, R>> {
        Forest2AflatInProgressMerger::make_state(self.0.merge_start())
    }
}

impl<T, R> Forest2AflatInProgressMerger<T, R> {
    fn make_state(
        inner_state: MergeState<
            Ref, Node<T, Ref2<R>>, SetsInProgressMerger<Node<T, Ref2<R>>, Node<T, Ref2<R>>>, Set<Node<T, Ref2<R>>>, Set<Node<T, Ref2<R>>>>,
    ) -> MergeState<Ref2<R>, T, Forest2AflatInProgressMerger<T, R>, Forest2<T, R>, Forest2<T, R>>
    {
        match inner_state {
            MergeState::Continue { item_ref, item: node, next, } =>
                MergeState::Continue {
                    item_ref: Ref2::Local(item_ref),
                    item: node.item,
                    next: Forest2AflatInProgressMerger {
                        inner_merger: next,
                        parent: node.parent,
                        depth: node.depth,
                    },
                },
            MergeState::Finish { merged, empty, } =>
                MergeState::Finish {
                    merged: Forest2 { local_nodes: merged, },
                    empty: Forest2 { local_nodes: empty, },
                },
        }
    }
}

impl<T, R> InProgressMerger<Ref2<R>, Ref2<R>, T, T, Forest2AflatInProgressMerger<T, R>, Forest2<T, R>, Forest2<T, R>>
    for Forest2AflatInProgressMerger<T, R>
{
    fn ref_transform(&self, source_ref: Ref2<R>) -> Option<Ref2<R>> {
        match source_ref {
            Ref2::Local(local_ref) =>
                self.inner_merger.ref_transform(local_ref).map(Ref2::Local),
            Ref2::External(external_ref) =>
                Some(Ref2::External(external_ref)),
        }
    }

    fn proceed(self, transformed_item: T) -> MergeState<Ref2<R>, T, Forest2AflatInProgressMerger<T, R>, Forest2<T, R>, Forest2<T, R>> {
        let node = Node {
            item: transformed_item,
            parent: match self.parent {
                Some(Ref2::Local(local_ref)) =>
                    self.inner_merger.ref_transform(local_ref).map(Ref2::Local),
                Some(Ref2::External(external_ref)) =>
                    Some(Ref2::External(external_ref)),
                None =>
                    None,
            },
            depth: self.depth,
        };
        Forest2AflatInProgressMerger::make_state(self.inner_merger.proceed(node))
    }
}

pub struct Forest2Down1InitMerger<T>(SetsInitMerger<Node<T, Ref2<Ref>>, Node<T, Ref>>);

pub struct Forest2Down1InProgressMerger<T> {
    inner_merger: SetsInProgressMerger<Node<T, Ref2<Ref>>, Node<T, Ref>>,
    parent: Option<Ref2<Ref>>,
    depth: usize,
}

impl<T> InitMerger<Ref2<Ref>, Ref, T, Forest2Down1InProgressMerger<T>, Forest1<T>, Forest2<T, Ref>> for Forest2Down1InitMerger<T> {
    fn ref_transform(&self, source_ref: Ref2<Ref>) -> Option<Ref> {
        match source_ref {
            Ref2::Local(local_ref) =>
                self.0.ref_transform(local_ref),
            Ref2::External(external_ref) =>
                Some(external_ref),
        }
    }

    fn merge_start(self) -> MergeState<Ref2<Ref>, T, Forest2Down1InProgressMerger<T>, Forest1<T>, Forest2<T, Ref>> {
        Forest2Down1InProgressMerger::make_state(self.0.merge_start())
    }
}

impl<T> Forest2Down1InProgressMerger<T> {
    fn make_state(
        inner_state: MergeState<
            Ref, Node<T, Ref2<Ref>>, SetsInProgressMerger<Node<T, Ref2<Ref>>, Node<T, Ref>>, Set<Node<T, Ref>>, Set<Node<T, Ref2<Ref>>>>,
    ) -> MergeState<Ref2<Ref>, T, Forest2Down1InProgressMerger<T>, Forest1<T>, Forest2<T, Ref>>
    {
        match inner_state {
            MergeState::Continue { item_ref, item: node, next, } =>
                MergeState::Continue {
                    item_ref: Ref2::Local(item_ref),
                    item: node.item,
                    next: Forest2Down1InProgressMerger {
                        inner_merger: next,
                        parent: node.parent,
                        depth: node.depth,
                    },
                },
            MergeState::Finish { merged, empty, } =>
                MergeState::Finish {
                    merged: Forest1 { nodes: merged, },
                    empty: Forest2 { local_nodes: empty, },
                },
        }
    }
}

impl<T> InProgressMerger<Ref2<Ref>, Ref, T, T, Forest2Down1InProgressMerger<T>, Forest1<T>, Forest2<T, Ref>>
    for Forest2Down1InProgressMerger<T>
{
    fn ref_transform(&self, source_ref: Ref2<Ref>) -> Option<Ref> {
        match source_ref {
            Ref2::Local(local_ref) =>
                self.inner_merger.ref_transform(local_ref),
            Ref2::External(external_ref) =>
                Some(external_ref),
        }
    }

    fn proceed(self, transformed_item: T) -> MergeState<Ref2<Ref>, T, Forest2Down1InProgressMerger<T>, Forest1<T>, Forest2<T, Ref>> {
        let node = Node {
            item: transformed_item,
            parent: match self.parent {
                Some(Ref2::Local(local_ref)) =>
                    self.inner_merger.ref_transform(local_ref),
                Some(Ref2::External(external_ref)) =>
                    Some(external_ref),
                None =>
                    None,
            },
            depth: self.depth,
        };
        Forest2Down1InProgressMerger::make_state(self.inner_merger.proceed(node))
    }
}

pub struct Forest2Down2InitMerger<T, R>(SetsInitMerger<Node<T, Ref2<Ref2<R>>>, Node<T, Ref2<R>>>);

pub struct Forest2Down2InProgressMerger<T, R> {
    inner_merger: SetsInProgressMerger<Node<T, Ref2<Ref2<R>>>, Node<T, Ref2<R>>>,
    parent: Option<Ref2<Ref2<R>>>,
    depth: usize,
}

impl<T, R> InitMerger<Ref2<Ref2<R>>, Ref2<R>, T, Forest2Down2InProgressMerger<T, R>, Forest2<T, R>, Forest2<T, Ref2<R>>>
    for Forest2Down2InitMerger<T, R>
{
    fn ref_transform(&self, source_ref: Ref2<Ref2<R>>) -> Option<Ref2<R>> {
        match source_ref {
            Ref2::Local(local_ref) =>
                self.0.ref_transform(local_ref).map(Ref2::Local),
            Ref2::External(external_ref) =>
                Some(external_ref),
        }
    }

    fn merge_start(self) -> MergeState<Ref2<Ref2<R>>, T, Forest2Down2InProgressMerger<T, R>, Forest2<T, R>, Forest2<T, Ref2<R>>> {
        Forest2Down2InProgressMerger::make_state(self.0.merge_start())
    }
}

impl<T, R> Forest2Down2InProgressMerger<T, R> {
    fn make_state(
        inner_state: MergeState<
            Ref, Node<T, Ref2<Ref2<R>>>, SetsInProgressMerger<Node<T, Ref2<Ref2<R>>>, Node<T, Ref2<R>>>,
            Set<Node<T, Ref2<R>>>,
            Set<Node<T, Ref2<Ref2<R>>>>,
        >,
    ) -> MergeState<Ref2<Ref2<R>>, T, Forest2Down2InProgressMerger<T, R>, Forest2<T, R>, Forest2<T, Ref2<R>>>
    {
        match inner_state {
            MergeState::Continue { item_ref, item: node, next, } =>
                MergeState::Continue {
                    item_ref: Ref2::Local(item_ref),
                    item: node.item,
                    next: Forest2Down2InProgressMerger {
                        inner_merger: next,
                        parent: node.parent,
                        depth: node.depth,
                    },
                },
            MergeState::Finish { merged, empty, } =>
                MergeState::Finish {
                    merged: Forest2 { local_nodes: merged, },
                    empty: Forest2 { local_nodes: empty, },
                },
        }
    }
}

impl<T, R> InProgressMerger<Ref2<Ref2<R>>, Ref2<R>, T, T, Forest2Down2InProgressMerger<T, R>, Forest2<T, R>, Forest2<T, Ref2<R>>>
    for Forest2Down2InProgressMerger<T, R>
{
    fn ref_transform(&self, source_ref: Ref2<Ref2<R>>) -> Option<Ref2<R>> {
        match source_ref {
            Ref2::Local(local_ref) =>
                self.inner_merger.ref_transform(local_ref).map(Ref2::Local),
            Ref2::External(external_ref) =>
                Some(external_ref),
        }
    }

    fn proceed(self, transformed_item: T) -> MergeState<Ref2<Ref2<R>>, T, Forest2Down2InProgressMerger<T, R>, Forest2<T, R>, Forest2<T, Ref2<R>>> {
        let node = Node {
            item: transformed_item,
            parent: match self.parent {
                Some(Ref2::Local(local_ref)) =>
                    self.inner_merger.ref_transform(local_ref).map(Ref2::Local),
                Some(Ref2::External(external_ref)) =>
                    Some(external_ref),
                None =>
                    None,
            },
            depth: self.depth,
        };
        Forest2Down2InProgressMerger::make_state(self.inner_merger.proceed(node))
    }
}


#[cfg(test)]
mod test {
    use super::super::merge::merge_no_transform;
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
        let child_a = layers!([&forest1, &mut forest2].make_node(root_ext, "child a"));
        assert_eq!(layers!([&forest1, &forest2].get(child_a)).map(|node| node.item), Some(&"child a"));

        layers!([&mut forest1, &mut forest2].get_mut(child_a)).map(|node| *node.item = "other child");
        assert_eq!(layers!([&forest1, &forest2].get(child_a)).map(|node| node.item), Some(&"other child"));

        let child_b = layers!([&forest1, &mut forest2].make_node(child_a, "child b"));
        assert_eq!(layers!([&forest1, &forest2].get(child_b)).map(|node| node.item), Some(&"child b"));

        let iter = layers!([&forest1, &forest2].towards_root_iter(child_b));
        assert_eq!(iter.map(|node| node.item).collect::<Vec<_>>(), vec![&"child b", &"other child", &"root"]);

        let iter = layers!([&forest1, &forest2].iter()).map(|p| p.1);
        let mut items: Vec<_> = iter.collect();
        items.sort();
        assert_eq!(items, vec![&"child b", &"other child", &"root", &"root2"]);
    }

    #[test]
    fn merge_aflat_forest1() {
        let mut forest1_a = Forest1::new();
        let root_a = forest1_a.make_root("root a");
        let _child_a_a = layers!([&mut forest1_a].make_node(root_a, "child_a a"));
        let _child_b_a = layers!([&mut forest1_a].make_node(root_a, "child_b a"));

        let mut forest1_b = Forest1::new();
        let root_b = forest1_b.make_root("root b");
        let child_a_b = layers!([&mut forest1_b].make_node(root_b, "child_a b"));
        let _child_b_b = layers!([&mut forest1_b].make_node(child_a_b, "child_b b"));

        let forest1_a = merge_no_transform(forest1_b.merge_aflat(forest1_a));

        let mut items: Vec<_> = layers!([&forest1_a].iter()).collect();
        items.sort_by_key(|item| item.1);
        let verify = vec![&"child_a a", &"child_a b", &"child_b a", &"child_b b", &"root a", &"root b"];
        assert_eq!(items.len(), verify.len());
        for ((set_ref, item), verify_item) in items.into_iter().zip(verify) {
            assert_eq!(item, verify_item);
            assert_eq!(layers!([&forest1_a].get(set_ref)).map(|node| node.item), Some(item));
        }
    }

    #[test]
    fn merge_down_forest1() {
        let mut forest1 = Forest1::new();
        let root1 = forest1.make_root("root1");
        let child1_a = layers!([&mut forest1].make_node(root1, "child1 a"));
        let _child1_b = layers!([&mut forest1].make_node(root1, "child1 b"));

        let mut forest2 = Forest2::new();
        let root2 = forest2.make_root("root2");
        let child2_a = layers!([&forest1, &mut forest2].make_node(root2, "child2 a"));
        let _child2_b = layers!([&forest1, &mut forest2].make_node(child2_a, "child2 b"));
        let child1_a2 = forest2.external(child1_a);
        let _child2_c = layers!([&forest1, &mut forest2].make_node(child1_a2, "child2 c"));

        let forest1 = merge_no_transform(forest2.merge_down(forest1));

        let mut items: Vec<_> = layers!([&forest1].iter()).collect();
        items.sort_by_key(|item| item.1);
        let verify = vec![&"child1 a", &"child1 b", &"child2 a", &"child2 b", &"child2 c", &"root1", &"root2"];
        assert_eq!(items.len(), verify.len());
        for ((set_ref, item), verify_item) in items.iter().cloned().zip(verify) {
            assert_eq!(item, verify_item);
            assert_eq!(layers!([&forest1].get(set_ref)).map(|node| node.item), Some(item));
        }
        let child2_c = items[4].0;
        let path: Vec<_> = layers!([&forest1].towards_root_iter(child2_c)).map(|p| p.item).collect();
        assert_eq!(path, vec![&"child2 c", &"child1 a", &"root1"]);
    }
}
