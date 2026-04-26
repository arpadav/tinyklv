//! Resume-flow tests for `DecodePartial` under the 2-arm `Packet`
//! design.
//!
//! `decode_partial` returns `Result<Packet<T, P>, &'static str>`.
//! Truncation surfaces as `Ok(Packet::NeedMore(p))` carrying every
//! field that landed so far. Resumption is driven by the caller (or
//! `Decoder::feed`/`Decoder::next` for buffered streaming). There is
//! no auto-finalise on EOF - if the user wants the partial converted
//! into the final type when the stream "ends", they must either:
//!
//! * call `partial.try_into()` directly, or
//! * use `Decoder::finish` once the upstream signals "no more bytes",
//!   or
//! * implement a `Done` break-condition that fires on a sentinel /
//!   checksum / length-equals-decoded byte count, and let the codegen
//!   call `Partial::finalize` on its own at the break.
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream   = &[u8],
    sentinel = b"RSM",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Triple {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]
    a: u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    b: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)]
    c: u32,
    #[klv(key = 0x04, dec = decb::u8,     enc = *encb::u8)]
    d: Option<u8>,
}

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream   = &[u8],
    sentinel = b"REQ",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct MissingReq {
    // --------------------------------------------------
    // `b` and `c` are required; `a` is optional. a packet that
    // only carries the optional field finishes its body cleanly but
    // fails finalisation with a missing-required label.
    // --------------------------------------------------
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]
    a: Option<u8>,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    b: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)]
    c: u32,
}

/// Build the body of a `Triple` (KLV pairs only, no sentinel/length).
fn triple_body(t: &Triple) -> Vec<u8> {
    #[rustfmt::skip]
    let mut body = vec![
        // a
        0x01, 0x01, t.a,
        // b
        0x02, 0x02,
    ];
    body.extend_from_slice(&t.b.to_be_bytes());
    // c
    body.push(0x03);
    body.push(0x04);
    body.extend_from_slice(&t.c.to_be_bytes());
    // d (optional) if Some
    if let Some(d) = t.d {
        body.push(0x04);
        body.push(0x01);
        body.push(d);
    }
    body
}

#[test]
/// Truncate the body mid-value (inside `b`'s 2-byte payload). The first
/// `decode_partial` must return `Ok(NeedMore(p))` with `p` carrying the
/// already-landed `a`. The second pass on the full body finalises into
/// the complete `Triple`.
fn resume_truncated_inside_value() {
    let want = Triple {
        a: 0xAA,
        b: 0xBBCC,
        c: 0xDDEEFF11,
        d: None,
    };
    let body = triple_body(&want);
    // a landed, b truncated mid-value: keep bytes up to `0x02 0x02 0xBB`
    // (b's key + len + one of two value bytes).
    let cut = 3 /* a klv */ + 2 /* b key+len */ + 1 /* one b payload byte */;
    let (first_half, _) = body.split_at(cut);

    let mut cursor: &[u8] = first_half;
    let p = match Triple::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!(
            "expected NeedMore after mid-value truncation, got {}",
            kind(&other)
        ),
    };

    // the partial should carry `a` (already decoded) and have `b`/`c`
    // still as None (not reached before the truncation).
    assert_eq!(p.a, Some(0xAA));
    assert_eq!(p.b, None);
    assert_eq!(p.c, None);

    // re-decode against the full body to finish (advanced callers in
    // real code would use `Decoder::feed`/`next` + `finish` to merge
    // partials across actual byte feeds; here we exercise the
    // codegen directly).
    let mut full_cursor: &[u8] = &body;
    let p2 = match Triple::decode_partial(&mut full_cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!("expected NeedMore on full body, got {}", kind(&other)),
    };
    let got: Triple = p2.try_into().expect("partial finalises cleanly");
    assert_eq!(got, want);
}

#[test]
/// Truncate right after a key byte (no len yet). NeedMore + partial
/// carries every field landed so far.
fn resume_truncated_after_key_byte() {
    let want = Triple {
        a: 1,
        b: 2,
        c: 3,
        d: None,
    };
    let body = triple_body(&want);
    let cut = 3 /* a */ + 4 /* b */ + 1 /* c's key byte (no len yet) */;
    let (first_half, _) = body.split_at(cut);

    let mut cursor: &[u8] = first_half;
    let p = match Triple::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!(
            "expected NeedMore after key-only truncation, got {}",
            kind(&other)
        ),
    };
    assert_eq!(p.a, Some(1));
    assert_eq!(p.b, Some(2));
    assert_eq!(p.c, None);
}

