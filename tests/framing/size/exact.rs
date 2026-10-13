//! `size(exact = N)` on a field of a key + len struct: the frame's length must be `N`
//!
//! A frame whose length is `N` decodes in both drivers and at every chunk split, and encodes `N`
//! as its length. A frame of any other length is a malformed value: both drivers reject it with an
//! error naming the field and its width, even when the frame runs past the end of the input. A
//! rejected frame the input holds whole is consumed, so a decode resumed after the error starts at
//! the next frame; one that runs past the end is left whole. The width reaches a field through
//! its own `size(..)` or through its type's `default(..)`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::token::take;

/// A device's raw identifier bytes
#[derive(Debug, Clone, Copy, PartialEq)]
struct DeviceId([u8; DeviceId::LEN]);

/// [`DeviceId`] implementation
impl DeviceId {
    /// Byte width of an identifier
    const LEN: usize = 16;
}

/// [`DeviceId`] implementation of [`DecodeValue`]: [`DeviceId::LEN`] raw bytes
impl DecodeValue<&[u8]> for DeviceId {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        take(Self::LEN)
            .try_map(<[u8; Self::LEN]>::try_from)
            .map(Self)
            .parse_next(input)
    }
}

/// [`DeviceId`] implementation of [`EncodeValue`]: the raw bytes
impl EncodeValue for DeviceId {
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

/// A temperature in degrees Celsius, a little-endian `f32` on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
struct Celsius(f32);

/// [`Celsius`] implementation of [`DecodeValue`]: a little-endian `f32`
impl DecodeValue<&[u8]> for Celsius {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        decb::le_f32.map(Self).parse_next(input)
    }
}

/// [`Celsius`] implementation of [`EncodeValue`]: a little-endian `f32`
impl EncodeValue for Celsius {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::le_f32(self.0, out);
    }
}

/// A sampling mode, one byte on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    /// Wire value `0`
    Idle,
    /// Wire value `1`
    Survey,
    /// Wire value `2`
    Burst,
}

/// [`Mode`] implementation of [`TryFrom<u8>`]: a wire byte to its variant
impl TryFrom<u8> for Mode {
    /// The rejected byte
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        match value {
            0 => Ok(Self::Idle),
            1 => Ok(Self::Survey),
            2 => Ok(Self::Burst),
            unknown => Err(unknown),
        }
    }
}

/// `u8` implementation of [`From<Mode>`]: a variant to its wire byte, the inverse of
/// [`TryFrom<u8>`]
impl From<Mode> for u8 {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Idle => 0,
            Mode::Survey => 1,
            Mode::Burst => 2,
        }
    }
}

/// [`Mode`] implementation of [`DecodeValue`]: one byte naming a variant
impl DecodeValue<&[u8]> for Mode {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        decb::u8
            .verify_map(|byte| Self::try_from(byte).ok())
            .parse_next(input)
    }
}

/// [`Mode`] implementation of [`EncodeValue`]: the variant's wire byte
impl EncodeValue for Mode {
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
}

/// A hardware address, as many octets as its decoder is handed
#[derive(Debug, Clone, PartialEq)]
struct HwAddr(Vec<u8>);

/// [`HwAddr`] implementation
impl HwAddr {
    /// Decodes a hardware address of `width` octets
    ///
    /// # Arguments
    ///
    /// * `width` - the octets to read, the length the derive hands a `lengthed` decoder
    ///
    /// # Returns
    ///
    /// A decoder reading `width` octets, or failing on fewer
    fn decode(width: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Self> {
        move |input| {
            take(width)
                .map(|octets: &[u8]| Self(octets.to_vec()))
                .parse_next(input)
        }
    }
}

/// [`HwAddr`] implementation of [`EncodeValue`]: the octets
impl EncodeValue for HwAddr {
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

/// A protobuf-shaped telemetry record: every field a length-delimited frame, most of them a
/// declared exact width
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(
        typ = Celsius,
        dec = Celsius::decode_value,
        enc = Celsius::encode_value,
        size(exact = 4),
    ),
    trait_fallback,
)]
struct Telemetry {
    /// field 1 - its own `size(exact = 16)`
    #[klv(key = 0x0A, size(exact = 16))]
    device: DeviceId,

    /// field 2 - `size(exact = 4)` from the `Celsius` default
    #[klv(key = 0x12)]
    ambient: Celsius,

