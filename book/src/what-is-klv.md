# What is KLV?

KLV (key, length, value) lays out binary data as items. Each item has three parts, back to back:

| part | holds | bytes |
|---|---|---|
| key | which field the item is | <span class="klv">k 02</span> |
| length | how many value bytes follow | <span class="klv">l 02</span> |
| value | the field's data | <span class="klv">v 09 2E</span> |

Items follow one another:

```klv
k 01 l 01 v 2A        // key 01: 1 byte, 42
k 02 l 02 v 09 2E     // key 02: 2 bytes, 2350
```

## Sentinel

On a live link, each packet starts with a sentinel: fixed bytes that mark where it begins. The length of the packet's body follows, then its items. The sentinel works as the packet's key, so a packet is itself a key, a length and a value:

```klv
j DE AD 00 00         // junk before the packet
s 53 45 4E 53 4F 52   // sentinel: "SENSOR"
l 07                  // the body: 7 bytes
    k 01 l 01 v 2A
    k 02 l 02 v 09 2E
```

## Nesting

A value can hold items of its own, laid out the same way:

```klv
k 03 l 0C                     // key 03: 12 bytes, which are two items
    k 01 l 04 v 03 11 7A D0   // lat 51477200
    k 02 l 04 v FF FE 10 CC   // lon -126772
```

## What it gives you

| property | what it means |
|---|---|
| forward compatible | an old decoder skips a key it does not know, since the length says how many bytes to skip |
| self-describing | the key names the field, so items need no fixed offset and can come in any order |
| resyncable | on a noisy link (UDP, radio, RS-422), the sentinel lets a decoder skip junk and find the next packet |

## What tinyklv leaves to you

The layout is fixed. How each part is written is yours to choose:

| part | common choices |
|---|---|
| sentinel | any fixed bytes, often 16 |
| key | a 1-byte `u8`, a 2-byte `u16`, a BER-OID |
| length | a 1-byte `u8`, a BER length, a 4-byte `u32` |
| value | anything |

tinyklv ships [codecs](./reference/codecs.md) for keys, lengths and values, and you can mix them per struct and per field.

**Next:** [First packet](./tutorial/1-basics/01-first-packet.md)
