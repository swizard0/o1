
pub enum MergeState<SR, SI, K, FM, FE> {
    Continue { item_ref: SR, item: SI, next: K, },
    Finish { merged: FM, empty: FE, },
}

pub trait InitMerger<SR, TR, SI, K, FM, FE> {
    fn ref_transform(&self, source_ref: SR) -> Option<TR>;
    fn merge_start(self) -> MergeState<SR, SI, K, FM, FE>;
}

pub trait InProgressMerger<SR, TR, SI, TI, K, FM, FE> {
    fn ref_transform(&self, source_ref: SR) -> Option<TR>;
    fn proceed(self, transformed_item: TI) -> MergeState<SR, SI, K, FM, FE>;
}

pub fn merge_no_transform<IM, SR, TR, I, K, FM, FE>(merge_init: IM) -> FM
    where IM: InitMerger<SR, TR, I, K, FM, FE>,
          K: InProgressMerger<SR, TR, I, I, K, FM, FE>,
{
    let mut merge_step = merge_init.merge_start();
    loop {
        match merge_step {
            MergeState::Finish { merged, .. } =>
                return merged,
            MergeState::Continue { item, next, .. } =>
                merge_step = next.proceed(item),
        }
    }
}
