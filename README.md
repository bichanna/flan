# Feo
The Feo Programming Language

Sorry for the shitty code :) This is my first serious Rust project.

The language will look like this:
```
{printfln: println} := import("fmt")
{each: each} := import("std")

names := ["Nobu", "Sol", "Thomas", "Damian", "Ryan", "Zen", "Esfir"]
each(names) <| func(name) println("Hello, %s!", name)


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
