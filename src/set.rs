
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ref {
    index: usize,
    serial: u64,
}

pub struct Set<T> {
    serial: u64,
    cells: Vec<Option<Cell<T>>>,
    free: Vec<usize>,
}

impl<T> Set<T> {
    pub fn new() -> Set<T> {
        Set {
            serial: 0,
            cells: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Set<T> {
        Set {
            serial: 0,
            cells: Vec::with_capacity(capacity),
            free: Vec::new(),
        }
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
        Ref { index, serial, }
    }

    pub fn remove(&mut self, set_ref: Ref) -> Option<T> {
        if set_ref.index < self.cells.len() {
            if let Some(Cell { item, serial, }) = self.cells[set_ref.index].take() {
                if serial == set_ref.serial {
                    self.free.push(set_ref.index);
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
            Some(Some(Cell { ref item, serial, })) if serial == &set_ref.serial =>
                Some(item),
            _ =>
                None,
        }
    }

    pub fn get_mut(&mut self, set_ref: Ref) -> Option<&mut T> {
        match self.cells.get_mut(set_ref.index) {
            Some(Some(Cell { ref mut item, serial, })) if serial == &set_ref.serial =>
                Some(item),
            _ =>
                None,
        }
    }

}

struct Cell<T> {
    item: T,
    serial: u64,
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
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
        let mut rng = rand::thread_rng();
        for item in 0 .. 10000 {
            match rng.gen_range(0, 10) {
                0 ..= 5 => {
                    let set_ref = set.insert(item);
                    inserted.push((item, set_ref));
                },
                6 ..= 7 if !inserted.is_empty() => {
                    let index = rng.gen_range(0, inserted.len());
                    let (item, set_ref) = inserted.swap_remove(index);
                    assert_eq!(set.remove(set_ref), Some(item));
                    removed.push(set_ref);
                },
                8 ..= 9 if !removed.is_empty() => {
                    let index = rng.gen_range(0, removed.len());
                    let set_ref = removed[index];
                    assert_eq!(set.remove(set_ref), None);
                },
                _ =>
                    (),
            }
        }
    }
}
