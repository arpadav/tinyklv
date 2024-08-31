pub trait HasValue<T> {
    fn v(&self) -> &T;
    fn v_mut(&mut self) -> &mut T;
    fn set(&mut self, v: T);
}
impl<T> HasValue<T> for Option<T> {
    fn v(&self) -> &T {
        self.as_ref().unwrap()
    }
    fn v_mut(&mut self) -> &mut T {
        self.as_mut().unwrap()
    }
    fn set(&mut self, v: T) {
        *self = Some(v);
    }
}