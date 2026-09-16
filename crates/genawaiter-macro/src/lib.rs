#![warn(future_incompatible, rust_2018_compatibility, rust_2018_idioms, unused)]
#![warn(clippy::cargo, clippy::pedantic)]
#![cfg_attr(feature = "strict", deny(warnings))]

#[macro_export]
#[cfg(feature = "proc_macro")]
macro_rules! stack_let_gen {
    ($name:ident, $body:expr $(,)?) => {
        ::genawaiter::stack::let_gen_using!($name, ::genawaiter::stack_producer!($body),);
    };
}

#[macro_export]
macro_rules! stack_let_gen_using {
    ($name:ident, $producer:expr $(,)?) => {
        // Safety: The goal here is to ensure the safety invariants of `Gen::new`, i.e.,
        // the lifetime of the `Co` argument (in `$producer`) must not outlive `shelf`
        // or `generator`.
        //
        // We create two variables, `shelf` and `generator`, which cannot be named by
        // user-land code (because of macro hygiene). Because they are declared in the
        // same scope, and cannot be dropped before the end of the scope (because they
        // cannot be named), they have equivalent lifetimes. The type signature of
        // `Gen::new` ties the lifetime of `co` to that of `shelf`. This means it has
        // the same lifetime as `generator`, and so the invariant of `Gen::new` cannot
        // be violated.
        let mut shelf = ::genawaiter::stack::Shelf::new();
        let mut generator = unsafe { ::genawaiter::stack::Gen::new(&mut shelf, $producer) };
        let $name = &mut generator;
    };
}

#[macro_export]
#[cfg(feature = "proc_macro")]
macro_rules! rc_gen {
    ($body:expr) => {
        ::genawaiter::rc::Gen::new(::genawaiter::rc_producer!($body))
    };
}

/// mmlx: `gen!` yields the boxed stream directly, so song files end
/// with a bare `gen!({...})` — no `Box::new`/`into_iter()` at use sites.
/// (Diverges from upstream, where `gen!` returns the bare `Gen`; the
/// iterator type is unnameable, so boxing anywhere else needs the same
/// adapter with worse readability.)
#[macro_export]
#[cfg(feature = "proc_macro")]
macro_rules! sync_gen {
    ($body:expr) => {
        Box::new(::genawaiter::sync::Gen::new(::genawaiter::sync_producer!($body)).into_iter())
    };
}
