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

use rayon::iter::ParallelIterator;

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

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn make_root(&mut self, item: T) -> Ref {
        self.insert(Node { item, parent: None, depth: 0, })
    }

    pub fn insert(&mut self, node: Node<T, Ref>) -> Ref {
        self.nodes.insert(node)
    }

    pub fn get<'s>(&'s self, node_ref: Ref) -> Option<Node<&'s T, Ref>> {
        self.nodes.get(node_ref)
            .map(|node| Node { item: &node.item, parent: node.parent, depth: node.depth, })
    }

    pub fn get_mut<'s>(&'s mut self, node_ref: Ref) -> Option<Node<&'s mut T, Ref>> {
        self.nodes.get_mut(node_ref)
            .map(|node| Node { item: &mut node.item, parent: node.parent, depth: node.depth, })
    }

    pub fn remove(&mut self, node_ref: Ref) -> Option<Node<T, Ref>> {
        self.nodes.remove(node_ref)
    }

    pub fn make_node<'s>(&'s mut self, parent_ref: Ref, item: T) -> Ref {
        if let Some(parent_depth) = self.get(parent_ref.clone()).map(|node| node.depth) {
            self.insert(Node { item, parent: Some(parent_ref), depth: parent_depth + 1, })
        } else {
            self.insert(Node { item, parent: None, depth: 0, })
        }
    }

    pub fn merge_aflat(self, target: Forest1<T>) -> Forest1InitMerger<T> {
        Forest1InitMerger(target.nodes.merge(self.nodes))
    }

    pub fn local_iter(&self) -> impl Iterator<Item = (Ref, &T)> {
        self.nodes.iter().map(|(set_ref, node)| (set_ref, &node.item))
    }

    pub fn local_par_iter(&self) -> impl ParallelIterator<Item = (Ref, &T)> where T: Sync {
        self.nodes.par_iter().map(|(set_ref, node)| (set_ref, &node.item))
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

    pub fn clear(&mut self) {
        self.local_nodes.clear();
    }

    pub fn len(&self) -> usize {
        self.local_nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn make_root(&mut self, item: T) -> Ref2<R> {
        self.insert(Node { item, parent: None, depth: 0, })
    }

    pub fn insert(&mut self, node: Node<T, Ref2<R>>) -> Ref2<R> {
        Ref2::Local(self.local_nodes.insert(node))
    }

    pub fn get<'s, 'a: 's, A>(&'s self, upper_layer_access: A, node_ref: Ref2<R>) -> Option<Node<&'s T, Ref2<R>>>
        where T: 'a, R: Clone, A: FnOnce(R) -> Option<Node<&'a T, R>>
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

    pub fn get_mut<'s, 'a: 's, A>(&'s mut self, upper_layer_access: A, node_ref: Ref2<R>) -> Option<Node<&'s mut T, Ref2<R>>>
        where T: 'a, R: Clone, A: FnOnce(R) -> Option<Node<&'a mut T, R>>
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

    pub fn remove<A>(&mut self, upper_layer_access: A, node_ref: Ref2<R>) -> Option<Node<T, Ref2<R>>>
        where A: FnOnce(R) -> Option<Node<T, R>>
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

    pub fn make_node<'s, 'a, A>(&'s mut self, upper_layer_access: A, parent_ref: Ref2<R>, item: T) -> Ref2<R>
        where T: 'a, R: Clone, A: FnOnce(R) -> Option<Node<&'a T, R>>
    {
        if let Some(parent_depth) = self.get(upper_layer_access, parent_ref.clone()).map(|node| node.depth) {
            self.insert(Node { item, parent: Some(parent_ref), depth: parent_depth + 1, })
        } else {
            self.insert(Node { item, parent: None, depth: 0, })
        }
    }

    pub fn external_ref(&self, node_ref: R) -> Ref2<R> {
        Ref2::External(node_ref)
    }

    pub fn local_iter(&self) -> impl Iterator<Item = (Ref2<R>, &T)> {
        self.local_nodes.iter()
            .map(|(set_ref, node)| (Ref2::Local(set_ref), &node.item))
    }

    pub fn local_par_iter(&self) -> impl ParallelIterator<Item = (Ref2<R>, &T)> where T: Sync, R: Sync + Send {
        self.local_nodes.par_iter()
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

#[macro_export]
macro_rules! layers {
    // [&forest].get(ref)
    { [$f:expr].get($ref:expr) } => {
        $crate::forest::Forest1::get($f, $ref)
    };
    { [$f:expr $(, $fs:expr)+].get($ref:expr) } => {
        $crate::forest::Forest2::get($f, |r| layers!([$($fs),*].get(r)), $ref)
    };

    // [&mut forest].get_mut(ref)
    { [$f:expr].get_mut($ref:expr) } => {
        $crate::forest::Forest1::get_mut($f, $ref)
    };
    { [$f:expr $(, $fs:expr)+].get_mut($ref:expr) } => {
        $crate::forest::Forest2::get_mut($f, |r| layers!([$($fs),*].get_mut(r)), $ref)
    };

    // [&mut forest].make_node(parent_ref, item)
    { [$f:expr].make_node($ref:expr, $item:expr) } => {
        $crate::forest::Forest1::make_node($f, $ref, $item)
    };
    { [$f:expr $(, $fs:expr)+].make_node($ref:expr, $item:expr) } => {
        $crate::forest::Forest2::make_node($f, |r| layers!([$($fs),*].get(r)), $ref, $item)
    };

    // [&mut forest].remove(ref)
    { [$f:expr].remove($ref:expr) } => {
        $crate::forest::Forest1::remove($f, $ref)
    };
    { [$f:expr $(, $fs:expr)+].remove($ref:expr) } => {
        $crate::forest::Forest2::remove($f, |r| layers!([$($fs),*].remove(r)), $ref)
    };

    // [&forest].towards_root_iter(ref)
    { [$($fs:expr),+].towards_root_iter($ref:expr) } => {
        $crate::forest::TowardsRootIter::new(|r| layers!([$($fs),*].get(r)), $ref)
    };

    // [&forest].iter()
    { [$f:expr].iter() } => {
        $crate::forest::Forest1::local_iter($f)
    };
    { [$f:expr $(, $fs:expr)+].iter() } => {
        $crate::forest::Forest2::local_iter($f)
            .chain(layers!([$($fs),*].iter()).map(|(set_ref, item)| ($crate::forest::Ref2::External(set_ref), item)))
    };

    // [&forest].par_iter()
    { [$f:expr].par_iter() } => {
        $crate::forest::Forest1::local_par_iter($f)
    };
    { [$f:expr $(, $fs:expr)+].par_iter() } => {
        $crate::forest::Forest2::local_par_iter($f)
            .chain(layers!([$($fs),*].par_iter()).map(|(set_ref, item)| ($crate::forest::Ref2::External(set_ref), item)))
    };
}

pub struct TowardsRootIter<R, A> {
    start: R,
    cursor: Option<R>,
    layer_access: A,
}

impl<R, A> TowardsRootIter<R, A> {
    pub fn new(layer_access: A, node_ref: R) -> TowardsRootIter<R, A> where R: Clone {
        TowardsRootIter {
            start: node_ref.clone(),
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.layer_access)(self.start.clone())
            .map_or((0, None), |node| (node.depth + 1, Some(node.depth + 1)))
    }

    fn count(self) -> usize where Self: Sized {
        (self.layer_access)(self.start.clone())
            .map_or(0, |node| node.depth + 1)
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
        Forest1,
        Forest2,
        TowardsRootIter,
    };

    #[test]
    fn direct_access() {
        let mut forest1 = Forest1::new();

        let root = forest1.make_root("root");
        assert_eq!(forest1.get(root).map(|node| node.item), Some(&"root"));
        assert_eq!(forest1.get(root).map(|node| node.parent), Some(None));
        assert_eq!(forest1.get(root).map(|node| node.depth), Some(0));

        let child_a = forest1.make_node(root, "child_a");
        assert_eq!(forest1.get(child_a).map(|node| node.item), Some(&"child_a"));
        assert_eq!(forest1.get(child_a).map(|node| node.parent), Some(Some(root)));
        assert_eq!(forest1.get(child_a).map(|node| node.depth), Some(1));

        let child_b = forest1.make_node(root, "child_b");
        assert_eq!(forest1.get(child_b).map(|node| node.item), Some(&"child_b"));
        assert_eq!(forest1.get(child_b).map(|node| node.parent), Some(Some(root)));
        assert_eq!(forest1.get(child_b).map(|node| node.depth), Some(1));

        let child_c = forest1.make_node(child_a, "child_c");
        assert_eq!(forest1.get(child_c).map(|node| node.item), Some(&"child_c"));
        assert_eq!(forest1.get(child_c).map(|node| node.parent), Some(Some(child_a)));
        assert_eq!(forest1.get(child_c).map(|node| node.depth), Some(2));

        let mut forest2 = Forest2::new();

        let root2 = forest2.make_root("root2");
        assert_eq!(forest2.get(|r| forest1.get(r), root2).map(|node| node.item), Some(&"root2"));
        assert_eq!(forest2.get(|r| forest1.get(r), root2).map(|node| node.parent), Some(None));
        assert_eq!(forest2.get(|r| forest1.get(r), root2).map(|node| node.depth), Some(0));

        let child_d = forest2.make_node(|r| forest1.get(r), root2, "child_d");
        assert_eq!(forest2.get(|r| forest1.get(r), child_d).map(|node| node.item), Some(&"child_d"));
        assert_eq!(forest2.get(|r| forest1.get(r), child_d).map(|node| node.parent), Some(Some(root2)));
        assert_eq!(forest2.get(|r| forest1.get(r), child_d).map(|node| node.depth), Some(1));

        let child_c_ext = forest2.external_ref(child_c);
        let child_e = forest2.make_node(|r| forest1.get(r), child_c_ext, "child_e");
        assert_eq!(forest2.get(|r| forest1.get(r), child_e).map(|node| node.item), Some(&"child_e"));
        assert_eq!(forest2.get(|r| forest1.get(r), child_e).map(|node| node.parent), Some(Some(child_c_ext)));
        assert_eq!(forest2.get(|r| forest1.get(r), child_e).map(|node| node.depth), Some(3));

        let iter = TowardsRootIter::new(|rr| forest2.get(|r| forest1.get(r), rr), child_e);
        assert_eq!(iter.map(|node| node.item).collect::<Vec<_>>(), vec![&"child_e", &"child_c", &"child_a", &"root"]);
    }

    #[test]
    fn layers_access_macro() {
        let mut forest1 = Forest1::new();
        let mut forest2 = Forest2::new();

        let root = forest1.make_root("root");
        assert_eq!(layers!([&forest1].get(root)).map(|node| node.item), Some(&"root"));

        let root2 = forest2.make_root("root2");
        assert_eq!(layers!([&forest2, &forest1].get(root2)).map(|node| node.item), Some(&"root2"));

        let root_ext = forest2.external_ref(root);
        let child_a = layers!([&mut forest2, &forest1].make_node(root_ext, "child a"));
        assert_eq!(layers!([&forest2, &forest1].get(child_a)).map(|node| node.item), Some(&"child a"));

        layers!([&mut forest2, &mut forest1].get_mut(child_a)).map(|node| *node.item = "other child");
        assert_eq!(layers!([&forest2, &forest1].get(child_a)).map(|node| node.item), Some(&"other child"));

        let child_b = layers!([&mut forest2, &forest1].make_node(child_a, "child b"));
        assert_eq!(layers!([&forest2, &forest1].get(child_b)).map(|node| node.item), Some(&"child b"));

        {
            let iter = layers!([&forest2, &forest1].towards_root_iter(child_b));
            assert_eq!(iter.map(|node| node.item).collect::<Vec<_>>(), vec![&"child b", &"other child", &"root"]);
        }
        {
            let iter = layers!([&forest2, &forest1].iter()).map(|p| p.1);
            let mut items: Vec<_> = iter.collect();
            items.sort();
            assert_eq!(items, vec![&"child b", &"other child", &"root", &"root2"]);
        }

        let mut maybe_node_ref = Some(child_b);
        while let Some(node_ref) = maybe_node_ref {
            maybe_node_ref = layers!([&forest2, &forest1].get(node_ref)).and_then(|node| node.parent);
            layers!([&mut forest2, &mut forest1].remove(node_ref));
        }
        assert_eq!(layers!([&forest2, &forest1].get(child_b)).map(|node| node.item), None);
        assert_eq!(layers!([&forest2, &forest1].get(child_a)).map(|node| node.item), None);
        assert_eq!(layers!([&forest2, &forest1].get(root_ext)).map(|node| node.item), None);
        assert_eq!(layers!([&forest2, &forest1].get(root2)).map(|node| node.item), Some(&"root2"));
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
    fn merge_aflat_forest2() {
        let forest1 = Forest1::new();
        let mut forest2_a = Forest2::new();
        let root_a = forest2_a.make_root("root a");
        let _child_a_a = layers!([&mut forest2_a, &forest1].make_node(root_a, "child_a a"));
        let _child_b_a = layers!([&mut forest2_a, &forest1].make_node(root_a, "child_b a"));

        let mut forest2_b = Forest2::new();
        let root_b = forest2_b.make_root("root b");
        let child_a_b = layers!([&mut forest2_b, &forest1].make_node(root_b, "child_a b"));
        let _child_b_b = layers!([&mut forest2_b, &forest1].make_node(child_a_b, "child_b b"));

        let forest2_a = merge_no_transform(forest2_b.merge_aflat(forest2_a));

        let mut items: Vec<_> = layers!([&forest2_a, &forest1].iter()).collect();
        items.sort_by_key(|item| item.1);
        let verify = vec![&"child_a a", &"child_a b", &"child_b a", &"child_b b", &"root a", &"root b"];
        assert_eq!(items.len(), verify.len());
        for ((set_ref, item), verify_item) in items.into_iter().zip(verify) {
            assert_eq!(item, verify_item);
            assert_eq!(layers!([&forest2_a, &forest1].get(set_ref)).map(|node| node.item), Some(item));
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
        let child2_a = layers!([&mut forest2, &forest1].make_node(root2, "child2 a"));
        let _child2_b = layers!([&mut forest2, &forest1].make_node(child2_a, "child2 b"));
        let child1_a2 = forest2.external_ref(child1_a);
        let _child2_c = layers!([&mut forest2, &forest1].make_node(child1_a2, "child2 c"));

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

    #[test]
    fn merge_down_forest2() {
        let forest0 = Forest1::new();
        let mut forest1 = Forest2::new();
        let root1 = forest1.make_root("root1");
        let child1_a = layers!([&mut forest1, &forest0].make_node(root1, "child1 a"));
        let _child1_b = layers!([&mut forest1, &forest0].make_node(root1, "child1 b"));

        let mut forest2 = Forest2::new();
        let root2 = forest2.make_root("root2");
        let child2_a = layers!([&mut forest2, &forest1, &forest0].make_node(root2, "child2 a"));
        let _child2_b = layers!([&mut forest2, &forest1, &forest0].make_node(child2_a, "child2 b"));
        let child1_a2 = forest2.external_ref(child1_a);
        let _child2_c = layers!([&mut forest2, &forest1, &forest0].make_node(child1_a2, "child2 c"));

        let forest1 = merge_no_transform(forest2.merge_down(forest1));

        let mut items: Vec<_> = layers!([&forest1, &forest0].iter()).collect();
        items.sort_by_key(|item| item.1);
        let verify = vec![&"child1 a", &"child1 b", &"child2 a", &"child2 b", &"child2 c", &"root1", &"root2"];
        assert_eq!(items.len(), verify.len());
        for ((set_ref, item), verify_item) in items.iter().cloned().zip(verify) {
            assert_eq!(item, verify_item);
            assert_eq!(layers!([&forest1, &forest0].get(set_ref)).map(|node| node.item), Some(item));
        }
        let child2_c = items[4].0;
        let forest0_ref = &forest0;
        let path: Vec<_> = layers!([&forest1, forest0_ref].towards_root_iter(child2_c)).map(|p| p.item).collect();
        assert_eq!(path, vec![&"child2 c", &"child1 a", &"root1"]);
    }

    #[test]
    fn merge_down_forest21() {
        let mut forest0 = Forest1::new();
        let root0 = forest0.make_root("root0");
        let _child0_a = layers!([&mut forest0].make_node(root0, "child0 a"));
        let child0_b = layers!([&mut forest0].make_node(root0, "child0 b"));

        let mut forest1 = Forest2::new();
        let root1 = forest1.make_root("root1");
        let child0_b = forest1.external_ref(child0_b);
        let child1_a = layers!([&mut forest1, &forest0].make_node(child0_b, "child1 a"));
        let _child1_b = layers!([&mut forest1, &forest0].make_node(root1, "child1 b"));

        let mut forest2 = Forest2::new();
        let root2 = forest2.make_root("root2");
        let child2_a = layers!([&mut forest2, &forest1, &forest0].make_node(root2, "child2 a"));
        let _child2_b = layers!([&mut forest2, &forest1, &forest0].make_node(child2_a, "child2 b"));
        let child1_a2 = forest2.external_ref(child1_a);
        let _child2_c = layers!([&mut forest2, &forest1, &forest0].make_node(child1_a2, "child2 c"));

        let forest1 = merge_no_transform(forest2.merge_down(forest1));
        let forest0 = merge_no_transform(forest1.merge_down(forest0));

        let mut items: Vec<_> = layers!([&forest0].iter()).collect();
        items.sort_by_key(|item| item.1);
        let verify = vec![&"child0 a", &"child0 b", &"child1 a", &"child1 b", &"child2 a", &"child2 b", &"child2 c", &"root0", &"root1", &"root2"];
        assert_eq!(items.len(), verify.len());
        for ((set_ref, item), verify_item) in items.iter().cloned().zip(verify) {
            assert_eq!(item, verify_item);
            assert_eq!(layers!([&forest0].get(set_ref)).map(|node| node.item), Some(item));
        }
        let child2_c = items[6].0;
        let path: Vec<_> = layers!([&forest0].towards_root_iter(child2_c)).map(|p| p.item).collect();
        assert_eq!(path, vec![&"child2 c", &"child1 a", &"child0 b", &"root0"]);
    }

    #[test]
    fn par_iter_forest21() {
        let mut forest0 = Forest1::new();
        let root0 = forest0.make_root("root0");
        let _child0_a = layers!([&mut forest0].make_node(root0, "child0 a"));
        let child0_b = layers!([&mut forest0].make_node(root0, "child0 b"));

        let mut forest1 = Forest2::new();
        let root1 = forest1.make_root("root1");
        let child0_b = forest1.external_ref(child0_b);
        let child1_a = layers!([&mut forest1, &forest0].make_node(child0_b, "child1 a"));
        let _child1_b = layers!([&mut forest1, &forest0].make_node(root1, "child1 b"));

        let mut forest2 = Forest2::new();
        let root2 = forest2.make_root("root2");
        let child2_a = layers!([&mut forest2, &forest1, &forest0].make_node(root2, "child2 a"));
        let _child2_b = layers!([&mut forest2, &forest1, &forest0].make_node(child2_a, "child2 b"));
        let child1_a2 = forest2.external_ref(child1_a);
        let _child2_c = layers!([&mut forest2, &forest1, &forest0].make_node(child1_a2, "child2 c"));

        use rayon::iter::ParallelIterator;
        let mut items: Vec<_> = layers!([&forest2, &forest1, &forest0].par_iter()).map(|p| p.1).collect();
        items.sort();
        assert_eq!(items, vec![
            &"child0 a", &"child0 b", &"child1 a", &"child1 b",
            &"child2 a", &"child2 b", &"child2 c",
            &"root0", &"root1", &"root2",
        ]);
    }
}
