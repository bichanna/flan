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

Flan is an acronym for "**F**unctional **LAN**guage." Although Flan has some functional features, it's expression oriented and not purely functional.

Also, I happen to love flan - a delicious custard dessert topped with caramel sauce.

## Snippets
Both the runtime (VM and standard library) and the compiler are only partially implemented, but here are some Flan code:
```javascript
i{iter, each, println} := import("std", "fmt")

names := ["Nobu", "Sol", "Damian", "Thomas", "Diego"]

names |> iter() |> each() ~ (name)
    println("Hello, {{}}!", name)
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

## Contents
This repository contains the core components of the language, including:
 - Compiler: converts text-based source code into a bytecode representation.
 - Runtime: executes the compiled bytecode and also provides built-in functions and standard library.

## Contribution
Bug reports and contributions are always welcome!
