# `stubby` - stubbing that doesn't hurt

A tiny stubbing library that even your IDE can understand!

## Why stub?

Stolen from [Wikipedia](https://en.wikipedia.org/wiki/Mock_object):

> In a unit test, mock objects can simulate the behavior of complex, real objects and are therefore useful when a real object is impractical or impossible to incorporate into a unit test.
> If an object has any of the following characteristics, it may be useful to use a mock object in its place:
>* the object supplies non-deterministic results (e.g. the current time or the current temperature); 
>* it has states that are difficult to create or reproduce (e.g. a network error); 
>* it is slow (e.g. a complete database, which would have to be prepared before the test); 
>* it does not yet exist or may change behavior; 
>* it would have to include information and methods exclusively for testing purposes (and not for its actual task).

## Usage example

```rust
use stubby::*;

struct TestStruct(Option<StubbyState>);

impl TestStruct {
    fn foo(&self) -> i32 {
        stub_if_some!(&self.0);
        10
    }
}

fn main() {
    let ts = TestStruct(None);
    assert_eq!(ts.foo(), 10);
}

#[test]
fn demo() {
    let mut mock = StubbyState::default();
    mock.insert(fn_name!(TestStruct::foo), 15);
    let ts = TestStruct(mock.into());
    assert_eq!(ts.foo(), 15);
}
```

## Why is mocking/stubbing in Rust so difficult? (Comparison to [`mockall`](https://lib.rs/crates/mockall))

Mocking in Rust is difficult because strong typing and compiling to machine code don't give any flexibility to mess with data/behaviour, like you would have in a duck-typed language, or a language with some kind of interpreter.

Because of this, a common way for mocking libraries to work in Rust is by using a procedural macro to generate a new `MockFoo` from your `impl Foo` block, which has extra methods to allow you to customise return types and do fancy stuff.
Then, at compile time, you either have `#[cfg(not(test))] use lib::Foo` or `#[cfg(test)] use lib::MockFoo as Foo`, which works because the generated `MockFoo` provides all the same methods the real `Foo` does.
However, with both Rust Analyzer and the Rust plugin for Jetbrains IDEs, these compile-time conditional imports completely break auto-complete, type hinting, and sometimes even syntax highlighting.
Plus, even where you're not using conditional imports, macro expansion for auto-complete/predictions often is less-well supported, especially with very complex generated types.
This means that while your mock usage has zero overhead during runtime, it has a big overhead during the most important phase: develop-time.

## How `stubby` works, and why it's more seamless

`stubby` is designed to avoid the pitfalls of conditional imports and procedural macros.
It does this by instead storing mocking behaviour as an attribute of the struct you want to mock, instead of creating an entirely new struct.
Avoiding procedural macros means slightly more boilerplate, though thanks to `stub_if_some!` that's usually only a single line per method.
`stubby` still has zero cost when compiled outside of `#[cfg(test)]` by replacing its state with `()`, but it still presents the exact same interface in order to give your IDE the easiest time of it.

As a bonus, `stubby` compiles far faster as it has zero dependencies, only uses declarative macros, and has 70 SLoC!

That having been said, it has only one of [`mockall`](https://lib.rs/crates/mockall)'s many features, and so if you're after a more feature-complete solution, check it out instead!
