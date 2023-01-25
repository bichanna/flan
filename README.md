# Feo
**WIP: This project is still in early development.**

Feo is a dynamically- and strongly-typed, minimal, mildly-functional programming language that compiles to bytecode for the [Feo VM](https://github.com/bichanna/fvm)

The language looks like this:
```
// printing out names
{printfln: printfln} := import("fmt")
{each: each} := import("std")

names := ["Nobu", "Sol", "Thomas", "Damian", "Ryan", "Zen", "Esfir"]
each(names) <| func(name) printfln("Hello, %s!", name)


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

// fibonacci
func fib(n) n <= 1 : n ? fib(n-1) + fib(n-2)

std.range(0, 10) |> std.each() <| func(i) {
    printfln("fib(%s) = %s", i, fib(i))
}
```
