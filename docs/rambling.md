
### reactivity (figured out)

ok nvm i have figured it out

so in fact we are just rerunning the whole clip code whenever state changes

difference is that we take note of which properties depend on other properties so we can know which ones won't be affected

```rs
clip complex_text(item: String, repetitions: Int) {
    let text_obj = text("Items: ");
    let full_text = "";
    for i in 0..repetitions {
        full_text += item + " ";
    }
    text_obj.text += full_text;
}
clip uses_complex_text() {
    let comp = complex_text("dog", 5);
    debug(comp.text); // should be `Items: dog dog dog dog dog `
    comp.repetitions = 3;
    debug(comp.text); // should now be `Items: dog dog dog `
}

// desugars to

// since neither are assigned, they don't depend on each other
clip complex_text(item: String use<>, repetitions: Int use<>) {
    ...
}
clip uses_complex_text() {
    let comp = complex_text("dog", 5);
    debug(comp.text); // should be `Items: dog dog dog dog dog `
    comp.repetitions = 3; // at this point `complex_text` is rerun
    debug(comp.text); // should now be `Items: dog dog dog `
}
```

so this means the following:

```rs
clip text(value: String, readonly chars: [Clip] use<value>) { ... }

clip uses_text() {
    let text = text("Hi mom");
    if text.chars.length() > 0 {
        text.x = 50; // this is legal, since `chars` does not depend on `display` 
                     // and so we can rerun `text` just fine (although like in 
                     // this case we don't even need to rerun it)
    }
    if text.chars.length() > 0 {
        text.value = ""; // not legal! `text.chars` depends on `text.value`
    }
}
```

### reactivity

ok so realistically honestly i should probably do reactivity by checking which parameters a clip depends on and then just. rerunning the function when those parameters are modified outside of it. that makes the most sense i think

nvm that would make some relationships impossible to do like this
```rs
clip thing(msg: string) {
    let text = text(msg);

    // This realistically should not have issues since `chars` does not depend 
    // on position, but if i naïvely just rerun clip functions then this would 
    // result in a circular dependency
    if text.chars.length() > 5 {
        text.x = 50;
    }
}
```

so maybe back to the "rewrite clip functions as reactive structs" idea

so then thinking about this
```rs
clip complex_text(item: String, repetitions: Int) {
    let text_obj = text("Items: ");
    let full_text = "";
    for i in 0..repetitions {
        full_text += item + " ";
    }
    text_obj.text += full_text;
}

clip uses_complex_text() {
    let comp = complex_text("dog", 5);
    debug(comp.text); // should be `Items: dog dog dog dog dog `
    comp.repetitions = 3;
    debug(comp.text); // should now be `Items: dog dog dog `
}

// what does it desugar to?
struct complex_text {
    item: String;
    repetitions: Int;
    priv full_text: String = {
        let v = "";
        for i in 0..self.repetitions {
            v += self.item + " ";
        }
        v
    }
    priv text_obj: String = {
        let v = "Items: ";
        v += full_text;
        v
    },
}
```

### await

```rs
clip thing(msg: string) {
    let text = text(msg);
    await 5s;
}

clip video() {
    await thing("Hi mom!");
}
// desugars to
clip video() {
    let temp = thing("Hi mom!");
    await temp.DURATION;
    drop(temp);
}
```

### escaping references out of a clip

```rs
struct EscapeHatch {
    target: (ref Clip)?;
}
clip this_is_a_clip(hatch: ref EscapeHatch) {
    let text = text("I'm finna escape");
    hatch.target = text;
    await 5s;
    // At this point the clip ends and the reference to text should be 
    // invalidated
}
clip muhaha() {
    let hatch = EscapeHatch {
        target: none;
    }
    await this_is_a_clip(hatch);
    hatch.target; // this would now result in the equivalent of a use-after-free
}
```

after monomorphization:

```rs
struct EscapeHatch_muhaha_0 {
    target: (ref Clip for muhaha)?;
}
clip this_is_a_clip(hatch: ref EscapeHatch_muhaha_0 for muhaha::this_is_a_clip) {
    let text = text("I'm finna escape");
    hatch.target = ref text for muhaha::this_is_a_clip; // Error here: reference should live for muhaha, 
                                                        // but only lives for muhaha::this_is_a_clip
    await 5s;
}
clip muhaha() {
    let hatch = EscapeHatch_muhaha_0 {
        target: none;
    }
    await this_is_a_clip(hatch);
    hatch.target; // We have errored previously so no longer possible :3
                  // but if this line did not exist then `for muhaha` == `for muhaha::this_is_a_clip` 
                  // and this whole code would be legal actually
}
```

