<div align="center">
    <h1>The Flan Programming Language</h1>
    <!-- |
    <a href="https://bichanna.github.io/flan-book/">Doc</a>
    | -->
</div><br>

<div align="center">
</div>

**WARNING!! THIS LANGUAGE IS IN DEVELOPMENT**
 --------------------------------------------------------------------------------------------------------
**F**unctional **LAN**guage is a high-level, dynamically- and strongly-typed, functional programming language that runs on a virtual machine.
**Flan** is my high school Computer Science project written in C++.

 -------------------------
Almost nothing works currently... See the TODO list [here](https://github.com/bichanna/flan/blob/master/TODO.md).

```js
std := import(:std)

names := ["Tsoding", "ThePrimeagen", "Fireship"]
std::for(names) ::: (name)
  std::println("I substribe to {{}}!", name)
```

```js
{println: println} := import(:std)

fn fib(n)
  if (n < 2)
    n
  else
    fib(n - 1) + fib(n - 2)

println("Result: {{}}", fib(12))
```
