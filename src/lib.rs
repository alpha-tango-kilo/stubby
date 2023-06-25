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
///     assert_eq!(fn_name!(), "fizz");
///     assert_eq!(fn_name!(buzz), "buzz");
///     assert_eq!(fn_name!(FizzBuzzer::run), "FizzBuzzer::run");
/// }
///
/// fn buzz() {}
///
/// struct FizzBuzzer;
///
/// impl FizzBuzzer {
///     fn run() {}
/// }
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
/// struct FizzBuzzer(MockState);
///
/// impl FizzBuzzer {
///     fn start(&self) -> String {
///         stub_if_some!(&self.0);
///         String::from("this when not testing")
///     }
/// }
///
/// #[test]
/// fn fizzbuzzer_start() {
///     let mut state = MockState::default();
///     state.insert(fn_name!(FizzBuzzer::start), String::from("stub response!"));
///     assert_eq!(FizzBuzzer(state).start(), String::from("stub response!"));
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
type MockStateInner = ();
#[cfg(test)]
type MockStateInner = std::collections::HashMap<&'static str, Box<dyn any::Any>>;

#[derive(Default)]
pub struct MockState(MockStateInner);

impl MockState {
    #[cfg(test)]
    pub fn insert<T: Clone + 'static>(&mut self, name: &'static str, obj: T) {
        self.0.insert(name, Box::new(obj));
    }
    #[cfg(not(test))]
    pub fn insert<T: Clone + 'static>(&mut self, _name: &'static str, _obj: T) {
        panic!("should not have mocks being used outside of #[cfg(test)]");
    }

    #[cfg(test)]
    pub fn get<T: Clone + 'static>(&self, name: &'static str) -> T {
        self.0
            .get(&name)
            .unwrap_or_else(|| panic!("no mock configured for {name}"))
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("incorrect type supplied for {name}"))
            .clone()
    }
    #[cfg(not(test))]
    pub fn get<T: Clone + 'static>(&self, _name: &'static str) -> T {
        panic!("should not have mocks being used outside of #[cfg(test)]");
    }
}
