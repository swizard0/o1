use super::set::{Ref, Set};

// External doubly linked lists manager
pub struct List<T> {
    set: Set<Link<T>>,
    head: Option<Ref>,
}

pub struct Link<T> {
    pub item: T,
    prev: Option<Ref>,
    next: Option<Ref>,
}

impl<T> List<T> {
    pub fn new() -> List<T> {
        List {
            set: Set::new(),
            head: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> List<T> {
        List {
            set: Set::with_capacity(capacity),
            head: None,
        }
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn prepend(&mut self, item: T) -> Ref {
        if let Some(ref mut prev_head_ref) = self.head {
            let item_ref =
                self.set.insert(Link { item, prev: None, next: Some(*prev_head_ref), });
            if let Some(prev_head @ Link { prev: None, .. }) = self.set.get_mut(*prev_head_ref) {
                prev_head.prev = Some(item_ref);
            }
            *prev_head_ref = item_ref;
            item_ref
        } else {
            let item_ref =
                self.set.insert(Link { item, prev: None, next: None, });
            self.head = Some(item_ref);
            item_ref
        }
    }

    pub fn remove(&mut self, link_ref: Ref) -> Option<T> {
        if let Some(Link { item, prev, next, }) = self.set.remove(link_ref) {
            if self.head.map_or(false, |head_ref| head_ref == link_ref) {
                self.head = next;
            }
            prev.and_then(|prev_ref| self.set.get_mut(prev_ref))
                .map(|Link { next: prev_next, .. }| *prev_next = next);
            next.and_then(|next_ref| self.set.get_mut(next_ref))
                .map(|Link { prev: next_prev, .. }| *next_prev = prev);
            Some(item)
        } else {
            None
        }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.and_then(|head_ref| self.remove(head_ref))
    }

    pub fn iter<'a>(&'a self) -> ListIter<'a, T> {
        ListIter {
            set: &self.set,
            cur: self.head,
        }
    }
}

pub struct ListIter<'a, T: 'a> {
    set: &'a Set<Link<T>>,
    cur: Option<Ref>,
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let &Link { ref item, next, .. } = self.set.get(self.cur?)?;
        self.cur = next;
        Some(item)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use rand::{self, Rng};
    use super::List;

    #[test]
    fn remove_head() {
        let mut list = List::new();
        let _ref_a = list.prepend("a");
        let _ref_b = list.prepend("b");
        let ref_c = list.prepend("c");
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"b", &"a"]);
        assert_eq!(list.remove(ref_c), Some("c"));
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"b", &"a"]);
        assert_eq!(list.len(), list.iter().count());
    }

    #[test]
    fn remove_mid() {
        let mut list = List::new();
        let _ref_a = list.prepend("a");
        let ref_b = list.prepend("b");
        let _ref_c = list.prepend("c");
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"b", &"a"]);
        assert_eq!(list.remove(ref_b), Some("b"));
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"a"]);
        assert_eq!(list.len(), list.iter().count());
    }

    #[test]
    fn remove_tail() {
        let mut list = List::new();
        let ref_a = list.prepend("a");
        let _ref_b = list.prepend("b");
        let _ref_c = list.prepend("c");
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"b", &"a"]);
        assert_eq!(list.remove(ref_a), Some("a"));
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"b"]);
        assert_eq!(list.len(), list.iter().count());
    }

    #[test]
    fn complex() {
        let mut list = List::new();
        let _ref_a = list.prepend("a");
        let ref_b = list.prepend("b");
        let _ref_c = list.prepend("c");
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"b", &"a"]);
        assert_eq!(list.remove(ref_b), Some("b"));
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"a"]);
        assert_eq!(list.remove(ref_b), None);
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"a"]);
        let ref_b = list.prepend("b");
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"b", &"c", &"a"]);
        assert_eq!(list.len(), list.iter().count());
        assert_eq!(list.remove(ref_b), Some("b"));
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&"c", &"a"]);
    }

    #[test]
    fn stress() {
        let mut list = List::new();
        let mut table = HashMap::new();
        let mut refs = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0 .. 16384 {
            let choice = rng.gen_range(0 .. 100);
            if choice < 66 {
                let item: u64 = rng.gen();
                let link_ref = list.prepend(item);
                table.insert(link_ref, item);
                refs.push(link_ref);
            } else if !refs.is_empty() {
                let index = rng.gen_range(0 .. refs.len());
                let link_ref = refs.swap_remove(index);
                let item = list.remove(link_ref).unwrap();
                assert_eq!(table.remove(&link_ref), Some(item));
            }
        }
        let mut list_items: Vec<_> = list.iter().cloned().collect();
        list_items.sort();
        let mut table_items: Vec<_> = table.values().cloned().collect();
        table_items.sort();
        assert_eq!(list_items, table_items);
    }
}
