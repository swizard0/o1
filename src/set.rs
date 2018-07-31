use std::{mem, sync::atomic::{self, AtomicUsize, ATOMIC_USIZE_INIT}};

#[cfg(feature = "with-rayon")]
use rayon::iter::{
    ParallelIterator,
    IntoParallelRefIterator,
    IndexedParallelIterator,
};

pub static UID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ref {
    index: usize,
    set_uid: u64,
    serial: u64,
}

pub struct Set<T> {
    uid: u64,
    serial: u64,
    cells: Vec<Cell<T>>,
    free: Vec<usize>,
    len: usize,
}

impl<T> Set<T> {
    pub fn new() -> Set<T> {
        Set {
            uid: UID_COUNTER.fetch_add(1, atomic::Ordering::Relaxed) as u64,
            serial: 0,
            cells: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Set<T> {
        Set {
            uid: UID_COUNTER.fetch_add(1, atomic::Ordering::Relaxed) as u64,
            serial: 0,
            cells: Vec::with_capacity(capacity),
            free: Vec::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn serial(&self) -> u64 {
        self.serial
    }

    pub fn insert(&mut self, item: T) -> Ref {
        let set_ref = self.insert_empty();
        self.cells[set_ref.index].state =
            CellState::Regular { item: Some(item), };
        set_ref
    }

    pub fn remove(&mut self, set_ref: Ref) -> Option<T> {
        if set_ref.set_uid != self.uid {
            return None;
        }
        match self.cells.get_mut(set_ref.index) {
            Some(Cell { serial, state: CellState::Regular { item: whole_item @ Some(..), }, }) if *serial == set_ref.serial => {
                self.free.push(set_ref.index);
                self.len -= 1;
                whole_item.take()
            },
            _ =>
                None
        }
    }

    pub fn get(&self, set_ref: Ref) -> Option<&T> {
        match self.cells.get(set_ref.index) {
            Some(&Cell { serial, state: CellState::Regular { ref item, }, }) if set_ref.set_uid == self.uid && serial == set_ref.serial =>
                item.as_ref(),
            _ =>
                None,
        }
    }

    pub fn get_mut(&mut self, set_ref: Ref) -> Option<&mut T> {
        match self.cells.get_mut(set_ref.index) {
            Some(&mut Cell { serial, state: CellState::Regular { ref mut item, }, }) if set_ref.set_uid == self.uid && serial == set_ref.serial =>
                item.as_mut(),
            _ =>
                None,
        }
    }

    pub fn consume<U, F>(&mut self, mut other_set: Set<U>, mut items_transformer: F) where F: ItemsTransformer<T, U> {
        // first add as many empty cells in `self` as in `other_set`
        // and replace `other_set`'s cells serials with new index
        self.cells.reserve(other_set.len());
        for other_cell in other_set.cells.iter_mut() {
            let taken_state = mem::replace(&mut other_cell.state, CellState::Regular { item: None, });
            if let CellState::Regular { item: Some(other_item), } = taken_state {
                let set_ref = self.insert_empty();
                other_cell.state = CellState::Reloc {
                    item: other_item,
                    reloc_index: set_ref.index,
                };
            }
        }

        // perform second pass with actual items transferring and transforming
        for other_cell_index in 0 .. other_set.cells.len() {
            let taken_state = mem::replace(&mut other_set.cells[other_cell_index].state, CellState::Regular { item: None, });
            if let CellState::Reloc { item: other_item, reloc_index, } = taken_state {
                other_set.cells[other_cell_index].state = CellState::Moved { reloc_index, };
                let other_set_ref = Ref {
                    index: other_cell_index,
                    serial: other_set.cells[other_cell_index].serial,
                    set_uid: other_set.uid,
                };
                let self_item = items_transformer.transform(other_set_ref, other_item, |transform_ref| {
                    match other_set.cells.get(transform_ref.index) {
                        Some(&Cell { serial, state: CellState::Moved { reloc_index, }, }) |
                        Some(&Cell { serial, state: CellState::Reloc { reloc_index, .. }, })
                            if other_set.uid == transform_ref.set_uid && serial == transform_ref.serial =>
                            Some(Ref {
                                index: reloc_index,
                                serial: self.cells[reloc_index].serial,
                                set_uid: self.uid,
                            }),
                        _ =>
                            None,
                    }
                });
                self.cells[reloc_index].state = CellState::Regular { item: Some(self_item), };
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Ref, &T)> {
        let set_uid = self.uid;
        self.cells.iter()
            .enumerate()
            .flat_map(move |(index, cell)| match cell.state {
                CellState::Regular { item: Some(ref item), } =>
                    Some((Ref { index, set_uid, serial: cell.serial, }, item)),
                _ =>
                    None,
            })
    }

    pub fn refs<'a>(&'a self) -> impl Iterator<Item = Ref> + 'a {
        self.iter().map(|pair| pair.0)
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.iter().map(|pair| pair.1)
    }

    #[cfg(feature = "with-rayon")]
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (Ref, &T)> where T: Sync {
        let set_uid = self.uid;
        self.cells.par_iter()
            .enumerate()
            .flat_map(move |(index, cell)| match cell.state {
                CellState::Regular { item: Some(ref item), } =>
                    Some((Ref { index, set_uid, serial: cell.serial, }, item)),
                _ =>
                    None,
            })
    }

    fn insert_empty(&mut self) -> Ref {
        self.serial += 1;
        let serial = self.serial;
        let index = if let Some(free_index) = self.free.pop() {
            self.cells[free_index] = Cell { serial, state: CellState::Regular { item: None, }, };
            free_index
        } else {
            let next_index = self.cells.len();
            self.cells.push(Cell { serial, state: CellState::Regular { item: None, }, });
            next_index
        };
        self.len += 1;
        Ref { index, serial, set_uid: self.uid, }
    }
}

pub trait ItemsTransformer<T, U> {
    fn transform<RF>(&mut self, set_ref: Ref, item: U, ref_transform: RF) -> T where RF: Fn(Ref) -> Option<Ref>;
}

impl<T, U, F> ItemsTransformer<T, U> for F where F: FnMut(Ref, U, &Fn(Ref) -> Option<Ref>) -> T {
    fn transform<RF>(&mut self, set_ref: Ref, item: U, ref_transform: RF) -> T where RF: Fn(Ref) -> Option<Ref> {
        (self)(set_ref, item, &ref_transform)
    }
}

struct Cell<T> {
    serial: u64,
    state: CellState<T>,
}

enum CellState<T> {
    Regular { item: Option<T>, },
    Reloc { item: T, reloc_index: usize, },
    Moved { reloc_index: usize, },
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, HashSet};
    use rand::{self, Rng};
    use super::Set;

    #[test]
    fn add_remove_10000() {
        let mut set = Set::new();
        let mut verify = HashMap::new();
        let mut rng = rand::thread_rng();
        for _ in 0 .. 10000 {
            let item: u64 = rng.gen();
            let set_ref = set.insert(item);
            verify.insert(item, set_ref);
        }
        for (item, &set_ref) in verify.iter() {
            let mut item = *item;
            assert_eq!(set.get(set_ref), Some(&item));
            assert_eq!(set.get_mut(set_ref), Some(&mut item));
            assert_eq!(set.remove(set_ref), Some(item));
        }
        for (&_, &set_ref) in verify.iter() {
            assert_eq!(set.remove(set_ref), None);
            assert_eq!(set.get(set_ref), None);
            assert_eq!(set.get_mut(set_ref), None);
        }
    }

    #[test]
    fn add_remove_loop_10000() {
        let mut set = Set::new();
        let mut inserted = Vec::new();
        let mut removed = Vec::new();
        let mut total = 0;
        let mut rng = rand::thread_rng();
        for item in 0 .. 100000 {
            match rng.gen_range(0, 10) {
                0 ..= 5 => {
                    let set_ref = set.insert(item);
                    inserted.push((item, set_ref));
                    total += 1;
                    assert_eq!(set.len(), total);
                },
                6 ..= 7 if !inserted.is_empty() => {
                    let index = rng.gen_range(0, inserted.len());
                    let (item, set_ref) = inserted.swap_remove(index);
                    assert_eq!(set.remove(set_ref), Some(item));
                    removed.push(set_ref);
                    total -= 1;
                    assert_eq!(set.len(), total);
                },
                8 ..= 9 if !removed.is_empty() => {
                    let index = rng.gen_range(0, removed.len());
                    let set_ref = removed[index];
                    assert_eq!(set.remove(set_ref), None);
                    assert_eq!(set.len(), total);
                },
                _ =>
                    (),
            }
        }
        let sample_table: HashSet<_> = set.iter().collect();
        for &(ref item, set_ref) in inserted.iter() {
            assert!(sample_table.contains(&(set_ref, item)));
        }
        let sample_table: HashSet<_> = inserted.iter().collect();
        for (set_ref, &item) in set.iter() {
            assert!(sample_table.contains(&(item, set_ref)));
        }
    }

    #[test]
    fn add_remove_consume_10000() {
        let mut set_a: Set<u64> = Set::new();
        let mut set_b: Set<u32> = Set::new();
        let mut inserted = Vec::new();

        let mut rng = rand::thread_rng();
        for _ in 0 .. 10000 {
            inserted.push((0, set_a.insert(rng.gen())));
            inserted.push((1, set_b.insert(rng.gen())));
        }
        rng.shuffle(&mut inserted);
        for _ in 0 .. 2500 {
            match inserted.pop() {
                Some((0, set_ref)) => {
                    set_a.remove(set_ref).unwrap();
                },
                Some((1, set_ref)) => {
                    set_b.remove(set_ref).unwrap();
                },
                Some((_, _)) =>
                    unreachable!(),
                None =>
                    (),
            }
        }

        let mut verify = HashMap::new();
        let mut table = HashSet::new();
        for (idx, set_ref) in inserted {
            table.insert((idx, set_ref));
            if idx == 0 {
                verify.insert(set_ref, set_a.get(set_ref).unwrap().clone());
            }
        }

        set_a.consume(set_b, |ref_b, item_b, ref_transform: &Fn(_) -> Option<_>| {
            assert!(table.remove(&(1, ref_b)));
            let ref_a = ref_transform(ref_b).unwrap();
            table.insert((0, ref_a));
            verify.insert(ref_a, item_b as u64);
            item_b as u64
        });

        for (idx, ref_a) in table {
            assert_eq!(idx, 0);
            let item_a = set_a.get(ref_a);
            let item_a_verify = verify.get(&ref_a);
            assert_eq!(item_a, item_a_verify);
        }
    }

    #[test]
    fn wrong_set_ref() {
        let mut set_a = Set::new();
        let mut set_b = Set::new();
        let set_a_ref = set_a.insert("set_a item");
        let set_b_ref = set_b.insert("set_b item");
        assert_eq!(set_a.get(set_a_ref), Some(&"set_a item"));
        assert_eq!(set_b.get(set_b_ref), Some(&"set_b item"));
        assert_eq!(set_a.get(set_b_ref), None);
        assert_eq!(set_b.get(set_a_ref), None);
    }
}
