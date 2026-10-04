//! The `#[derive(Klv)]` inputs that must not compile
//!
//! Each block fails to compile for the reason above it. The module exists only so that rustdoc
//! runs these as `compile_fail` doctests; it is not part of the crate's API or its rendered docs
//!
//! `keylen` with `sentinel`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
//! # }
//! #[derive(Klv)]
//! #[klv(keylen(dec = header), allow_unimplemented_encode, sentinel = b"\x06")]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` with `break_on`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
//! # }
//! #[derive(Klv)]
//! #[klv(keylen(dec = header), allow_unimplemented_encode, break_on = 0xFF)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` with a separate `key(..)`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
//! # }
//! #[derive(Klv)]
//! #[klv(keylen(dec = header), key(dec = decb::u8), allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` with a separate `len(..)` and no `key(..)`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
//! # }
//! #[derive(Klv)]
//! #[klv(keylen(dec = header), len(dec = decb::u8_as_usize), allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` declared twice:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
//! # }
//! #[derive(Klv)]
//! #[klv(keylen(dec = header), keylen(dec = header), allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` written as a name-value (not a list):
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
//! # }
//! #[derive(Klv)]
//! #[klv(keylen = header, allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` written as a bare path (not a list):
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! #[derive(Klv)]
//! #[klv(keylen, allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `keylen` without a `dec`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! #[derive(Klv)]
//! #[klv(keylen(), allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! A `keylen` field decoder whose error type is not `ContextError`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! # fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//! #     Ok((decb::u8(input)?, Framing::Inline))
//! # }
//! fn id(_input: &mut &[u8]) -> Result<u8, ()> {
//!     Err(())
//! }
//!
//! #[derive(Klv)]
//! #[klv(keylen(dec = header), allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = id)]
//!     id: u8,
//! }
//! ```
//!
//! `key(..)` with an `enc` and no `dec`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};
//! #[derive(Klv)]
//! #[klv(
//!     key(enc = encb::u8),
//!     len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
//!     allow_unimplemented_encode
//! )]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `len(..)` with a `dec` and no `enc`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};
//! #[derive(Klv)]
//! #[klv(
//!     key(dec = decb::u8, enc = encb::u8),
//!     len(dec = decb::u8_as_usize),
//!     allow_unimplemented_decode
//! )]
//! struct Reading {
//!     #[klv(key = 0x01, enc = *encb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `key(..)` declared twice:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! #[derive(Klv)]
//! #[klv(
//!     key(dec = decb::u8),
//!     key(dec = decb::u8),
//!     len(dec = decb::u8_as_usize),
//!     allow_unimplemented_encode
//! )]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! `len(..)` declared twice:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! #[derive(Klv)]
//! #[klv(
//!     key(dec = decb::u8),
//!     len(dec = decb::u8_as_usize),
//!     len(dec = decb::u8_as_usize),
//!     allow_unimplemented_encode
//! )]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! Neither `key(..)` + `len(..)` nor `keylen(..)`:
//!
//! ```compile_fail
//! # use tinyklv::{dec::binary as decb, prelude::*};
//! #[derive(Klv)]
//! #[klv(allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//! }
//! ```
//!
//! Author: aav
