# pyexc

An easy way to define Python exceptions.

```rust
use pyexc::PythonException;

#[derive(PythonException)]
pub enum MyExceptions {
    #[base(module = "errors")]
    #[format("Hello World!!")]
    Foo,
}
```
