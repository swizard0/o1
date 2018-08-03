
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
