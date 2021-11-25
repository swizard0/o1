use std::mem;

pub fn drain_merge_sorted<T, F>(vec_dst: &mut Vec<T>, vec: &mut Vec<T>, mut less_p: F) where F: FnMut(&T, &T) -> bool {
    if vec.is_empty() {
        return;
    }
    if vec_dst.is_empty() {
        mem::swap(vec_dst, vec);
    }

    vec_dst.reserve(vec.len());
    let mut vec_dst_len = vec_dst.len();
    let mut vec_len = vec.len();
    let mut total_len = vec_dst_len + vec_len;
    unsafe {
        vec_dst.set_len(total_len);
        loop {
            if vec_len == 0 {
                assert_eq!(vec_dst_len, total_len);
                break;
            } else if vec_dst_len == 0 {
                assert_eq!(vec_len, total_len);
                vec_dst.as_mut_ptr().copy_from_nonoverlapping(vec.as_ptr(), vec_len);
                break;
            } else {
                let item_dst = vec_dst.as_mut_ptr().add(vec_dst_len - 1);
                let item = vec.as_mut_ptr().add(vec_len - 1);
                let target = vec_dst.as_mut_ptr().add(total_len - 1);
                if less_p(&*item_dst, &*item) {
                    target.copy_from_nonoverlapping(item, 1);
                    vec_len -= 1;
                } else {
                    target.copy_from_nonoverlapping(item_dst, 1);
                    vec_dst_len -= 1;
                }
                total_len -= 1;
            }
        }
        vec.set_len(0);
    }
}

#[cfg(test)]
mod test {
    use rand::{self, Rng};

    fn less_p(a: &usize, b: &usize) -> bool {
        a < b
    }

    #[test]
    fn two_empty() {
        let mut vec_a = vec![];
        let mut vec_b = vec![];
        super::drain_merge_sorted(&mut vec_a, &mut vec_b, less_p);
        assert_eq!(vec_a, vec![]);
        assert_eq!(vec_b, vec![]);
    }

    #[test]
    fn dst_empty_and_src_not() {
        let mut vec_a = vec![];
        let mut vec_b = vec![42];
        super::drain_merge_sorted(&mut vec_a, &mut vec_b, less_p);
        assert_eq!(vec_a, vec![42]);
        assert_eq!(vec_b, vec![]);
    }

    #[test]
    fn src_empty_and_dst_not() {
        let mut vec_a = vec![42];
        let mut vec_b = vec![];
        super::drain_merge_sorted(&mut vec_a, &mut vec_b, less_p);
        assert_eq!(vec_a, vec![42]);
        assert_eq!(vec_b, vec![]);
    }

    #[test]
    fn simple() {
        let mut vec_a = vec![1, 4, 7, 8];
        let mut vec_b = vec![5, 6, 10];
        super::drain_merge_sorted(&mut vec_a, &mut vec_b, less_p);
        assert_eq!(vec_a, vec![1, 4, 5, 6, 7, 8, 10]);
        assert_eq!(vec_b, vec![]);
    }

    #[test]
    fn merge_100000() {
        let mut rng = rand::thread_rng();
        let vec_a_len = rng.gen_range(50000 .. 100000);
        let mut vec_a: Vec<_> = (0 .. vec_a_len).map(|_| rng.gen()).collect();
        vec_a.sort();
        let vec_b_len = rng.gen_range(1000 .. 100000);
        let mut vec_b: Vec<_> = (0 .. vec_b_len).map(|_| rng.gen()).collect();
        vec_b.sort();

        super::drain_merge_sorted(&mut vec_a, &mut vec_b, less_p);
        assert_eq!(vec_a.len(), vec_a_len + vec_b_len);
        assert_eq!(vec_b.len(), 0);

        let vec_a_len = vec_a.len();
        for i in 1 .. vec_a_len {
            assert!(vec_a[i - 1] < vec_a[i]);
        }
    }

    #[test]
    fn destructors() {
        use std::sync::atomic;
        let drops_counter = atomic::AtomicUsize::new(0);

        struct DropCount<'a> {
            value: usize,
            counter: &'a atomic::AtomicUsize,
        }

        impl<'a> Drop for DropCount<'a> {
            fn drop(&mut self) {
                self.counter.fetch_add(1, atomic::Ordering::Relaxed);
            }
        }

        let mut rng = rand::thread_rng();
        let vec_a_len = rng.gen_range(5000 .. 10000);
        let vec_b_len = rng.gen_range(100 .. 10000);

        {
            let mut vec_a: Vec<_> = (0 .. vec_a_len).map(|_| DropCount { value: rng.gen(), counter: &drops_counter, }).collect();
            vec_a.sort_by_key(|dc| dc.value);
            let mut vec_b: Vec<_> = (0 .. vec_b_len).map(|_| DropCount { value: rng.gen(), counter: &drops_counter, }).collect();
            vec_b.sort_by_key(|dc| dc.value);

            super::drain_merge_sorted(&mut vec_a, &mut vec_b, |dca, dcb| dca.value < dcb.value);
            assert_eq!(vec_a.len(), vec_a_len + vec_b_len);
            assert_eq!(vec_b.len(), 0);

            let vec_a_len = vec_a.len();
            for i in 1 .. vec_a_len {
                assert!(vec_a[i - 1].value < vec_a[i].value);
            }
        }

        assert_eq!(drops_counter.load(atomic::Ordering::Relaxed), vec_a_len + vec_b_len);
    }
}
