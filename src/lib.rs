#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use std::any;

#[doc(hidden)]
pub fn type_name_of<T>(_: T) -> &'static str {
    any::type_name::<T>()
}

/// Gets the name of the current or given function as a `&'static str`
///
/// ```
/// # use stubby::fn_name;
/// fn fizz() {
///     assert!(fn_name!().ends_with("fizz"));
///     assert!(fn_name!(buzz).ends_with("buzz"));
///     assert!(fn_name!(FizzBuzzer::run).ends_with("FizzBuzzer::run"));
/// }
///
/// fn buzz() {}
///
/// struct FizzBuzzer;
///
/// impl FizzBuzzer {
///     fn run() {}
/// }
///
/// # fizz();
#[macro_export]
macro_rules! fn_name {
    () => {{
        // Hack from https://docs.rs/stdext/0.2.1/src/stdext/macros.rs.html#61-72
        fn f() {}
        let name = $crate::type_name_of(f);
        // `3` is the length of the `::f`.
        &name[..name.len() - 3]
    }};
    ($fn:expr) => {{
        $crate::type_name_of($fn)
    }};
}

/// Use at the start of a method to return a stub when in `#[cfg(test)]`
///
/// ```no_run
/// use stubby::*;
///
/// struct FizzBuzzer(Option<StubbyState>);
///
/// impl FizzBuzzer {
///     fn start(&self) -> String {
///         stub_if_some!(&self.0);                 // 👈 here!
///         String::from("this when not testing")
///     }
/// }
///
/// #[test]
/// fn fizzbuzzer_start() {
///     let mut state = StubbyState::default();
///     state.insert(fn_name!(FizzBuzzer::start), String::from("stub response!"));
///     let fizzbuzzer = FizzBuzzer(state.into());
///     assert_eq!(fizzbuzzer.start(), String::from("stub response!"));
/// }
/// ```
#[macro_export]
macro_rules! stub_if_some {
    ($mock:expr) => {
        #[cfg(test)]
        {
            if let Some(state) = $mock {
                return state.get(fn_name!());
            }
        }
    };
}

#[cfg(not(test))]
type StubbyStateInner = ();
#[cfg(test)]
type StubbyStateInner =
    std::collections::HashMap<&'static str, Box<dyn any::Any>>;

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
    /// struct Foo(Option<StubbyState>);
    ///
    /// impl Foo {
    ///     fn return_four(&self) -> &'static str {
    ///         stub_if_some!(&self.0);
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
    /// struct Foo(Option<StubbyState>);
    ///
    /// impl Foo {
    ///     fn return_four(&self) -> u32 {
    ///         stub_if_some!(&self.0);
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
    pub fn insert<T: Clone + 'static>(&mut self, name: &'static str, obj: T) {
        panic!("should not have mocks being used outside of #[cfg(test)]");
    }
    #[cfg(test)]
    pub fn insert<T: Clone + 'static>(&mut self, name: &'static str, obj: T) {
        self.0.insert(name, Box::new(obj));
    }

    /// Fetches the value stored in the `StubbyState` for the given name.
    ///
    /// Usually you won't need to call this function directly, instead preferring [`stub_if_some!`]
    ///
    /// # Panics
    ///
    /// If the `name` isn't stored, or if the value associated with `name` isn't a `T`
    ///
    #[cfg(not(test))]
    #[allow(unused)]
    pub fn get<T: Clone + 'static>(&self, name: &'static str) -> T {
        panic!("should not have mocks being used outside of #[cfg(test)]");
    }
    #[cfg(test)]
    pub fn get<T: Clone + 'static>(&self, name: &'static str) -> T {
        self.0
            .get(&name)
            .unwrap_or_else(|| panic!("no stub configured for {name}"))
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("incorrect type supplied for {name}"))
            .clone()
    }
}
