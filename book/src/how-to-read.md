# How to read this book

## Bytes

```klv
j DE AD BE 00            // junk
s 53 45 4E 53 4F 52      // "SENSOR"
l 0A                     // length 10
    k 01 l 01 v 2A       // seq 42
    k 02 l 02 v 09 2E    // temp 2350
    u 07 01 FF           // unknown
```

| name | description | bytes |
|---|---|---|
| sentinel | marks the start of a packet | <span class="klv">s 53 45 4E 53 4F 52</span> |
| key | identifies the field | <span class="klv">k 01</span> |
| length | how many value bytes follow | <span class="klv">l 01</span> |
| value | the field's data | <span class="klv">v 2A</span> |
| junk | not part of a packet | <span class="klv">j DE AD</span> |
| unknown item | a key no field declares, with its length and value | <span class="klv">u 07 01 FF</span> |
| comment | a note, not part of the bytes | <span class="klv">// note</span> |

## Attributes

```rust,ignore
#[klv(
    sentinel = b"SENSOR",
    deny_unknown_keys,
    strict,
    // key(..), len(..) as on page 1.1
)]
struct Sensor {
    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        size(exact = 2),
    )]
    temp: u16,
}
```

| keyword | related to |
|---|---|
| <span class="klv-attr-k">key</span>, <span class="klv-attr-k">keylen</span>, <span class="klv-attr-k">oneof</span> | a key byte, e.g. <span class="klv">k 01</span> |
| <span class="klv-attr-l">len</span>, <span class="klv-attr-l">size</span>, <span class="klv-attr-l">exact</span>, <span class="klv-attr-l">lengthed</span>, <span class="klv-attr-l">implicit</span>, <span class="klv-attr-l">reserve</span> | a length byte, e.g. <span class="klv">l 01</span> |
| <span class="klv-attr-v">dec</span>, <span class="klv-attr-v">enc</span>, <span class="klv-attr-v">typ</span>, <span class="klv-attr-v">latebind</span>, <span class="klv-attr-v">default</span> | a value byte, e.g. <span class="klv">v 2A</span> |
| <span class="klv-attr-s">sentinel</span> | a sentinel byte, e.g. <span class="klv">s 53</span> |
| <span class="klv-attr-u">unknown</span>, <span class="klv-attr-u">deny_unknown_keys</span> | an unknown item, e.g. <span class="klv">u 07 01 FF</span> |
| <span class="klv-attr-sigil">*</span>, <span class="klv-attr-sigil">&amp;</span> | how a value is handed to its encoder |
| <span class="klv-attr-x">stream</span>, <span class="klv-attr-x">strict</span>, .. | none: plain text |

## Diffs

- every tutorial page starts from page 1.1's struct and builds off it
- a page shows only its change, as a diff: red lines go, green lines come
- apply the diff while you follow along, and you get the page's result

<details>
<summary>Full example: page 1.1</summary>

```rust
{{#include ../../examples/book/basics/book_basics_first_packet.rs:all}}
```

</details>

```diff
     #[klv(
         key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        dec = decber::ber_oid,
+        enc = *encber::ber_oid,
+        size(implicit),
     )]
     /// Temperature, in 0.01 °C
     temp: u16,
```

```klv
k 01 l 01 v 2A      // seq 42
k 02      v 92 2E   // 2350, no length
```

<details>
<summary>Full example: the change</summary>

```rust
{{#include ../../examples/book/sizes/book_sizes_implicit.rs}}
```

</details>

**Next:** [What is KLV?](./what-is-klv.md)
