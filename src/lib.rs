//! FAUX
//!
//! A library to create mocks out of `struct`s without polluting your
//! code with traits that exist for test only.
//!
//! This library makes liberal use of unsafe Rust features, and it is
//! not recommended for use outside of tests.
//!
//! Basic Usage:
//! ```edition2018
//! // creates the mockable struct
//! #[faux::create]
//! pub struct Foo {
//!     a: u32,
//! }
//!
//! // mocks the methods
//! #[faux::methods]
//! impl Foo {
//!     pub fn new(a: u32) -> Self {
//!         Foo { a }
//!     }
//!
//!     pub fn get_stuff(&self) -> u32 {
//!         self.a
//!     }
//! }
//!
//! fn main() {
//!   // `faux` will not override making the real version of your struct
//!   let real = Foo::new(3);
//!   assert_eq!(real.get_stuff(), 3);
//!
//!   // while providing a method to create a mock
//!   let mut mock = Foo::faux();
//!   unsafe { faux::when!(mock.get_stuff).then(|_| 10) }
//!   assert_eq!(mock.get_stuff(), 10);
//! }
//! ```

pub use faux_macros::{create, methods};
use proc_macro_hack::proc_macro_hack;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
};

#[proc_macro_hack]
pub use faux_macros::when;

pub struct WhenHolder<'q, I, O> {
    pub id: TypeId,
    pub faux: &'q mut Faux,
    pub _marker: std::marker::PhantomData<(I, O)>,
}

impl<'q, I, O> WhenHolder<'q, I, O> {
    /// Stores the mock method
    ///
    /// # Safety:
    ///
    /// This function effectively erases the lifetime relationships of
    /// the inputs and outputs It is the user's responsability to not
    /// pass a mock that would capture a variable that would be used
    /// after it has been deallocated.
    ///
    /// Another way in which this function is unsafe is if the output
    /// of this function has a logical lifetime link to the input.  At
    /// the moment the mock gets called, that link would be erased
    /// which could create multiple mutable references to the same
    /// object.
    ///
    /// Example:
    ///
    /// ```
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn out_ref(&self, a : &mut i32) -> &mut i32 {
    ///         panic!("something here")
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///   // set up the mock such that the output is the same reference as the input
    ///   unsafe { faux::when!(mock.out_ref).then(|i| i) }
    ///
    ///   let mut x = 5;
    ///   // y is now a mutable reference back x, but there is no compile-time link between the two
    ///   let y = mock.out_ref(&mut x);
    ///
    ///   // We can check that they are both the same value
    ///   assert_eq!(*y, 5);
    ///   assert_eq!(x, 5);
    ///
    ///   // x now changes y. This is UB and is not allowed in safe Rust!
    ///   x += 1;
    ///   assert_eq!(x, 6);
    ///   assert_eq!(*y, 6);
    ///
    ///   // and if we change y then x also gets changed
    ///   *y += 1;
    ///   assert_eq!(x, 7);
    ///   assert_eq!(*y, 7);
    /// }
    /// ```
    pub unsafe fn then(self, mock: impl FnOnce(I) -> O) {
        self.faux.mock_once(self.id, mock);
    }
}

#[doc(hidden)]
pub enum MaybeFaux<T> {
    Real(T),
    Faux(RefCell<Faux>),
}

impl<T> MaybeFaux<T> {
    pub fn faux() -> Self {
        MaybeFaux::Faux(RefCell::new(Faux::default()))
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct Faux {
    one_time_mocks: HashMap<TypeId, Box<dyn FnOnce(()) -> ()>>,
    safe_one_time_mocks: HashMap<TypeId, Box<dyn FnOnce(Box<dyn Any>) -> Box<dyn Any>>>,
}

impl Faux {
    pub unsafe fn mock_once<I, O>(&mut self, id: TypeId, mock: impl FnOnce(I) -> O) {
        let mock = Box::new(mock) as Box<dyn FnOnce(_) -> _>;
        let mock = std::mem::transmute(mock);
        self.one_time_mocks.insert(id, mock);
    }

    pub unsafe fn call_mock<I, O>(&mut self, id: &TypeId, input: I) -> Option<O> {
        let mock = self.one_time_mocks.remove(&id)?;
        let mock: Box<dyn FnOnce(I) -> O> = std::mem::transmute(mock);
        Some(mock(input))
    }

    pub fn mock_once_safe<I: 'static, O: 'static>(
        &mut self,
        id: TypeId,
        mock: impl FnOnce(I) -> O + 'static,
    ) {
        let mock = |input: Box<dyn Any>| {
            let input = *(input.downcast().unwrap());
            let output = mock(input);
            Box::new(output) as Box<dyn Any>
        };
        self.safe_one_time_mocks.insert(id, Box::new(mock));
    }

    pub fn safe_call_mock<I: 'static, O: 'static>(&mut self, id: &TypeId, input: I) -> Option<O> {
        let mock = self.safe_one_time_mocks.remove(&id)?;
        let output = mock(Box::new(input) as Box<dyn Any>);
        Some(*(output.downcast().unwrap()))
    }
}
