# Feo
The Feo Programming Language

Sorry for the shitty code :) This is my first serious Rust project.

The language will look like this:
```
{printfln: println} := import("fmt");
{each} := import("std");

names := ["Nobu", "Sol", "Thomas", "Damian", "Ryan", "Zen", "Esfir"];
each(names) <| func(name) println("Hello, %{}!", name);
```
