use tinyklv::Klv;

use winnow::stream::Stream;
use winnow::combinator::seq;
use winnow::error::AddContext;
use winnow::{prelude::*, token::take};

#[derive(Klv, Debug)]
#[klv(
    stream = &[u8],
    sentinel = b"\x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00",
    key(enc = tinyklv::codecs::ber::enc::ber_oid,
        dec = tinyklv::codecs::ber::dec::ber_oid::<u64>),
    len(enc = tinyklv::codecs::ber::enc::ber_length,
        dec = tinyklv::codecs::ber::dec::ber_length),
    default(ty = u8, dec = tinyklv::codecs::binary::dec::be_u8),
    default(ty = u16, dec = tinyklv::codecs::binary::dec::be_u16),
    default(ty = u32, dec = tinyklv::codecs::binary::dec::be_u32),
    default(ty = i16, dec = tinyklv::codecs::binary::dec::be_i16),
    default(ty = i32, dec = tinyklv::codecs::binary::dec::be_i32),
    default(ty = String, dec = tinyklv::codecs::binary::dec::to_string, dyn = true),
)]
pub struct Misb0601 {
    #[klv(key = 0x01)]
    pub checksum: u16,

    #[klv(key = 0x02, dec = crate::dec::precision_timestamp)]
    pub precision_timestamp: Option<chrono::DateTime<chrono::Utc>>,

    #[klv(key = 0x03)]
    pub mission_id: Option<String>,

    #[klv(key = 0x04)]
    pub platform_tail_number: Option<String>,

    #[klv(key = 0x05)]
    pub platform_heading_angle: Option<u16>,

    #[klv(key = 0x06)]
    pub platform_pitch_angle: Option<i16>,

    #[klv(key = 0x07)]
    pub platform_roll_angle: Option<i16>,

    #[klv(key = 0x08)]
    pub platform_true_airspeed: Option<u8>,

    #[klv(key = 0x09)]
    pub platform_indicated_airspeed: Option<u8>,

    #[klv(key = 0x0a)]
    pub platform_designation: Option<String>,

    #[klv(key = 0x0b)]
    pub image_source_sensor: Option<String>,

    #[klv(key = 0x0c)]
    pub image_coordinate_system: Option<String>,

    #[klv(key = 0x0d)]
    pub sensor_latitude: Option<i32>,

    #[klv(key = 0x0e)]
    pub sensor_longitude: Option<i32>,

    #[klv(key = 0x0f)]
    /// Altitude of sensor above MSL (mean sea level).
    pub sensor_true_altitude: Option<u16>,

    #[klv(key = 0x10)]
    /// Horizontal field of view of selected imaging sensor
    pub sensor_hfov: Option<u16>,

    #[klv(key = 0x11)]
    /// Vertical field of view of selected imaging sensor
    pub sensor_vfov: Option<u16>,

    #[klv(key = 0x12)]
    /// Relative rotation angle of sensor to platform longitudinal axis
    pub sensor_relative_azimuth_angle: Option<u32>,
}

impl ::tinyklv::prelude::Seek<&[u8]> for Misb0601 {
    fn seek<'s, 'z>(
        input: &'s mut &'z [u8],
    ) -> ::tinyklv::reexport::winnow::PResult<&'s mut &'z [u8]> {
        let checkpoint = input.checkpoint();
        let packet_len = match ::winnow::combinator::trace(
                "",
                move |input: &mut _| {
                    (
                        b"\x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00"
                            .void(),
                        tinyklv::codecs::ber::dec::ber_length,
                    )
                        .map(|t| { (t.1,) })
                        .parse_next(input)
                },
            )
            .parse_next(input)
        {
            Ok(x) => x.0 as usize,
            Err(e) => {
                return Err(
                    e
                        .backtrack()
                        .add_context(
                            input,
                            &checkpoint,
                            ::tinyklv::reexport::winnow::error::StrContext::Label(
                                "Unable to find recognition sentinal and packet length for initial parsing of `Misb0601` packet",
                            ),
                        ),
                );
            }
        };
        ::tinyklv::reexport::winnow::token::take(packet_len)
            .parse_next(input)
            .map(move |slice| {
                let mut_ref: &mut &[u8] = input;
                *mut_ref = slice;
                mut_ref
            })
    }
}

// impl ::tinyklv::prelude::Seek<&[u8]> for Misb0601 {
//     fn seek<'s, 'a>(input: &'s mut &'a [u8]) -> ::tinyklv::reexport::winnow::PResult<&'s mut &'a [u8]> {
//         let checkpoint = input.checkpoint();
//         let packet_len = match ::tinyklv::reexport::winnow::combinator::seq!(_:
//             b"\x01",
//             tinyklv::codecs::ber::dec::ber_length,
//         ).parse_next(input) {
//             Ok(x) => x.0 as usize,
//             Err(e) => return Err(e.backtrack().add_context(
//                 input,
//                 &checkpoint,
//                 ::tinyklv::reexport::winnow::error::StrContext::Label(
//                     concat!("Unable to find recognition sentinal and packet length for initial parsing of `", stringify!(#name), "` packet")
//                 )
//             )),
//         };
//         ::tinyklv::reexport::winnow::token::take(packet_len)
//             .parse_next(input)
//             .map(move |slice| {
//                 let mut_ref: &mut &[u8] = input; // Ensure mutability is preserved.
//                 *mut_ref = slice;
//                 mut_ref
//             })
//         // ::tinyklv::reexport::winnow::token::take(packet_len)
//         //     .parse_next(input)
//         //     .map(|mut result| result)

//         // ::tinyklv::reexport::winnow::token::take(packet_len).parse_next(input)
//         // match ::tinyklv::reexport::winnow::token::take(packet_len).parse_next(input) {
//         //     Ok(mut x) => Ok(&mut x),
//         //     Err(e) => match e.is_incomplete() {
//         //         true => return Err(::tinyklv::reexport::winnow::error::ErrMode::Incomplete(::tinyklv::reexport::winnow::error::Needed::Unknown)),
//         //         false => return Err(e.backtrack()),
//         //     },
//         // }
//     }
// }