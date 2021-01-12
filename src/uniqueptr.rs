/// An box that bypasses borrow checking.
pub struct UniquePtr<T: ?Sized>(*mut T);
unsafe impl<T: Send + ?Sized> Send for UniquePtr<T> {}
unsafe impl<T: Sync + ?Sized> Sync for UniquePtr<T> {}

impl<T> UniquePtr<T> {
  pub fn new(val: T) -> Self {
    UniquePtr(Box::into_raw(Box::new(val)))
  }
}

impl<T: ?Sized> UniquePtr<T> {
  pub fn from_box(b: Box<T>) -> Self {
    UniquePtr(Box::into_raw(b))
  }

  pub unsafe fn deref(&self) -> &'static T {
    &*self.0
  }

  pub unsafe fn deref_mut(&self) -> &'static mut T {
    &mut *self.0
  }
}

impl<T: ?Sized> Drop for UniquePtr<T> {
  fn drop(&mut self) {
    unsafe {
      std::mem::drop(Box::from_raw(self.0));
    }
  }
}