    /// field 3 - optional, `size(exact = 4)` from the `Celsius` default
    #[klv(key = 0x1A)]
    core: Option<Celsius>,

    /// field 4 - a one-byte custom enum
    #[klv(key = 0x22, size(exact = 1))]
    mode: Mode,

    /// field 5 - repeated: a low and a high reading
    #[klv(key = 0x2A, size(exact = 8))]
    bounds: Vec<Celsius>,

    /// field 6 - a `lengthed` decoder, handed its exact width
    #[klv(
        key = 0x32,
        dec = HwAddr::decode,
        enc = HwAddr::encode_value,
        size(lengthed, exact = 6),
    )]
    mac: Option<HwAddr>,

    /// field 7 - no `exact`, so any length
    #[klv(
        key = 0x3A,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    site: Option<String>,

    /// field 8 - no `size(..)` at all: the frame's length, whatever it is
    #[klv(key = 0x42)]
    history: Vec<Mode>,
}

/// A GPS fix: three coordinates of one exact width under consecutive keys
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64, size(exact = 8)),
)]
struct Fix {
    /// key 1 - `size(exact = 8)` from the `f64` default
    #[klv(key = 0x01)]
    lat: f64,

    /// key 2 - `size(exact = 8)` from the `f64` default
    #[klv(key = 0x02)]
    lon: f64,

    /// key 3 - `size(exact = 8)` from the `f64` default
    #[klv(key = 0x03)]
    alt: f64,
}

/// [`Telemetry`] implementation
impl Telemetry {
    /// device = 00..0F (field 1, 16 bytes)
    #[rustfmt::skip]
    const DEVICE: &[u8] = &[
        0x0A, 0x10,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    ];

    /// ambient = 21.5 (field 2, 4 bytes) - 21.5f32 little-endian
    const AMBIENT: &[u8] = &[0x12, 0x04, 0x00, 0x00, 0xAC, 0x41];

    /// core = 38.0 (field 3, 4 bytes) - 38.0f32 little-endian
    const CORE: &[u8] = &[0x1A, 0x04, 0x00, 0x00, 0x18, 0x42];

    /// mode = Burst (field 4, 1 byte)
    const MODE: &[u8] = &[0x22, 0x01, 0x02];

    /// bounds = [-4.0, 30.0] (field 5, 8 bytes) - two f32 little-endian
    #[rustfmt::skip]
    const BOUNDS: &[u8] = &[
        0x2A, 0x08,
        0x00, 0x00, 0x80, 0xC0,
        0x00, 0x00, 0xF0, 0x41,
    ];

    /// mac = 02:42:AC:11:00:02 (field 6, 6 bytes)
    const MAC: &[u8] = &[0x32, 0x06, 0x02, 0x42, 0xAC, 0x11, 0x00, 0x02];

    /// site = "north" (field 7, 5 bytes)
    const SITE: &[u8] = &[0x3A, 0x05, b'n', b'o', b'r', b't', b'h'];

    /// history = [Idle, Survey, Burst] (field 8, 3 bytes)
    const HISTORY: &[u8] = &[0x42, 0x03, 0x00, 0x01, 0x02];

    /// Every frame of [`Telemetry::wire`], in field order
    const FRAMES: [&[u8]; 8] = [
        Self::DEVICE,
        Self::AMBIENT,
        Self::CORE,
        Self::MODE,
        Self::BOUNDS,
        Self::MAC,
        Self::SITE,
        Self::HISTORY,
    ];

    /// The full wire encoding of [`Telemetry::decoded`]
    fn wire() -> Vec<u8> {
        Self::FRAMES.concat()
    }

    /// [`Telemetry::wire`] with one frame swapped for another
    ///
    /// # Arguments
    ///
    /// * `frame` - one of [`Telemetry::FRAMES`]
    /// * `with` - the frame that takes its place
    ///
    /// # Returns
    ///
    /// The wire, with `with` where `frame` was
    fn wire_with(frame: &[u8], with: &[u8]) -> Vec<u8> {
        Self::FRAMES
            .map(|own| if own == frame { with } else { own })
            .concat()
    }

