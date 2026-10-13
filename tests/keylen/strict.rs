//! `strict` on a fused `#[klv(keylen(..))]` struct: a `Sliced` value its decoder rejects, or leaves
//! bytes of unread, is malformed
//!
//! Both drivers reject such a value at every chunk split, with an error naming the field, and
//! leave the cursor past its frame. The same message without `strict` drops the value, so the
//! field keeps what it had: nothing, which an `Option` or `skip_if_default` field finalizes to its
//! default, or the value of a frame before it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::{header, value};
use tinyklv::prelude::*;

/// A proto3 message under `strict`: `int32 id = 1` (a varint), `string name = 3` and `optional
/// string nick = 4` (length-delimited)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    strict,
    default(typ = i32, dec = varint::dec_i32),
    default(typ = String, dec = value::dec_string),
)]
struct StrictPerson {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 3, wire 2 LEN - `Sliced(n)`, the empty string when absent
    #[klv(key = 0x1A, skip_if_default)]
    name: String,

    /// field 4, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x22)]
    nick: Option<String>,
}

/// [`StrictPerson`] without `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
    default(typ = String, dec = value::dec_string),
)]
struct Person {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 3, wire 2 LEN - `Sliced(n)`, the empty string when absent
    #[klv(key = 0x1A, skip_if_default)]
    name: String,

    /// field 4, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x22)]
    nick: Option<String>,
}

/// A line item: `int32 price = 1` (a varint), 0 when absent
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
)]
struct Item {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08, skip_if_default)]
    price: i32,
}

/// An order under `strict`: `Item item = 17` (a nested message) and `int32 count = 13` (a varint)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    strict,
    default(typ = i32, dec = varint::dec_i32),
)]
struct StrictOrder {
    /// field 17, wire 2 LEN - `Sliced(n)`, decoded one-shot from its slice
    #[klv(key = 0x8A, dec = Item::decode_value)]
    item: Item,

    /// field 13, wire 0 VARINT - `Inline`
    #[klv(key = 0x68)]
    count: i32,
}

/// [`StrictOrder`] without `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
)]
struct Order {
    /// field 17, wire 2 LEN - `Sliced(n)`, decoded one-shot from its slice
    #[klv(key = 0x8A, dec = Item::decode_value)]
    item: Item,

    /// field 13, wire 0 VARINT - `Inline`
    #[klv(key = 0x68)]
    count: i32,
}

/// A badge under `strict`: `int32 id = 1`, length-delimited
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    strict,
    default(typ = i32, dec = varint::dec_i32),
)]
struct StrictBadge {
    /// field 1, wire 2 LEN - `Sliced(n)`, one varint read from it
    #[klv(key = 0x0A)]
    id: i32,
}

/// [`StrictBadge`] without `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
)]
struct Badge {
    /// field 1, wire 2 LEN - `Sliced(n)`, one varint read from it
    #[klv(key = 0x0A)]
    id: i32,
}

/// A `skip_if_default` value that is not UTF-8 is malformed under `strict`; without it the value
/// is dropped and the field finalizes to the empty string
#[test]
fn a_value_its_decoder_rejects_is_malformed() {
    let message = "`StrictPerson::name` value (its decoder rejected the frame's value)";
    #[rustfmt::skip]
    let wire = [
        // id = 5
        0x08, 0x05,
        // name, Sliced(2): bytes that are not UTF-8
        0x1A, 0x02, 0xFF, 0xFE,
    ];
    StrictPerson::assert_rejects_at(&wire, wire.len(), message);
    let dropped = Person {
        id: 5,
        name: String::new(),
        nick: None,
    };
    dropped.assert_decodes_from(&wire);
}

/// An `Option` value that is not UTF-8 is malformed under `strict`; without it the value is
/// dropped and the field finalizes to `None`
#[test]
fn an_option_value_its_decoder_rejects_is_malformed() {
    let message = "`StrictPerson::nick` value (its decoder rejected the frame's value)";
    #[rustfmt::skip]
    let wire = [
        // id = 5
        0x08, 0x05,
        // nick, Sliced(2): bytes that are not UTF-8
        0x22, 0x02, 0xFF, 0xFE,
    ];
    StrictPerson::assert_rejects_at(&wire, wire.len(), message);
    let dropped = Person {
        id: 5,
        name: String::new(),
        nick: None,
    };
    dropped.assert_decodes_from(&wire);
}

/// A nested message cut short inside its slice is malformed under `strict`: its own one-shot
/// decode stops at the cut frame, leaving it unread. Without `strict` the nested message keeps
/// what it read before the cut
#[test]
fn a_nested_message_cut_short_in_its_slice_is_malformed() {
    let message = "`StrictOrder::item` value (its decoder left bytes of the frame's value unread)";
    #[rustfmt::skip]
    let item: &[u8] = &[
        // item, Sliced(2)
        0x8A, 0x01, 0x02,
        // price, its varint cut after a continuation byte
        0x08, 0x96,
    ];
    #[rustfmt::skip]
    let wire = [
        item,
        // count = 1
        &[0x68, 0x01],
    ]
    .concat();
    StrictOrder::assert_rejects_at(&wire, item.len(), message);
    let cut = Order {
        item: Item { price: 0 },
        count: 1,
    };
    cut.assert_decodes_from(&wire);
}

/// A value whose decoder leaves bytes of its slice unread is malformed under `strict`; without it
/// the field keeps what the decoder read and the rest of the slice is dropped
#[test]
fn a_value_with_bytes_left_in_its_slice_is_malformed() {
    let message = "`StrictBadge::id` value (its decoder left bytes of the frame's value unread)";
    #[rustfmt::skip]
    let wire = [
        // id, Sliced(3): 5, then two bytes no decoder reads
        0x0A, 0x03, 0x05, 0xEE, 0xEE,
    ];
    StrictBadge::assert_rejects_at(&wire, wire.len(), message);
    Badge { id: 5 }.assert_decodes_from(&wire);
}
