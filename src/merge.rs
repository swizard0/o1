
pub enum MergeState<SR, SI, K, F> {
    Continue { item_ref: SR, item: SI, next: K, },
    Finish(F),
}

pub trait InitMerger<SR, TR, SI, K, F> {
    fn ref_transform(&self, source_ref: SR) -> Option<TR>;
    fn merge_start(self) -> MergeState<SR, SI, K, F>;
}

pub trait InProgressMerger<SR, TR, SI, TI, K, F> {
    fn ref_transform(&self, source_ref: SR) -> Option<TR>;
    fn proceed(self, transformed_item: TI) -> MergeState<SR, SI, K, F>;
}
