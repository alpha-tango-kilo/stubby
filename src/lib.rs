#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use std::fmt;

/// Gets the name of the current or given function as a [`StubbyName`]
#[macro_export]
macro_rules! fn_name {
    () => {{
        // Hack from https://docs.rs/stdext/0.2.1/src/stdext/macros.rs.html#61-72
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        // `3` is the length of the `::f`.
        $crate::StubbyName::__macro_new(&name[..name.len() - 3])
    }};
    ($fn:expr) => {{
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        $crate::StubbyName::__macro_new(type_name_of($fn))
    }};
}

/// Use at the start of a method to return a stub when in `#[cfg(test)]`, **if one is found**.
/// If no stub is set, then the method executes normally
///
/// For unconditional stubbing, use [`stub!`]
///
/// ```no_run
/// use stubby::*;
///
/// struct FizzBuzzer(StubbyState);
///
/// impl FizzBuzzer {
///     fn start(&self) -> String {
///         stub_if_found!(&self.0);                 // 👈 here!
///         String::from("this if no stub provided")
///     }
///
///     fn next(&self) -> String {
///         stub_if_found!(&self.0);                 // 👈 here!
///         String::from("this if no stub provided")
///     }
/// }
///
/// #[test]
/// fn fizzbuzzer_start() {
///     let mut state = StubbyState::default();
///     state.insert(fn_name!(FizzBuzzer::start), String::from("stub response!"));
///     let fizzbuzzer = FizzBuzzer(state);
///     assert_eq!(fizzbuzzer.start(), String::from("stub response!"));
///     assert_eq!(fizzbuzzer.next(), String::from("this if no stub provided"));
/// }
/// ```
#[macro_export]
macro_rules! stub_if_found {
    ($mock:expr) => {
        #[cfg(test)]
        {
            if let Some(t) = $mock.get(fn_name!()) {
                return t;
            }
        }
    };
}

/// Use at the start of a method to return a stub when in `#[cfg(test)]`
///
/// # Panics
///
/// If no stub is set
///
/// For stubbing sometimes, use [`stub_if_found!`]
///
/// ```no_run
/// use stubby::*;
///
/// struct FizzBuzzer(StubbyState);
///
/// impl FizzBuzzer {
///     fn start(&self) -> String {
///         stub!(&self.0);                 // 👈 here!
///         String::from("this if no stub provided")
///     }
///
///     fn next(&self) -> String {
///         stub!(&self.0);                 // 👈 here!
///         String::from("this if no stub provided")
///     }
/// }
///
/// #[test]
/// fn fizzbuzzer_start() {
///     let mut state = StubbyState::default();
///     state.insert(fn_name!(FizzBuzzer::start), String::from("stub response!"));
///     let fizzbuzzer = FizzBuzzer(state);
///     assert_eq!(fizzbuzzer.start(), String::from("stub response!"));
///     // ⚠ Would panic:
///     // assert_eq!(fizzbuzzer.next(), String::from("this if no stub provided"));
/// }
/// ```
#[macro_export]
macro_rules! stub {
    ($mock:expr) => {
        #[cfg(test)]
        {
            let name = fn_name!();
            $mock
                .get(name)
                .unwrap_or_else(|| panic!("no stub configured for {name}"))
        }
    };
}

/// An interned string type for holding function names.
/// Returned by [`fn_name!`]
///
/// Prevents trying to store mocks in [`StubbyState`] by just giving the method name as a `&str`
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct StubbyName(&'static str);

impl StubbyName {
    #[doc(hidden)]
    pub fn __macro_new(name: &'static str) -> Self {
        Self(name)
    }
}

impl fmt::Display for StubbyName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[cfg(not(test))]
type StubbyStateInner = ();
#[cfg(test)]
type StubbyStateInner =
    std::collections::HashMap<StubbyName, Box<dyn std::any::Any>>;

/// Stores stub information.
/// Initialise with [`StubbyState::new`] or [`StubbyState::default`]
///
/// In `#[cfg(test)]`, contains a map of function names to return types (as `Box<dyn Any>`).
///
/// In `#[cfg(not(test))`, contains `()`.
#[derive(Default)]
pub struct StubbyState(StubbyStateInner);

impl StubbyState {
    /// Creates a new, empty `StubbyState`
    pub fn new() -> Self {
        StubbyState::default()
    }

    /// Adds a new function to be stubbed with the given `obj`.
    /// Repeated `insert`s will overwrite existing entries.
    ///
    /// Note: no type checking is done to ensure `obj` is the correct return type for the function.
    /// This will lead to a panic when the value is accessed.
    ///
    /// ```no_run
    /// use stubby::*;
    ///
    /// struct Foo(StubbyState);
    ///
    /// impl Foo {
    ///     fn return_four(&self) -> &'static str {
    ///         stub_if_found!(&self.0);
    ///         "four"
    ///     }
    /// }
    ///
    /// let mut stubs = StubbyState::new();
    /// // Correct
    /// stubs.insert(fn_name!(Foo::return_four), "five");
    /// // Bad, will cause panic
    /// stubs.insert(fn_name!(Foo::return_four), 5);
    /// ```
    ///
    /// There is also no type inference, so be careful with numeric types:
    ///
    /// ```no_run
    /// # use stubby::*;
    ///
    /// struct Foo(StubbyState);
    ///
    /// impl Foo {
    ///     fn return_four(&self) -> u32 {
    ///         stub_if_found!(&self.0);
    ///         4
    ///     }
    /// }
    ///
    /// let mut stubs = StubbyState::new();
    /// // Bad! 4 is considered an i32
    /// stubs.insert(fn_name!(Foo::return_four), 5);
    /// // Good! Type specified using generic parameter
    /// stubs.insert::<u32>(fn_name!(Foo::return_four), 5);
    /// // Good! Numeric type specified in literal
    /// stubs.insert(fn_name!(Foo::return_four), 5u32);
    /// ```
    #[cfg(not(test))]
    #[allow(unused)]
    pub fn insert<T: Clone + 'static>(&mut self, name: StubbyName, obj: T) {
        panic!("should not have stubs being used outside of #[cfg(test)]");
    }
    #[cfg(test)]
    pub fn insert<T: Clone + 'static>(&mut self, name: StubbyName, obj: T) {
        self.0.insert(name, Box::new(obj));
    }

    /// Fetches the value stored in the `StubbyState` for the given name.
    ///
    /// Usually you won't need to call this function directly, instead preferring [`stub_if_found!`] or [`stub!`]
    ///
    /// # Panics
    ///
    /// If the `name` isn't stored, or if the value associated with `name` isn't a `T`
    ///
    #[cfg(not(test))]
    #[allow(unused)]
    pub fn get<T: Clone + 'static>(&self, name: StubbyName) -> Option<T> {
        panic!("should not have stubs being used outside of #[cfg(test)]");
    }
    #[cfg(test)]
    pub fn get<T: Clone + 'static>(&self, name: StubbyName) -> Option<T> {
        self.0.get(&name).map(|any| {
            any.downcast_ref::<T>()
                .unwrap_or_else(|| panic!("incorrect type supplied for {name}"))
                .clone()
        })
    }
}
