use std::{mem, sync::atomic::{self, AtomicUsize, ATOMIC_USIZE_INIT}};

#[cfg(feature = "with-rayon")]
use rayon::iter::{
    ParallelIterator,
    IntoParallelRefIterator,
    IndexedParallelIterator,
};
use super::merge::{
    MergeState,
    InitMerger,
    InProgressMerger,
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

    pub fn merge<U>(mut self, mut source_set: Set<U>) -> SetsInitMerger<U, T> {
        self.cells.reserve(source_set.len());
        for source_cell in source_set.cells.iter_mut() {
            let taken_state = mem::replace(&mut source_cell.state, CellState::Regular { item: None, });
            if let CellState::Regular { item: Some(source_item), } = taken_state {
                let set_ref = self.insert_empty();
                source_cell.state = CellState::Reloc {
                    item: source_item,
                    reloc_index: set_ref.index,
                };
            }
        }
        SetsInitMerger {
            source: source_set,
            target: self,
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

struct Cell<T> {
    serial: u64,
    state: CellState<T>,
}

enum CellState<T> {
    Regular { item: Option<T>, },
    Reloc { item: T, reloc_index: usize, },
    Moved { reloc_index: usize, },
}

pub struct SetsInitMerger<SI, TI> {
    source: Set<SI>,
    target: Set<TI>,
}

pub struct SetsInProgressMerger<SI, TI> {
    source: Set<SI>,
    target: Set<TI>,
    next_index: usize,
    reloc_index: usize,
}

impl<SI, TI> InitMerger<Ref, Ref, SI, SetsInProgressMerger<SI, TI>, Set<TI>> for SetsInitMerger<SI, TI> {
    fn ref_transform(&self, source_ref: Ref) -> Option<Ref> {
        transform_ref(&self.target, &self.source, source_ref)
    }

    fn merge_start(self) -> MergeState<Ref, SI, SetsInProgressMerger<SI, TI>, Set<TI>> {
        SetsInProgressMerger::make_state(self.source, self.target, 0)
    }
}

impl<SI, TI> SetsInProgressMerger<SI, TI> {
    fn make_state(mut source_set: Set<SI>, target_set: Set<TI>, index: usize) -> MergeState<Ref, SI, SetsInProgressMerger<SI, TI>, Set<TI>> {
        for source_cell_index in index .. source_set.cells.len() {
            let taken_state =
                mem::replace(&mut source_set.cells[source_cell_index].state, CellState::Regular { item: None, });
            if let CellState::Reloc { item, reloc_index, } = taken_state {
                source_set.cells[source_cell_index].state = CellState::Moved { reloc_index, };
                let item_ref = Ref {
                    index: source_cell_index,
                    serial: source_set.cells[source_cell_index].serial,
                    set_uid: source_set.uid,
                };
                return MergeState::Continue {
                    item_ref, item,
                    next: SetsInProgressMerger {
                        source: source_set,
                        target: target_set,
                        next_index: source_cell_index + 1,
                        reloc_index,
                    },
                };
            }
        }
        MergeState::Finish(target_set)
    }
}

impl<SI, TI> InProgressMerger<Ref, Ref, SI, TI, SetsInProgressMerger<SI, TI>, Set<TI>> for SetsInProgressMerger<SI, TI> {
    fn ref_transform(&self, source_ref: Ref) -> Option<Ref> {
        transform_ref(&self.target, &self.source, source_ref)
    }

    fn proceed(mut self, transformed_item: TI) -> MergeState<Ref, SI, SetsInProgressMerger<SI, TI>, Set<TI>> {
        self.target.cells[self.reloc_index].state = CellState::Regular { item: Some(transformed_item), };
        SetsInProgressMerger::make_state(self.source, self.target, self.next_index)
    }
}

fn transform_ref<T, U>(target_set: &Set<T>, source_set: &Set<U>, source_ref: Ref) -> Option<Ref> {
    match source_set.cells.get(source_ref.index) {
        Some(&Cell { serial, state: CellState::Moved { reloc_index, }, }) |
        Some(&Cell { serial, state: CellState::Reloc { reloc_index, .. }, })
            if source_set.uid == source_ref.set_uid && serial == source_ref.serial =>
            Some(Ref {
                index: reloc_index,
                serial: target_set.cells[reloc_index].serial,
                set_uid: target_set.uid,
            }),
        _ =>
            None,
    }
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, HashSet};
    use rand::{self, Rng};
    use super::super::merge::{MergeState, InitMerger, InProgressMerger};
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

        let merge_init = set_a.merge(set_b);
        let mut merge_step = merge_init.merge_start();
        let set_a = loop {
            match merge_step {
                MergeState::Finish(set) =>
                    break set,
                MergeState::Continue { item_ref, item, next, } => {
                    assert!(table.remove(&(1, item_ref)));
                    let transformed_item = item as u64;
                    let transformed_ref = next.ref_transform(item_ref).unwrap();
                    table.insert((0, transformed_ref));
                    verify.insert(transformed_ref, transformed_item);
                    merge_step = next.proceed(transformed_item);
                },
            }
        };

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
