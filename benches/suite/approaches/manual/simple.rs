//! manual x simple record: hand-pushed key/length/value triples to encode, a hand-rolled
//! key/length/value loop to decode. The key constants below are shared by both directions
//! so they cannot drift apart.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{Manual, put};
use crate::suite::{Codec, records::Simple};

mod key {
    /// Key for field `a` (`u8`)
    pub const A: u8 = 0x01;
    /// Key for field `b` (`u16`)
    pub const B: u8 = 0x02;
    /// Key for field `c` (`u32`)
    pub const C: u8 = 0x03;
    /// Key for field `d` (`u64`)
    pub const D: u8 = 0x04;
    /// Key for field `e` (`i16`)
    pub const E: u8 = 0x05;
    /// Key for field `f` (`i32`)
    pub const F: u8 = 0x06;
    /// Key for field `g` (`f32`)
    pub const G: u8 = 0x07;
    /// Key for field `h` (`f64`)
    pub const H: u8 = 0x08;
}

/// [`Manual`] implementation of [`Codec`] for [`Simple`]
impl Codec<Simple> for Manual {
    fn encode(rec: &Simple) -> Vec<u8> {
        // --------------------------------------------------
        // push each field as a key/length/value triple
        // --------------------------------------------------
        let mut out = Vec::new();
        put(&mut out, key::A, &[rec.a]);
        put(&mut out, key::B, &rec.b.to_be_bytes());
        put(&mut out, key::C, &rec.c.to_be_bytes());
        put(&mut out, key::D, &rec.d.to_be_bytes());
        put(&mut out, key::E, &rec.e.to_be_bytes());
        put(&mut out, key::F, &rec.f.to_be_bytes());
        put(&mut out, key::G, &rec.g.to_be_bytes());
        put(&mut out, key::H, &rec.h.to_be_bytes());
        out
    }

    fn decode(body: &[u8]) -> Option<Simple> {
        // --------------------------------------------------
        // initialise field accumulators
        // --------------------------------------------------
        let mut a = None;
        let mut b = None;
        let mut c = None;
        let mut d = None;
        let mut e = None;
        let mut f = None;
        let mut g = None;
        let mut h = None;
        // --------------------------------------------------
        // walk key/length/value triples and dispatch by tag
        // --------------------------------------------------
        let mut j = 0;
        while j + 2 <= body.len() {
            let tag = body[j];
            let len = usize::from(body[j + 1]);
            j += 2;
            let val = body.get(j..j + len)?;
            j += len;
            match tag {
                key::A => a = Some(*val.first()?),
                key::B => b = Some(u16::from_be_bytes(val.try_into().ok()?)),
                key::C => c = Some(u32::from_be_bytes(val.try_into().ok()?)),
                key::D => d = Some(u64::from_be_bytes(val.try_into().ok()?)),
                key::E => e = Some(i16::from_be_bytes(val.try_into().ok()?)),
                key::F => f = Some(i32::from_be_bytes(val.try_into().ok()?)),
                key::G => g = Some(f32::from_be_bytes(val.try_into().ok()?)),
                key::H => h = Some(f64::from_be_bytes(val.try_into().ok()?)),
                _ => {}
            }
        }
        // --------------------------------------------------
        // construct record, propagating None on any missing field
        // --------------------------------------------------
        Some(Simple {
            a: a?,
            b: b?,
            c: c?,
            d: d?,
            e: e?,
            f: f?,
            g: g?,
            h: h?,
        })
    }
}
