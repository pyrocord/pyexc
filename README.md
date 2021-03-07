# pyexc

An easy way to define Python exceptions.

```rust
use pyexc::PythonException;

#[derive(PythonException)]
pub enum MyBaseException {
    #[base(module = "errors")]
    #[format("Hello")]
    Base,
    #[format("World")]
    Bar,
    #[format("!")]
    Baz,
}

// Inheritance is experimental!

#[derive(PythonException)]
pub enum MySubException {
    #[base(module = "other_errors", inherits = "errors.MyBaseException")]
    #[format("Error!")]
    BaseBase,
    #[format("SEGFAULT")]
    FooBar,
    #[format("Fatal!")]
    FooBaz,
}
```

Allows usage in ``Result``, as well as providing a generally Rust-like interface for Python exceptions.
