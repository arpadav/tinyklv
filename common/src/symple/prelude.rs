pub trait HasValue<T> {
    fn get(&self) -> Option<&T>;
    fn get_mut(&mut self) -> Option<&mut T>;
    fn set(&mut self, v: T);
}
impl<T> HasValue<T> for Option<T> {
    fn get(&self) -> Option<&T> {
        self.as_ref()
    }
    fn get_mut(&mut self) -> Option<&mut T> {
        self.as_mut()
    }
    fn set(&mut self, v: T) {
        *self = Some(v);
    }
}

pub trait Merge {
    fn merge(&mut self, other: Self);
}
pub trait MergeAll<T: Merge> {
    fn merge_all(self) -> Option<T>;
}

impl<I, T> MergeAll<T> for I
where
    I: IntoIterator<Item = T>,
    T: Merge,
{
    fn merge_all(self) -> Option<T> {
        let mut iter = self.into_iter();
        let mut merged_item = iter.next()?; // first item
        iter.for_each(|x| merged_item.merge(x));
        Some(merged_item)
    }
}
