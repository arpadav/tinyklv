This isn’t a bug in your code. It’s a Kani/CBMC limitation surfacing.

What you’re seeing:

* `Unwinding loop ... Vec::<u8>::extend_with`
* `iteration 459`, `460`, …
* `aborting path on assume(false)`

That pattern = **loop unwinding bound hit**.

### What’s actually happening

Kani lowers Rust → CBMC. CBMC handles loops by **bounded unwinding**.

When the bound is exceeded, it inserts:

```
assume(false)
```

which kills that execution path. ([Amazon Science][1])

So your trace literally means:

> “I had to stop exploring because the loop ran more times than I’m allowed to model.”

`Vec::extend_with` is especially bad because:

* it may loop proportional to length/capacity
* symbolic sizes → Kani assumes it could be arbitrarily large

---

### Why it keeps increasing (459 → 460 → …)

You’re hitting the **global unwind bound**, not a real loop exit condition.

Kani is trying deeper unwinds → still not enough → keeps failing.

---

### Fixes (practical)

#### 1. Increase unwind bound

```
cargo kani --unwind 600
```

or higher.

But this scales poorly.

---

#### 2. Constrain sizes (best fix)

Your harness likely allows arbitrary-length vectors.

Force small bounds:

```rust
let len: usize = kani::any();
kani::assume(len < 32);
```

Or for Vec:

```rust
let mut v = Vec::new();
let n: usize = kani::any();
kani::assume(n < 32);

for _ in 0..n {
    v.push(kani::any());
}
```

---

#### 3. Avoid std `Vec` growth paths

`extend_with` internally reallocates + loops.

Instead:

* pre-allocate
* use fixed-size arrays if possible

---

#### 4. Stub or abstract

If you don’t care about Vec internals:

```rust
#[kani::stub(std::vec::Vec::extend_with, your_stub)]
```

---

#### 5. Use `--unwinding-assertions` awareness

Kani enables them → ensures soundness but causes these failures instead of silent truncation. ([Amazon Science][1])

---

### Quick diagnosis checklist

If you see:

* `extend_with`
* high iteration counts
* `assume(false)`

→ it is **almost always missing bounds on symbolic size**

---

### Minimal actionable fix

Add:

```rust
kani::assume(len < 32);
```

(or even `< 8`) wherever size influences loops.

---

If you paste your harness, I can point exactly where the unconstrained growth is coming from.

[1]: https://assets.amazon.science/cb/d2/3a8470634d0a97ea4e3e62e02b7b/verifying-dynamic-trait-objects-in-rust.pdf?utm_source=chatgpt.com "Verifying Dynamic Trait Objects in Rust"

