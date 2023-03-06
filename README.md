<div align="center">
    <h1>The Impala Programming Language</h1>
    |
    <a href="https://bichanna.github.io/impala-book/">Doc</a>
    |
</div><br>

<div align="center">
</div>

**IMP**ure function**A**l **LA**nguage is a high-level, dynamically- and strongly-typed, functional programming language that runs on a virtual machine.
**Impala** is my highschool Computer Science project written in Rust.

I'm sorry for the ugly and inefficient and awful code. That's because Impala is my first serious project.

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
range := import("range")

func fizzbuzz(n) match [n % 3, n % 5] {
    [0, 0] -> "FizzBuzz",
    [0, _] -> "Fizz",
    [_, 0] -> "Buzz",
    _ -> string(n),
}

range.range(1, 10001) |> range.each() <| func(n) {
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
