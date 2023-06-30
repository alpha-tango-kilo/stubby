#![cfg_attr(debug_assertions, warn(missing_docs))]
#![doc = include_str!("../README.md")]

use std::fmt;

/// Gets the name of the current or given function as a [`StubbyName`]
///
/// # Limitations
///
/// This macro relies on some hacks to try and ensure that the produced
/// [`StubbyName`] is the same both with parameter-less invocations in the
/// method, and named function invocations in your tests. These hacks are due to
/// limitations/discrepancies in the output of [`std::any::type_name`]
///
/// Things that won't work:
/// * Separate stubs for the same method with different generic parameters -
///   only one stub per method, regardless of generics
/// * Using [`stub!`] or [`stub_if_found!`] within a closure (you probably
///   shouldn't be doing this anyway) - the parent method's name will be
///   taken/used
#[macro_export]
macro_rules! fn_name {
    () => {{
        // Hack from https://docs.rs/stdext/0.2.1/src/stdext/macros.rs.html#61-72
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        // `3` is the length of the `::f`
        let name = &name[..name.len() - 3];
        // async generic functions end up with ::{{closure}} at the end, trim
        // that so it matches the other macro invocation
        let name = name.trim_end_matches("::{{closure}}");
        $crate::StubbyName::__macro_new(name)
    }};
    ($fn:expr) => {{
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of($fn);
        // Generic parameters show in this stubby form, but not the other, so
        // trim them here
        let has_generic = name.ends_with('>');
        let name = if has_generic {
            // FooBar<BazQux> -> FooBar
            let mut done = false;
            name.trim_end_matches(|c| {
                if c == '<' {
                    done = true;
                    return true;
                }
                !done
            })
        } else {
            name
        };
        $crate::StubbyName::__macro_new(name)
    }};
}

/// Use at the start of a method to return a stub when in `#[cfg(test)]`, **if
/// one is found**. If no stub is set, then the method executes normally
///
/// For unconditional stubbing, use [`stub!`]
///
/// ```no_run
/// # use stubby::*;
/// struct FizzBuzzer(StubbyState);
///
/// impl FizzBuzzer {
///     fn start(&self) -> String {
///         stub_if_found!(&self.0); // 👈 here!
///         String::from("this if no stub provided")
///     }
///
///     fn next(&self) -> String {
///         stub_if_found!(&self.0); // 👈 here!
///         String::from("this if no stub provided")
///     }
/// }
///
/// #[test]
/// fn fizzbuzzer_start() {
///     let mut state = StubbyState::default();
///     state.insert(
///         fn_name!(FizzBuzzer::start),
///         String::from("stub response!"),
///     );
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
            #[cfg(not(debug_assertions))]
            compile_error!(
                "stubby does not work in release mode, do not run tests with \
                 --release"
            );
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
            #[cfg(not(debug_assertions))]
            compile_error!(
                "stubby does not work in release mode, do not run tests with \
                 --release"
            );
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
/// Prevents trying to store mocks in [`StubbyState`] by just giving the method
/// name as a `&str`
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(Default))]
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

