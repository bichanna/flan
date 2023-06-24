<div align="center">
    <h1>The Flan Scripting Language</h1>
</div><br>
<div align="center">
</div>

**🚧WARNING!! THIS LANGUAGE IS IN DEVELOPMENT🚧**
--------------------------------------------------------------------------------------------------------

Flan is a **F**unctional **LAN**guage

Nothing works currently, but here are some examples:
```javascript
i{each, println} := import(:std, :fmt)

names := ["Nobu", "Sol", "Damian", "Thomas", "Diego"]
names |> each() ~ (name) println("Hello, {{}}!", name)
```

```javascript
i{range, each} := import(:range)

fn fizzBuzz n = match [n % 3, n % 5] these
    [0, 0] -> "fizzbuzz",
    [0, _] -> "fizz",
    [_, 0] -> "buzz",
    _ -> str(n)

range(0, 100) |> each() <~ println() <| fizzBuzz(it)
```
