# pyexc

An easy way to define Python exceptions.

```rust
use pyexc::PythonException;

#[derive(PythonException)]
pub enum MyExceptions {
    #[base(module = "errors")]
    #[format("Hello")]
    Foo,
    #[format("World")]
    Bar,
    #[format("!")]
    Baz,
}

#[derive(PythonException)]
pub enum MySubExceptions {
    #[base(module = "other_errors", inherits = "MyExceptions.Foo")]
    #[format("Error!")]
    FooFoo,
    #[format("SEGFAULT")]
    FooBar,
    #[format("Fatal!")]
    FooBaz,
}
```