macro_rules! stubby_state {
    ( $( $t:path ),* ) => {
        use std::{
            cmp::Ordering,
            fmt,
            hash::{
                Hash,
                Hasher,
            },
        };

        use $crate::*;

        #[cfg(debug_assertions)]
        type StubbyFunction = Box<dyn Fn() -> Box<dyn std::any::Any> $(+ $t)*>;

        #[cfg(not(debug_assertions))]
        type StubbyStateInner = ();
        #[cfg(debug_assertions)]
        type StubbyStateInner =
            std::collections::BTreeMap<StubbyName, StubbyFunction>;

        /// Stores stub information.
        /// Initialise with [`StubbyState::new`] or [`StubbyState::default`]
        ///
        /// # Will `StubbyState` effect my `Eq`, `Ord`, `Hash`, `Clone` (etc.) derived traits?
        ///
        /// `StubbyState` tries to implement as many traits as possible in order
        /// for maximum compatibility with the struct/enum that you're adding
        /// `stubby` to. These trait implementations aim is to stay out of the
        /// way of derived implementations (i.e. `StubbyState` shouldn't affect
        /// `==`), which means that **most `StubbyState` trait implementations
        /// do not behave as they 'should'** (i.e. one `StubbyState` is always
        /// equal to another). Please read the documentation for the individual
        /// traits if you want to see how `StubbyState` behaves.
        ///
        /// Unfortunately, it's not possible for `StubbyState` to implement
        /// `Copy`, as in test mode it contains a
        /// [`BTreeMap`](std::collections::BTreeMap)
        ///
        /// # What actually is `StubbyState`, and how is it zero-sized in release mode?
        ///
        /// In `#[cfg(debug_assertions)]`, `StubbyState` contains a map of
        /// function names to boxed closures that return the stub values
        ///
        /// In `#[cfg(not(debug_assertions))`, `StubbyState` contains `()`
        ///
        /// `debug_assertions` has to be used as opposed to `test` because when
        /// running `cargo test`, dependencies are compiled in debug mode, not
        /// test mode
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
            /// Note: no type checking is done to ensure `obj` is the correct
            /// return type for the function. This will lead to a panic when
            /// the value is accessed.
            ///
            /// ```no_run
            /// # use stubby::*;
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
            /// There is also no type inference, so be careful with numeric
            /// types:
            ///
            /// ```no_run
            /// # use stubby::*;
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
            #[cfg(debug_assertions)]
            pub fn insert<T: Clone + $($t +)* 'static>(
                &mut self,
                name: StubbyName,
                obj: T,
            ) {
                self.0.insert(name, cloneable_into_stubby_function(obj));
            }
            #[cfg(not(debug_assertions))]
            #[allow(unused)]
            pub fn insert<T: Clone + $($t +)* 'static>(
                &mut self,
                name: StubbyName,
                obj: T,
            ) {
                panic!(
                    "should not have stubs being used outside of #[cfg(test)]"
                );
            }

            /// Adds a new function to be stubbed using the given
            /// function/closure. Typically used for return types that are
            /// `!Clone`
            ///
            /// ```no_run
            /// use stubby::*;
            ///
            /// struct NotClone(i32, StubbyState);
            ///
            /// impl NotClone {
            ///     fn sum_with(&self, a: i32) -> Self {
            ///         stub!(&self.1);
            ///         NotClone(self.0 + a, StubbyState::default())
            ///     }
            /// }
            ///
            /// #[test]
            /// fn not_clone_new() {
            ///     let mut stubs = StubbyState::new();
            ///     stubs.insert_with(fn_name!(NotClone::sum_with), || {
            ///         NotClone(0, StubbyState::default())
            ///     });
            ///
            ///     let not_clone = NotClone(10, stubs);
            ///     assert_eq!(
            ///         not_clone.sum_with(10),
            ///         NotClone(0, StubbyState::default())
            ///     );
            /// }
            /// ```
            #[cfg(debug_assertions)]
            pub fn insert_with<T: 'static>(
                &mut self,
                name: StubbyName,
                func: impl Fn() -> T + $($t +)* 'static,
            ) {
                self.0.insert(name, Box::new(move || Box::new(func())));
            }
            #[cfg(not(debug_assertions))]
            #[allow(unused)]
            pub fn insert_with<T: 'static>(
                &mut self,
                name: StubbyName,
                func: impl Fn() -> T + $($t +)* 'static,
            ) {
                panic!(
                    "should not have stubs being used outside of #[cfg(test)]"
                );
            }

            /// Fetches the value stored in the `StubbyState` for the given
            /// name.
            ///
            /// Usually you won't need to call this function directly, instead
            /// preferring [`stub_if_found!`] or [`stub!`]
            ///
            /// # Panics
            ///
            /// If the `name` isn't stored, or if the value associated with
            /// `name` isn't a `T`
            #[cfg(debug_assertions)]
            pub fn get<T: $($t +)* 'static>(&self, name: StubbyName) -> Option<T> {
                self.0.get(&name).map(|stubby_fn: &StubbyFunction| {
                    *stubby_fn().downcast::<T>().unwrap_or_else(|_| {
                        panic!("incorrect type supplied for {name}")
                    })
                })
            }
            #[cfg(not(debug_assertions))]
            #[allow(unused)]
            pub fn get<T: $($t +)* 'static>(&self, name: StubbyName) -> Option<T> {
                panic!(
                    "should not have stubs being used outside of #[cfg(test)]"
                );
            }
        }

        impl fmt::Debug for StubbyState {
            #[cfg(not(debug_assertions))]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.debug_tuple("StubbyState").finish()
            }

            #[cfg(debug_assertions)]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                // StubbyFunction is !Debug, so just substitute it for a fixed
                // string It's not like I had any useful type
                // information anyway lol
                let inner_debug = self
                    .0
                    .keys()
                    .map(|name| (name, "StubbyFunction"))
                    .collect::<std::collections::BTreeMap<_, _>>();
                f.debug_tuple("StubbyState").field(&inner_debug).finish()
            }
        }

        impl Clone for StubbyState {
            /// Returns an empty `StubbyState`
            fn clone(&self) -> Self {
                StubbyState::default()
            }
        }

        impl PartialEq for StubbyState {
            /// Always returns true
            fn eq(&self, _other: &Self) -> bool {
                true
            }
        }

        impl Eq for StubbyState {}

        impl Hash for StubbyState {
            /// Fixed hash
            fn hash<H: Hasher>(&self, state: &mut H) {
                state.finish();
            }
        }

        impl PartialOrd for StubbyState {
            /// `StubbyState`s are always equal in order
            fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
                Some(Ordering::Equal)
            }
        }

        impl Ord for StubbyState {
            /// `StubbyState`s are always equal in order
            fn cmp(&self, _other: &Self) -> Ordering {
                Ordering::Equal
            }
        }

        #[cfg(debug_assertions)]
        fn cloneable_into_stubby_function<T: Clone + $($t +)* 'static>(
            obj: T,
        ) -> StubbyFunction {
            Box::new(move || Box::new(obj.clone()))
        }
    };
}

