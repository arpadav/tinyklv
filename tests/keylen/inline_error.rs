//! The one-shot error of a malformed `Inline` value in a fused `#[klv(keylen(..))]` struct
//!
//! Its display names the field first and then says why the value decoder failed, in the
//! decoder's own words: its label, its cause, or both. The cause itself stays in the error's
//! cause chain, labelled or not, at any depth. A failure nested `Inline` values deep names every
//! field on the way down, outermost first, in text that grows with the depth and no faster, with
//! two labels and at most two errors in its cause chain at any depth
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::error::ContextErrorExt;
use crate::common::varint;
use crate::support::header;
use tinyklv::dec::ber as decber;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::num::TryFromIntError;
use winnow::error::StrContext;

/// Message whose `Inline` values fail mid-buffer, each decoder reporting why its own way
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
)]
struct Probe {
    /// field 1, wire 0 VARINT - fails with its own label alone
    #[klv(key = 0x08, dec = Probe::nonzero)]
    count: Option<u64>,

    /// field 2, wire 0 VARINT - fails with a cause alone
    #[klv(key = 0x10, dec = Probe::narrow)]
    level: Option<u8>,
}

/// [`Probe`] implementation
impl Probe {
    /// The label [`Probe::nonzero`] fails with
    const NONZERO: &str = "non-zero varint";

    /// Decodes a varint that must not be zero
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the value; advanced past the varint on success
    ///
    /// # Returns
    ///
    /// The varint, or an error labelled [`Probe::NONZERO`] and nothing else, with the cursor
    /// rewound to the varint
    fn nonzero(input: &mut &[u8]) -> tinyklv::Result<u64> {
        varint::dec_u64
            .verify(|value| *value != 0)
            .context(StrContext::Label(Self::NONZERO))
            .parse_next(input)
    }

    /// Decodes a varint as a `u8`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the value; advanced past the varint on success
    ///
    /// # Returns
    ///
    /// The `u8`, or an error whose cause is the failed conversion and nothing else, with the
    /// cursor rewound to the varint
    fn narrow(input: &mut &[u8]) -> tinyklv::Result<u8> {
        varint::dec_u64.try_map(u8::try_from).parse_next(input)
    }
}

/// A chain of messages, each nesting the next as an `Inline` value right after its key
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_u8),
    allow_unimplemented_encode,
)]
struct Chain {
    /// key `0x01` - `Inline`, another [`Chain`]
    #[klv(key = 0x01, dec = Chain::boxed)]
    next: Option<Box<Chain>>,

    /// key `0x02` - `Inline` BER-OID, malformed past `u8::MAX`
    #[klv(key = 0x02, dec = Chain::leaf)]
    leaf: Option<u8>,
}

/// [`Chain`] implementation
impl Chain {
    /// Display bytes each nesting level may add to the error
    const DISPLAY_PER_LEVEL: usize = 256;

    /// Errors the cause chain holds at any depth when the leaf decoder's error carries a cause:
    /// the outermost value decoder's error, then that cause
    const CAUSE_LINKS: usize = 2;

    /// The label [`Chain::leaf`] fails with
    const LEAF: &str = "u8 BER-OID";

    /// Decodes a BER-OID as a `u8`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the value; advanced past the BER-OID on success
    ///
    /// # Returns
    ///
    /// The `u8`, or, for a BER-OID past `u8::MAX`, an error with the cursor rewound to it: labelled
    /// [`Chain::LEAF`] with the failed conversion as its cause, or, past `u64`, labelled by
    /// `ber_oid` and then [`Chain::LEAF`], with no cause
    fn leaf(input: &mut &[u8]) -> tinyklv::Result<u8> {
        decber::ber_oid::<u64>
            .try_map(u8::try_from)
            .context(StrContext::Label(Self::LEAF))
            .parse_next(input)
    }

    /// Decodes the [`Chain`] nested at the cursor
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the nested chain's first frame
    ///
    /// # Returns
    ///
    /// The nested chain, or its decode error
    fn boxed(input: &mut &[u8]) -> tinyklv::Result<Box<Self>> {
        Self::decode_value(input).map(Box::new)
    }

    /// A chain `depth` levels deep whose innermost `leaf` overflows `u8`
    ///
    /// # Arguments
    ///
    /// * `depth` - how many `next` frames lead to the `leaf`
    fn overflowing_leaf_wire(depth: usize) -> Vec<u8> {
        #[rustfmt::skip]
        let leaf = [
            // leaf = 256, past `u8::MAX`
            0x02, 0x82, 0x00,
        ];
        [vec![0x01; depth], leaf.to_vec()].concat()
    }
}

