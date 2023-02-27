<div align="center">
    <h1>The Feo Programming Language</h1>
    |
    <a href="https://bichanna.github.io/feo-book/">Doc</a>
    |
</div><br>

<div align="center">
</div>

**Feo** is a dynamically- and strongly-typed, functional, simple programming language with automatic momery management.
**Feo** is my highschool Computer Science project written in Rust.

I'm sorry for the ugly and inefficient and awful code. That's because Feo is my first serious project.

## Examples

```js
// printing out names
{println: fprintln} := import("fmt")
{each: each} := import("std")

names := ["Nobu", "Sol", "Thomas", "Damian", "Ryan", "Zen", "Esfir"]
each(names) <| func(name) fprintln("Hello, {}!", name)
```

```js
// fizzbuzz
std := import("std")

func fizzbuzz(n) match [n % 3, n % 5] {
    [0, 0] -> "FizzBuzz",
    [0, _] -> "Fizz",
    [_, 0] -> "Buzz",
    _ -> string(n),
}

std.range(1, 101) |> std.each() <| func(n) {
    println(fizzbuzz(n))
}
```

```js
std := import("std")
fmt := import("fmt")

// fibonacci
func fib(n) n <= 1 : n ? fib(n-1) + fib(n-2)

std.range(0, 10) |> std.each() <| func(i) {
    fmt.println("fib({}) = {}", i, fib(i))
}
```
