use std::sync::atomic::{self, AtomicUsize, ATOMIC_USIZE_INIT};

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
    cells: Vec<Option<Cell<T>>>,
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
        self.serial += 1;
        let serial = self.serial;
        let cell = Cell { item, serial };
        let index = if let Some(free_index) = self.free.pop() {
            assert!(self.cells[free_index].is_none());
            self.cells[free_index] = Some(cell);
            free_index
        } else {
            let next_index = self.cells.len();
            self.cells.push(Some(cell));
            next_index
        };
        self.len += 1;
        Ref { index, serial, set_uid: self.uid, }
    }

    pub fn remove(&mut self, set_ref: Ref) -> Option<T> {
        if set_ref.set_uid == self.uid && set_ref.index < self.cells.len() {
            if let Some(Cell { item, serial, }) = self.cells[set_ref.index].take() {
                if serial == set_ref.serial {
                    self.free.push(set_ref.index);
                    self.len -= 1;
                    return Some(item);
                } else {
                    self.cells[set_ref.index] = Some(Cell { item, serial, });
                }
            }
        }
        None
    }

    pub fn get(&self, set_ref: Ref) -> Option<&T> {
        match self.cells.get(set_ref.index) {
            Some(Some(Cell { ref item, serial, })) if set_ref.set_uid == self.uid && serial == &set_ref.serial =>
                Some(item),
            _ =>
                None,
        }
    }

    pub fn get_mut(&mut self, set_ref: Ref) -> Option<&mut T> {
        match self.cells.get_mut(set_ref.index) {
            Some(Some(Cell { ref mut item, serial, })) if set_ref.set_uid == self.uid && serial == &set_ref.serial =>
                Some(item),
            _ =>
                None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Ref, &T)> {
        let set_uid = self.uid;
        self.cells.iter()
            .enumerate()
            .flat_map(move |(index, cell)| {
                cell.as_ref().map(|&Cell { ref item, serial, }| (Ref { index, set_uid, serial, }, item))
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
            .flat_map(move |(index, cell)| {
                cell.as_ref().map(|&Cell { ref item, serial, }| (Ref { index, set_uid, serial, }, item))
            })
    }
}

struct Cell<T> {
    item: T,
    serial: u64,
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
