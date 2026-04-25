#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/15b-consume-impl.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream   = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Heartbeat {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    sequence: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temperature_centideg: u16,

    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    uptime_s: u32,
}

/// A consuming iterator that decodes values as they are encountered
pub struct ConsumeIter<'a, P, S, I, B>
where
    I: Iterator<Item = B>,
    B: AsRef<[u8]>,
{
    dec: &'a mut Decoder<P, S>,
    input: I,
    _marker: std::marker::PhantomData<B>,
}

impl<'a, P, S, I, B> Iterator for ConsumeIter<'a, P, S, I, B>
where
    I: Iterator<Item = B>,
    B: AsRef<[u8]>,
    S: winnow::stream::Stream,
    P: Partial + Default,
    Decoder<P, S>: tinyklv::PartialIterator<P>,
{
    type Item = <P as Partial>::Final;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(v) = self.dec.iter().next() {
                return Some(v);
            }
            let next = self.input.next()?;
            self.dec.feed(next.as_ref());
        }
    }
}

/// A trait to consume an iterator of byte chunks, yielding decoded values
pub trait Consume<P, S>
where
    S: winnow::stream::Stream,
    P: Partial + Default,
    Decoder<P, S>: tinyklv::PartialIterator<P>,
{
    fn consume<I, B>(&mut self, input: I) -> ConsumeIter<'_, P, S, I::IntoIter, B>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>;
}

impl<P, S> Consume<P, S> for Decoder<P, S>
where
    S: winnow::stream::Stream,
    P: Partial + Default,
    Decoder<P, S>: tinyklv::PartialIterator<P>,
{
    fn consume<I, B>(&mut self, input: I) -> ConsumeIter<'_, P, S, I::IntoIter, B>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        ConsumeIter {
            dec: self,
            input: input.into_iter(),
            _marker: std::marker::PhantomData,
        }
    }
}

fn main() {
    let want = vec![
        Heartbeat { sequence: 1, temperature_centideg: 2300, uptime_s: 10 },
        Heartbeat { sequence: 2, temperature_centideg: 2310, uptime_s: 20 },
        Heartbeat { sequence: 3, temperature_centideg: 2340, uptime_s: 30 },
    ];
    let buf: Vec<u8> = want.iter()
        .flat_map(|p| p.encode_frame())
        .collect();

    let mut dec = Heartbeat::decoder();
    let got: Vec<Heartbeat> = dec.consume(buf.chunks(3)).collect();

    assert_eq!(got, want);
    println!("decoded {} packets via custom consume impl", got.len());
}