#[deprecated(
    since = "0.2.4",
    note = "prefer to specify sync or unsync in imports"
)]
pub use unsync::StubbyState;

/// Single-threaded version of `StubbyState` that doesn't require `Send + Sync`
pub mod unsync {
    stubby_state!();
}

/// Thread-safe version of `StubbyState` that does require `Send + Sync`
pub mod sync {
    stubby_state!(Send, Sync);

    #[test]
    fn is_send_and_sync() {
        fn f<T: Send + Sync>(_: T) {}
        f(StubbyState::new());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        unsync::StubbyState,
        *,
    };

    #[test]
    fn not_cloneable() {
        #[derive(Debug, Eq, PartialEq)]
        struct NotClone;

        fn producer() -> NotClone {
            NotClone
        }

        let mut stubby = StubbyState::new();
        stubby.insert_with(StubbyName::default(), producer);

        stubby.get::<NotClone>(StubbyName::default()).unwrap();
    }

    #[test]
    fn generics() {
        #[allow(clippy::extra_unused_type_parameters)]
        fn f<T>() -> StubbyName {
            fn_name!()
        }

        // See Limitations section in `fn_name!`
        assert_eq!(fn_name!(f::<i32>), f::<i32>());
        assert_eq!(fn_name!(f::<i32>), f::<bool>());
    }

    #[test]
    fn closures() {
        #[allow(
            clippy::extra_unused_type_parameters,
            clippy::redundant_closure_call
        )]
        fn f() -> StubbyName {
            (|| fn_name!())()
        }

        // See Limitations section in `fn_name!`
        assert_eq!(fn_name!(f), f());
    }

    #[tokio::test]
    async fn async_with_lifetime_parameter() {
        async fn f<'a>() -> StubbyName {
            fn_name!()
        }

        // See Limitations section in `fn_name!`
        assert_eq!(fn_name!(f), f().await);
    }
}
