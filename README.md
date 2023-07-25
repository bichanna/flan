<div align="center">
    <h1>The Flan Scripting Language</h1>
</div><br>
<div align="center">
</div>

[![Rust CI](https://github.com/bichanna/flan/actions/workflows/rust-ci.yml/badge.svg?event=push)](https://github.com/bichanna/flan/actions/workflows/rust-ci.yml)
![GitHub](https://img.shields.io/github/license/bichanna/flan)
[![Rust Version](https://img.shields.io/badge/rustc-1.56%2B-blue?logo=rust)](https://www.rust-lang.org/tools/install)

**🚧WARNING!! THIS PROJECT IS IN DEVELOPMENT🚧**
--------------------------------------------------------------------------------------------------------

Flan is an acronym for "**F**unctional **LAN**guage," and I happen to have a fondness for flan - a delicious custard dessert topped with caramel sauce.

Nothing works currently, but here are some examples:
```javascript
i{each, println} := import("std", "fmt")

names := ["Nobu", "Sol", "Damian", "Thomas", "Diego"]
names |> each() ~ (name) println("Hello, {{}}!", name)
```

```javascript
i{each, range} := import("std")

fn fizzBuzz(n) = match [n % 3, n % 5] with
    | [0, 0] -> "fizzbuzz"
    | [0, _] -> "fizz"
    | [_, 0] -> "buzz"
    | _ -> str(n)

range(101) |> each() <~ fizzBuzz(it) |> println()
```
