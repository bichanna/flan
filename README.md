<div align="center">
    <h1>The Flan Scripting Language</h1>
</div><br>
<div align="center">
</div>

**🚧WARNING!! THIS PROJECT IS IN DEVELOPMENT🚧**
--------------------------------------------------------------------------------------------------------

Flan is an acronym for "**F**unctional **LAN**guage," while I happen to have a fondness for flan - a delicious custard dessert topped with caramel sauce.

Nothing works currently, but here are some examples:
```javascript
i{each, println} := import("std", "fmt")

names := ["Nobu", "Sol", "Damian", "Thomas", "Diego"]
names |> each() ~ (name) println("Hello, {{}}!", name)
```

```javascript
i{each} := import("std")

fn fizzBuzz(n) = where [n % 3, n % 5] match
    case [0, 0] -> "fizzbuzz",
    case [0, _] -> "fizz",
    case [_, 0] -> "buzz",
    _ then str(n)

(0..=100) |> each() <~ println() <| fizzBuzz(it)
```
