# Feo
The Feo Programming Language

Sorry for the shitty code :) This is my first serious Rust project.

The language will look like this:
```
let {printfln: println} = import("fmt");
let {each} = import("std");

let names = ["Nobu", "Sol", "Thomas", "Damian", "Ryan", "Zen", "Esfir"];
each(names) <| func(name) println("Hello, %{}!", name);
```