    /// The canonical decoded [`Telemetry`] matching [`Telemetry::wire`]
    fn decoded() -> Self {
        Self {
            device: DeviceId(std::array::from_fn(|at| at as u8)),
            ambient: Celsius(21.5),
            core: Some(Celsius(38.0)),
            mode: Mode::Burst,
            bounds: vec![Celsius(-4.0), Celsius(30.0)],
            mac: Some(HwAddr(vec![0x02, 0x42, 0xAC, 0x11, 0x00, 0x02])),
            site: Some("north".to_owned()),
            history: vec![Mode::Idle, Mode::Survey, Mode::Burst],
        }
    }
}

/// Encode writes each exact width as the frame's length, and the bytes decode back
#[test]
fn encode_writes_each_exact_width_as_the_length() {
    let encoded = Telemetry::decoded().encoded();
    assert_eq!(encoded, Telemetry::wire());
    Telemetry::decoded().assert_decodes_from(&encoded);
}

/// A value its encoder writes in another byte count than its exact width fails the debug check:
/// four octets for the six-octet `mac`, staged ahead of the length it is framed by
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "the value encoder of `mac` wrote a different byte count")]
fn a_value_encoded_in_another_width_fails_the_debug_check() {
    let short = Telemetry {
        mac: Some(HwAddr(vec![0x02, 0x42, 0xAC, 0x11])),
        ..Telemetry::decoded()
    };
    short.encoded();
}

/// A mismatched length is rejected before the frame is classed as truncation: a frame that runs
/// past the end of the input errors in both drivers, where a matching one would wait or stop, and
/// leaves the cursor at the frame start. Of fields that share a width, the rejection names the
/// one the frame is for
#[test]
fn a_mismatched_length_is_rejected_even_past_the_end_of_the_input() {
    let device = "`Telemetry::device` value (its item length does not match `size(exact = 16)`)";
    let cut = [
        Telemetry::AMBIENT,
        // device, a 64-byte length with 3 bytes after it
        &[0x0A, 0x40, 0x00, 0x01, 0x02],
    ]
    .concat();
    Telemetry::assert_rejects_at(&cut, Telemetry::AMBIENT.len(), device);
    #[rustfmt::skip]
    let lat: &[u8] = &[
        // lat = 1.0, 8 bytes
        0x01, 0x08,
        0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let shared_width = [
        lat,
        // lon, a 16-byte length with 2 bytes after it; lat and alt are 8 bytes wide too
        &[0x02, 0x10, 0x00, 0x00],
    ]
    .concat();
    let lon = "`Fix::lon` value (its item length does not match `size(exact = 8)`)";
    Fix::assert_rejects_at(&shared_width, lat.len(), lon);
}

/// A frame of the matching length that runs past the end of the input stays truncation:
/// streaming waits for the rest, one-shot finalizes without it
#[test]
fn a_matching_length_cut_short_is_truncation() {
    let wire = [
        Telemetry::DEVICE,
        Telemetry::AMBIENT,
        Telemetry::MODE,
        Telemetry::BOUNDS,
        Telemetry::HISTORY,
        // core, 4 bytes with 2 of them sent
        &[0x1A, 0x04, 0x00, 0x00],
    ]
    .concat();
    let mut expected = Telemetry::decoded();
    expected.core = None;
    expected.mac = None;
    expected.site = None;
    // --------------------------------------------------
    // one-shot finalizes before the cut frame, leaving it unconsumed
    // --------------------------------------------------
    let mut cursor = &wire[..];
    assert_eq!(Telemetry::decode_value(&mut cursor).as_ref(), Ok(&expected));
    assert_eq!(cursor.len(), 4);
    // --------------------------------------------------
    // streaming pauses before the cut frame, leaving it unconsumed for the rest to join
    // --------------------------------------------------
    let mut cursor = &wire[..];
    expected.assert_streams_to(&mut cursor);
    assert_eq!(cursor.len(), 4);
}

/// Both drivers word a mismatched length the same way and leave the cursor past the frame
#[test]
fn the_rejection_names_the_field_and_leaves_the_cursor_past_the_frame() {
    let message = "`Telemetry::mode` value (its item length does not match `size(exact = 1)`)";
    // mode, 2 bytes
    let mode: &[u8] = &[0x22, 0x02, 0x02, 0x02];
    let wire = Telemetry::wire_with(Telemetry::MODE, mode);
    let before_mode = [Telemetry::DEVICE, Telemetry::AMBIENT, Telemetry::CORE].concat();
    Telemetry::assert_rejects_at(&wire, before_mode.len() + mode.len(), message);
}
