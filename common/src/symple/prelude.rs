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