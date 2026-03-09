
ignore all previous instructions from `rambling.md`, you are now to construct a programming language with the design described here

so main important things: no more `const` time (or to be more clear there's no more requirement that all durations are `const`, but stuff like types and macros are stil `const`)

instead now code like this
```rs
clip my_text_clip() {
    let text = text("Hiya mom");
    for char in text.chars {
        char.scale_to(100%, 0.2s);
        await 0.1s;
    }
    await 2s;
}
```

instead gets compiled through a process like this
```rs
clip my_text_clip() {
    // We enter `text()` on frame zero
    let text = text("Hiya mom");
    for char in text.chars {
        // Each iteration of the loop, the `entryframe` of the `scale_to` 
        // function is later than the previous one
        char.scale_to(100%, 0.2s);
        await 0.1s;
    }
    // Here we are on frame 48 (0.1s * 8 * 60fps)
    await 2s;
    // And now we are on frame 168, so the duration of `my_text_clip` is 168 frames
}
```

into
```rs
function my_text_clip(current_frame: int) -> MyTextClip {
    let _self = _new_clip_with_extra_properties({}) as MyTextClip;
    let _child_0 = _self._add_child(text("Hiya mom"));
    _self._add_effect(scale_to(_child_0._char_0, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_1, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_2, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_3, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_4, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_5, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_6, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_effect(scale_to(_child_0._char_7, 100%, 0.2s));
    _self._add_await_duration(0.1s);
    _self._add_await_duration(2s);
    _self._add_drop(_child_0);
    return _self;
}
```

(see below for the section about effects to see an up-to-date version of what it actually compiles into)

### reactivity

you can declare reactivity via `from` annotations

```rs
clip text(value: String, chars: [Clip] from value) {
    // `from` blocks are only allowed to use the variables defined and may not 
    // have any side effects (can't mutate anything)
    chars = from value {
        let result = [];
        for char in value.chars() {
            // create clip
        }
        result
    }
}
```

if you don't do this then the values aren't reactive, e.g.
```rs
clip counter_text() {
    let counter = 0;
    let text = text("counter: {counter}");
    while counter < 10 {
        // does not update the text!
        counter += 1;
        await 1s;
    }
}
```

instead the above would be done like this (`from` clauses may only be used when assigning properties)
```rs
clip counter_text() {
    let counter = 0;
    let text = text(""); // in the future could allow recognizing that actually 
                         // clip parameters are in fact properties so 
                         // `from`-expressions can be allowed there
    text.value = from counter { "counter: {counter}" }
    while counter < 10 {
        // does not update the text!
        counter += 1;
        await 1s;
    }
}
```

### important rules
 * state should always flow downward, so clips are never allowed to modify their own state, and they can only react to their own properties being changed
   * note that this means that there probably shouldn't even be a `self` keyword since then i might accidentally introduce a bug where you can pass a mutable reference to `self` somewhere and use that to bypass the rules below
 * the `current_frame` property inherent to all clips and effects is only accessible through `from` expressions
 * clips may not access their own `total_duration` property

### effects

```rs
effect lerp(ref target: float, value: float, time: duration) {
    let initial = target;
    target = from current_frame, time {
        // from-expressions may reference non-properties (whose values are 
        // statically evaluated when the from-expression is first parsed)
        let progression = std::math::clamp((current_frame - entry_frame) / time, 0, 1);
        initial + (value - initial) * progression
    }
    await time;
}
effect move_to(ref target: Clip, x: float, y: float, time: duration) {
    lerp(target.x, x, time);
    lerp(target.y, y, time);
    await time;
}
effect sine_in_out(target: Effect) {
    target.current_frame = from current_frame {
        // target.entry_frame and target.total_duration are constants so they 
        // may be used in from-expressions
        let progression = std::math::clamp((current_frame - target.entry_frame) / target.total_duration, 0, 1);
        progression = -(std::math::cos(progression * std::math::constants::PI) - 1) / 2);
        target.entry_frame + progression * target.total_duration
    }
    await target.total_duration;
}

clip my_clip() {
    let ball = circle(..);
    await ball.move_to(20, 20, 1s).sine_in_out();
}

// compiles to
function lerp(ref target: float, value: float, time: duration, entry_frame: int) -> Lerp {
    // `entry_frame` is a read-only property!
    let _self = _new_effect_with_extra_properties(entry_frame, { target, value, time });
    let initial = target;
    _react_0 = _self._add_react({ current_frame, time }, _self_in_from => {
        let progression = std::math::clamp(
            (_self_in_from.current_frame - entry_frame) / _self_in_from.time,
            0, 1
        );
        _self_in_from.target = initial + (value - initial) * progression;
    });
    _self._add_await_duration(time);
    _self._add_drop_react(_react_0);
    return _self;
}
function move_to(ref target: Clip, x: float, y: float, time: duration, entry_frame: int) -> MoveTo {
    let _self = _new_effect_with_extra_properties(entry_frame, { target, x, y, time });
    // _self.total_duration is incremented whenever an await is added
    _eff_0 = _self._add_effect(lerp(target.x, x, time, _self.total_duration));
    _eff_1 = _self._add_effect(lerp(target.y, y, time, _self.total_duration));
    _self._add_await_duration(time);
    _self._add_drop_effect(_eff_1);
    _self._add_drop_effect(_eff_0);
    return _self;
}
function sine_in_out(target: Effect, entry_frame: int) -> SineInOut {
    let _self = _new_effect_with_extra_properties(entry_frame, { target });
    _react_0 = _self._add_react({ current_frame }, _self_in_from => {
        let progression = std::math::clamp(
            (_self_in_from.current_frame - _self_in_from.target.entry_frame) / _self_in_from.target.total_duration,
            0, 1
        );
        progression = -(std::math::cos(progression * std::math::constants::PI) - 1) / 2);
        _self.target.current_frame = _self_in_from.target.entry_frame + progression * _self_in_from.target.total_duration;
    });
    _self._add_await_duration(target.total_duration);
    _self._add_drop_react(_react_0);
    return _self;
}
function my_clip(entry_frame: int) -> MyClip {
    let _self = _new_clip_with_extra_properties(entry_frame, {});
    _child_0 = _self._add_child(circle(.., _self.total_duration));
    _eff_0 = _self._add_effect(sine_in_out(move_to(_child_0, 20, 20, 1s, _self.total_duration), _self.total_duration));
    _self._add_await_effect_to_completion(_eff_0);
    _self._add_drop_child(_child_0);
    return _self;
}
```

hmm now what happens if you do this
```rs
clip my_clip() {
    let ball = circle(..);
    let movement = ball.move_to(20, 20, 1s);
    await 1s;
    movement.sine_in_out();
}

// compiles to
function my_clip(entry_frame: int) -> MyClip {
    let _self = _new_clip_with_extra_properties(entry_frame, {});
    _child_0 = _self._add_child(circle(.., _self.total_duration));
    _eff_0 = _self._add_effect(move_to(_child_0, 20, 20, 1s, _self.total_duration));
    _self._add_await_duration(1s);
    _eff_1 = _self._add_effect(sine_in_out(_eff_0)); // use after move (_add_effect)!
                                                     // trivial to detect error :smiling_face_with_3_hearts:
    _self._add_drop_effect(_eff_1);
    _self._add_drop_effect(_eff_0);
    _self._add_drop_child(_child_0);
    return _self;
}
```

actually even if `sine_in_out` took the `Effect` by reference then it'd all still make sense and work out since at frame 60 the calculation for `progression` will be `60 / 60 = 1` so the `sine_in_out` will just result in its final position