#[test]
/// Drive the byte-at-a-time scenario through [`tinyklv::Decoder`] in
/// fresh (sentinel-framed) mode. Each `feed` is a single byte; once
/// the full sentinel + length + body have been fed, `.next()` yields
/// `Ok(Triple)`. Sentinel framing is the supported way to bound a
/// streaming packet without explicit `Decoder::finish` or a `Done`
/// break condition.
fn streaming_byte_at_a_time_sentinel_framed() {
    let want = Triple {
        a: 9,
        b: 0x1234,
        c: 0x56789ABC,
        d: Some(42),
    };
    let frame: Vec<u8> = want.encode_frame();

    let mut dec = Triple::decoder();
    let mut got: Option<Triple> = None;
    for chunk in frame.chunks(1) {
        dec.feed(chunk);
        if let Some(r) = dec.next() {
            got = Some(r);
        }
    }
    assert_eq!(got.expect("a complete packet"), want);
    assert!(dec.buffered().is_empty(), "no bytes left behind");
}

#[test]
/// Finalise via `TryFrom<Partial> for T` ergonomically. A
/// fully-populated partial converts cleanly.
fn try_from_partial_ok() {
    let want = Triple {
        a: 7,
        b: 77,
        c: 777,
        d: Some(77),
    };
    let body = triple_body(&want);
    let mut cursor: &[u8] = &body;
    let p = match Triple::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!("expected NeedMore (no Done break), got {}", kind(&other)),
    };
    let got: Triple = p.try_into().expect("finalise");
    assert_eq!(got, want);
}

#[test]
/// Missing required field at end-of-body: finalisation surfaces a
/// `&'static str` label whose text names the offending field. Same
/// information the historical inline `gen_item_set_progress` produced,
/// now stripped of its synthetic input/checkpoint context (which the
/// previous design had to fabricate).
fn missing_required_label_via_finalize() {
    // body contains only field `a` (optional). `b` and `c` required.
    let body: Vec<u8> = vec![0x01, 0x01, 0x07];

    let mut cursor: &[u8] = &body;
    let p = match MissingReq::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!("expected NeedMore (no Done break), got {}", kind(&other)),
    };
    let label: &'static str = <MissingReq as core::convert::TryFrom<_>>::try_from(p)
        .expect_err("partial must fail finalisation: required `b` is missing");
    assert!(
        label.contains("MissingReq::b"),
        "label must name the missing required field; got: {label}"
    );
}

/// Split a framed Triple mid-body and feed through Decoder. First half yields None, completing the feed yields the correct Triple.
#[test]
fn decoder_resume_mid_value() {
    let want = Triple {
        a: 0xAA,
        b: 0xBBCC,
        c: 0xDDEEFF11,
        d: None,
    };
    let frame = want.encode_frame();
    let split = frame.len() / 2;
    let mut dec = Triple::decoder();
    dec.feed(&frame[..split]);
    assert!(dec.next().is_none(), "truncated feed yields None");
    dec.feed(&frame[split..]);
    let got = dec.next().expect("complete feed yields Some");
    assert_eq!(got, want);
    assert!(dec.buffered().is_empty());
}

/// Truncate after the first KLV triple. Decoder reassembles the packet once the remaining bytes arrive.
#[test]
fn decoder_resume_after_key_byte() {
    let want = Triple {
        a: 1,
        b: 2,
        c: 3,
        d: None,
    };
    let frame = want.encode_frame();
    let split = 3 + 1 + 3 + 1; // sentinel + len + a_triple + one more byte
    let mut dec = Triple::decoder();
    dec.feed(&frame[..split]);
    assert!(dec.next().is_none(), "truncated feed yields None");
    dec.feed(&frame[split..]);
    let got = dec.next().expect("complete feed yields Some");
    assert_eq!(got, want);
    assert!(dec.buffered().is_empty());
}

/// Optional field `d` survives streaming reassembly across 4-byte chunks.
#[test]
fn decoder_resume_with_optional() {
    let want = Triple {
        a: 9,
        b: 0x1234,
        c: 0x56789ABC,
        d: Some(42),
    };
    let frame = want.encode_frame();
    let mut dec = Triple::decoder();
    let mut got = Vec::new();
    for chunk in frame.chunks(4) {
        dec.feed(chunk);
        for r in dec.iter() {
            got.push(r);
        }
    }
    assert_eq!(got.len(), 1);
    assert_eq!(got[0], want);
    assert_eq!(got[0].d, Some(42));
}

fn kind<T, P>(p: &Result<Packet<T, P>, &'static str>) -> &'static str
where
    P: tinyklv::Partial<Final = T>,
{
    match p {
        Ok(Packet::Ready(_)) => "Ok(Ready)",
        Ok(Packet::NeedMore(_)) => "Ok(NeedMore)",
        Err(_) => "Err(label)",
    }
}
