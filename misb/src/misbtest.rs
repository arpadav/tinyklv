use tinyklv::Klv;

use winnow::stream::Stream;
use winnow::combinator::seq;
use winnow::error::AddContext;
use winnow::{prelude::*, token::take};

#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = b"x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00",
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
    pub checksum: Option<u16>,

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

// #[automatically_derived]
// impl tinyklv::prelude::StreamDecode<&[u8]> for Misb0601 {
//     fn decode(input: &mut &[u8]) -> winnow::PResult<Self> {
//         let checkpoint = input.checkpoint();

//         let packet_len = seq!(_:
//             0x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00",
//             tinyklv::codecs::ber::dec::ber_length,
//         ).parse_next(input)?.0 as usize;

//         let mut packet = take(packet_len).parse_next(input)?;
//         let packet: &mut &[u8] = &mut packet;
        
//         let mut checksum: Option<u16> = None;
//         let mut precision_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
//         let mut mission_id: Option<String> = None;
//         let mut platform_tail_numer: Option<String> = None;
//         let mut platform_heading_angle: Option<u16> = None;
//         let mut platform_pitch_angle: Option<i16> = None;
//         let mut platform_roll_angle: Option<i16> = None;
//         let mut platform_true_airspeed: Option<u8> = None;
//         let mut platform_indicated_airspeed: Option<u8> = None;
//         let mut platform_designation: Option<String> = None;
//         let mut image_source_sensor: Option<String> = None;
//         let mut image_coordinate_system: Option<String> = None;
//         let mut sensor_latitude: Option<i32> = None;
//         let mut sensor_longitude: Option<i32> = None;
//         let mut sensor_true_altitude: Option<u16> = None;
//         let mut sensor_hfov: Option<u16> = None;
//         let mut sensor_vfov: Option<u16> = None;
//         let mut sensor_relative_azimuth_angle: Option<u32> = None;

//         loop {
//             match (
//                 tinyklv::codecs::ber::dec::ber_oid::<u64>,
//                 tinyklv::codecs::ber::dec::ber_length,
//             ).parse_next(packet) {
//                 Ok((key, len)) => match (key, len) {
//                     (0x01, _) => checksum = tinyklv::codecs::binary::dec::be_u16(packet).ok(),
//                     (0x02, _) => precision_timestamp = crate::dec::precision_timestamp(packet).ok(),
//                     (0x03, len) => mission_id = tinyklv::codecs::binary::dec::to_string(packet, len).ok(),
//                     (0x04, len) => platform_tail_numer = tinyklv::codecs::binary::dec::to_string(packet, len).ok(),
//                     (0x05, _) => platform_heading_angle = tinyklv::codecs::binary::dec::be_u16(packet).ok(),
//                     (0x06, _) => platform_pitch_angle = tinyklv::codecs::binary::dec::be_i16(packet).ok(),
//                     (0x07, _) => platform_roll_angle = tinyklv::codecs::binary::dec::be_i16(packet).ok(),
//                     (0x08, _) => platform_true_airspeed = tinyklv::codecs::binary::dec::be_u8(packet).ok(),
//                     (0x09, _) => platform_indicated_airspeed = tinyklv::codecs::binary::dec::be_u8(packet).ok(),
//                     (0x0a, len) => platform_designation = tinyklv::codecs::binary::dec::to_string(packet, len).ok(),
//                     (0x0b, len) => image_source_sensor = tinyklv::codecs::binary::dec::to_string(packet, len).ok(),
//                     (0x0c, len) => image_coordinate_system = tinyklv::codecs::binary::dec::to_string(packet, len).ok(),
//                     (0x0d, _) => sensor_latitude = tinyklv::codecs::binary::dec::be_i32(packet).ok(),
//                     (0x0e, _) => sensor_longitude = tinyklv::codecs::binary::dec::be_i32(packet).ok(),
//                     (0x0f, _) => sensor_true_altitude = tinyklv::codecs::binary::dec::be_u16(packet).ok(),
//                     (0x10, _) => sensor_hfov = tinyklv::codecs::binary::dec::be_u16(packet).ok(),
//                     (0x11, _) => sensor_vfov = tinyklv::codecs::binary::dec::be_u16(packet).ok(),
//                     (0x12, _) => sensor_relative_azimuth_angle = tinyklv::codecs::binary::dec::be_u32(packet).ok(),
//                     (_, len) => { let _ = take::<usize, &[u8], winnow::error::ContextError>(len).parse_next(packet); },
//                 }
//                 Err(_) => break,
//             }
//         }

//         // println!("checksum: {:?}", checksum);
//         // println!("precision_timestamp: {:?}", precision_timestamp);
//         // println!("mission_id: {:?}", mission_id);
//         // println!("platform_tail_numer: {:?}", platform_tail_numer);
//         // println!("platform_heading_angle: {:?}", platform_heading_angle);
//         // println!("platform_pitch_angle: {:?}", platform_pitch_angle);
//         // println!("platform_roll_angle: {:?}", platform_roll_angle);
//         // println!("platform_true_airspeed: {:?}", platform_true_airspeed);
//         // println!("platform_indicated_airspeed: {:?}", platform_indicated_airspeed);
//         // println!("platform_designation: {:?}", platform_designation);
//         // println!("image_source_sensor: {:?}", image_source_sensor);
//         // println!("image_coordinate_system: {:?}", image_coordinate_system);
//         // println!("sensor_latitude: {:?}", sensor_latitude);
//         // println!("sensor_longitude: {:?}", sensor_longitude);
//         // println!("sensor_true_altitude: {:?}", sensor_true_altitude);
//         // println!("sensor_hfov: {:?}", sensor_hfov);
//         // println!("sensor_vfov: {:?}", sensor_vfov);
//         // println!("sensor_relative_azimuth_angle: {:?}", sensor_relative_azimuth_angle);

//         Err(winnow::error::ErrMode::Backtrack(
//             winnow::error::ContextError::new()
//             .add_context(input, &checkpoint, 
//                 winnow::error::StrContext::Label("Misb0601")
//             )
//         ))
//     }
// }