### arrow functions

```rs
clip this_is_an_arrow_function() {
    let a = 5;
    let func = () => {
        a = 7;
    };
    func();
    // Now it should be clear that at this point a = 7;
}

// it desugars to

struct ArrowFunction0Captures {
    a: ref int;
}
function arrow_function_0(captures: ArrowFunction0Captures) {
    captures.a = 7;
}

clip this_is_an_arrow_function() {
    let a = 5;
    let captures = ArrowFunction0Captures {
        a: a
    };
    arrow_function_0(captures);
}
```

### rambling about lifetimes

so if i do this

```rs
clip basic_lifetime() {
    let video = short_video_clip("clip.mp4");
    await video;
    // At this point video is over and obviously should no longer be available
}
```

now consider that we can manipulate frames after the fact and do this

```rs
effect freeze_frame(target: ref Clip, const for_time: duration) {
    target.frame = self.ENTRY_FRAME;
    await for_time;
}

clip nonbasic_lifetime() {
    let video = short_video_clip("clip.mp4");
    // Aand actually there is no issue since this reference should just extend 
    // the lifetime of the clip by the specified duration (however, how do we 
    // communicate that to the type system? do we just naïvely extend clip 
    // lifetimes by reference lifetimes?)
    await video.freeze_frame(5s);
}
```

OK IDEA:

```rs
clip basic_lifetime() {
    let video = short_video_clip("clip.mp4");
    await video; // await moves the thing passed into it and drops it afterwards
}

clip nonbasic_lifetime() {
    let video = short_video_clip("clip.mp4");
    await video.freeze_frame(5s);

    // Here we can still use video all we want. If we want to play it to the end:
    await video; // and this then moves it

    // Or if want to do something else with it, we can. Or we can explicitly 
    // stop it and remove it via drop:
    drop(video);
}
```

ok yeah this works, although some silly things are now possible

```rs
clip this_should_lifetime_error() {
    let video = short_video_clip("clip.mp4"); // assume clip lasts 4s
    video.freeze_frame(5s);
    await video; // so here this just waits 4s of the video being frozen? because 
                 // how would it know that freeze_frame could extend the clips 
                 // lifetime by 5s
                 // i mean ig that makes sense
}
```

### oh dear

i have just realized something tragic

consider this
```rs
clip basic_text_animation(text: string) {
    let label = std::clips::text(text);

    for char in label.chars {
        char.scale_to(0%)
        // This clip cannot be awaited because it depends on runtime information
        clip {
            char.scale_to(100%);
            await 1s;
        }
    }
    await 5s;
}
```

this is quite undesirable since it does mean that something like the following is impossible:

```rs
effect animate_in_text(text: ref TextClip) {
    for char in text.chars {
        char.scale_to(0%);
    }
    for char in text.chars {
        char.scale_to(100%);
        await 0.2s; // Oops! Await depends on non-const data!
    }
}
clip my_animation() {
    let text = text("Hi, mom!");

    await animate_in_text(text);
    // Do something right after the animation finishes
}
```

but i think we'll just have to live with this since i think allowing arbitrary runtimes would be a terrible idea for optimization reasons

### v1

Vid has four kinds of functions: `clip`, `effect`, `function`, and `const function`. However, in reality, all of the other ones are just syntactic sugar over `function`.

A `clip` desugars to this:

```rs
clip thing(name: string, age: float) {
    let label = text("Hello, {name}! Your age is {age}!");
    for _ in 0..3 {
        age = age + 1;
        await 1s;
    }
    await 5s;
}

// Desugars to (after const loop unrolling)
clip thing(name: string, age: float) {
    let label = text("Hello, {name}! Your age is {age}!");
    age = age + 1;
    await 1s;
    age = age + 1;
    await 1s;
    age = age + 1;
    await 1s;
    await 5s;
}

function thing(name: string, age: float, frame: int) -> <UniqueTypeForClipThing> {
    return {
        // Basic clip properties
        x: float;
        y: float;
        scale: float;
        // etc.

        // Current global frame (at least according to this clip)
        frame: int;
        // The frame this clip was entered on
        const ENTRY_FRAME: int;
        // Total length of this clip
        const DURATION: duration;

        // Properties of this clip
        name: string;
        age: float = {
            if self.frame > self.ENTRY_FRAME + 1s {
                age + 1
            }
            else if self.frame > self.ENTRY_FRAME + 2s {
                age + 2
            }
            else if self.frame > self.ENTRY_FRAME + 2s {
                age + 2
            }
        }

        private label = text("Hello, {name}! Your age is {age}!");
        
        __children: [Clip];
    }
}
```

### v0

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
