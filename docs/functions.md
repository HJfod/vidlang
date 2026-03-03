
Vid has four kinds of functions: `clip`, `effect`, `function`, and `const function`. Although, in reality, all `const function`s can be written as normal `function`s, and all `clip`s can be written as `effect`s, so there are really two types of functions: `effect`s and `function`s. These correspond pretty much to `async` functions and normal functions in other programming languages; in other words, `effect`s are functions that have an associated *duration*, and can be `await`ed. In constrast, normal `function`s always return their result immediately.

`const function` is a function that can only be run at const time. In other words, it is syntactic sugar over a function whose parameters are all const. For example: 

```vid
const function add(a: int, b: int) -> int {
    a + b
}
// Is equivalent to
function add(const a: int, const b: int) -> int {
    a + b
}
// Except that with const functions, the compiler can issue more exact errors 
// if you try to call them with non-const data.
```

`clip` is essentially just syntactic sugar over `effect`. For example:

```vid
clip thing(name: string, age: float) {
    let label = text("Hello, {name}! Your age is {age}!");
    await 5s;
}
// Is just syntactic sugar over
effect thing(name: string, age: float) -> Clip + { name: string, age: float } {
    return clip {
        property name: string = thing::name;
        property age: float = thing::age;
        let label = text("Hello, {name}! Your age is {age}!");
        await 5s;
    }
}
```

Clips correspond to essentially classes/structs in other languages.