/// A decoder failing with only its own label: the display names the field, then that label
#[test]
fn oneshot_display_names_field_then_decoder_label() {
    #[rustfmt::skip]
    let wire = [
        // count = 0, which the decoder rejects
        0x08, 0x00,
        // level = 5
        0x10, 0x05,
    ];
    let error = Probe::oneshot_error(&wire);
    assert!(
        error.names_in_order(&["Probe::count", Probe::NONZERO]),
        "display: {error}"
    );
}

/// A decoder failing with only a cause: the display names the field, then the cause, and the
/// cause itself stays in the error's cause chain
#[test]
fn oneshot_error_keeps_decoder_cause() {
    #[rustfmt::skip]
    let wire = [
        // level = 300, past `u8::MAX`
        0x10, 0xAC, 0x02,
        // count = 1
        0x08, 0x01,
    ];
    let cause = u8::try_from(300_u64).unwrap_err().to_string();
    let error = Probe::oneshot_error(&wire);
    assert!(
        error.names_in_order(&["Probe::level", &cause]),
        "display: {error}"
    );
    assert!(
        error.find_cause::<TryFromIntError>().is_some(),
        "display: {error}"
    );
}

/// A decoder failing with its own label and a cause: the display names the field, then that
/// label, and the cause itself stays in the error's cause chain
#[test]
fn oneshot_error_keeps_labelled_decoder_cause() {
    let error = Chain::oneshot_error(&Chain::overflowing_leaf_wire(0));
    assert!(
        error.names_in_order(&["Chain::leaf", Chain::LEAF]),
        "display: {error}"
    );
    assert!(
        error.find_cause::<TryFromIntError>().is_some(),
        "display: {error}"
    );
}

/// A failure two `Inline` levels down names both fields, outermost first, then the decoder's
/// own reason, and carries two labels: the outermost field's and the next one's
#[test]
fn nested_oneshot_error_names_every_field_outermost_first() {
    let error = Chain::oneshot_error(&Chain::overflowing_leaf_wire(2));
    assert!(
        error.names_in_order(&["Chain::next", "Chain::next", "Chain::leaf", Chain::LEAF]),
        "display: {error}"
    );
    let labels: Vec<_> = error.labels().collect();
    assert!(
        matches!(
            labels[..],
            [outer, inner] if outer.contains("Chain::next") && inner.contains("Chain::next")
        ),
        "labels: {labels:?}"
    );
}

/// A failure 1, 3 or 32 `Inline` levels down holds the same two links at each depth: no level in
/// between adds a link, and none loses the leaf decoder's cause
#[test]
fn nested_oneshot_cause_chain_holds_two_links() {
    for depth in [1, 3, 32] {
        let error = Chain::oneshot_error(&Chain::overflowing_leaf_wire(depth));
        let links = error.causes().count();
        assert_eq!(
            links,
            Chain::CAUSE_LINKS,
            "depth {depth}: cause chain of: {error}"
        );
        assert!(
            error.find_cause::<TryFromIntError>().is_some(),
            "depth {depth}: display: {error}"
        );
    }
}

/// A decoder failing with its own label and no cause, one `Inline` level down: the cause chain
/// holds the outermost value decoder's error alone, since the leaf decoder's error carries none
#[test]
fn nested_oneshot_error_without_cause_holds_one_link() {
    #[rustfmt::skip]
    let wire = [
        // next = a chain of one frame
        0x01,
        // leaf = 2^64, past `u64::MAX`
        0x02, 0x82, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00,
    ];
    let error = Chain::oneshot_error(&wire);
    assert!(
        error.names_in_order(&["Chain::next", "Chain::leaf"]),
        "display: {error}"
    );
    let links = error.causes().count();
    assert_eq!(links, Chain::CAUSE_LINKS - 1, "cause chain of: {error}");
}

/// The nested error's display grows linearly with the depth: depth 8 first, which a display
/// that doubles per level already overruns while it is still small, then depth 32, which one
/// that grows with the square of the depth overruns
#[test]
fn nested_oneshot_display_grows_linearly() {
    for depth in [8, 32] {
        let display = Chain::oneshot_error(&Chain::overflowing_leaf_wire(depth)).to_string();
        assert!(
            display.len() <= (depth + 1) * Chain::DISPLAY_PER_LEVEL,
            "depth {depth}: {} display bytes",
            display.len()
        );
    }
}
