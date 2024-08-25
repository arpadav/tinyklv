use crate::Klv;

use winnow::stream::Stream;
use winnow::combinator::seq;
use winnow::error::AddContext;
use winnow::{prelude::*, token::take};

#[derive(Klv)]
#[klv(
    stream = u8,
    sentinel = b"\x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00",
    key(enc = tinyklv::enc::ber_oid, dec = tinyklv::dec::ber_oid),
    len(enc = tinyklv::enc::ber_length, dec = tinyklv::dec::ber_length),
    default(ty = String, dyn = true, dec = tinyklv::dec::to_string),
    default(ty = u8, dec = winnow::binary::be_u8),
    default(ty = u16, dec = winnow::binary::be_u16),
    default(ty = u32, dec = winnow::binary::be_u32),
    default(ty = i16, dec = winnow::binary::be_i16),
    default(ty = i32, dec = winnow::binary::be_i32),
)]
pub struct Misb0601 {
    #[klv(key = b"\x01")]
    pub checksum: u16,

    #[klv(key = b"\x02", dec = dec::precision_timestamp)]
    pub precision_timestamp: chrono::DateTime<chrono::Utc>,

    #[klv(key = b"\x03")]
    pub mission_id: String,

    #[klv(key = b"\x04")]
    pub platform_tail_numer: String,

    #[klv(key = b"\x05")]
    pub platform_heading_angle: u16,

    #[klv(key = b"\x06")]
    pub platform_pitch_angle: i16,

    #[klv(key = b"\x07")]
    pub platform_roll_angle: i16,

    #[klv(key = b"\x08")]
    pub platform_true_airspeed: u8,

    #[klv(key = b"\x09")]
    pub platform_indicated_airspeed: u8,

    #[klv(key = b"\x0a")]
    pub platform_designation: String,

    #[klv(key = b"\x0b")]
    pub image_source_sensor: String,

    #[klv(key = b"\x0c")]
    pub image_coordinate_system: String,

    #[klv(key = b"\x0d")]
    pub sensor_latitude: i32,

    #[klv(key = b"\x0e")]
    pub sensor_longitude: i32,

    #[klv(key = b"\x0f")]
    /// Altitude of sensor above MSL (mean sea level).
    pub sensor_true_altitude: u16,

    #[klv(key = b"\x10")]
    /// Horizontal field of view of selected imaging sensor
    pub sensor_hfov: u16,

    #[klv(key = b"\x11")]
    /// Vertical field of view of selected imaging sensor
    pub sensor_vfov: u16,

    #[klv(key = b"\x12")]
    /// Relative rotation angle of sensor to platform longitudinal axis
    pub sensor_relative_azimuth_angle: u32,
}

impl crate::prelude::StreamDecode<&[u8]> for Misb0601 {
    fn decode(input: &mut &[u8]) -> winnow::PResult<Self> {
        let checkpoint = input.checkpoint();

        let packet_len = seq!(_:
            b"\x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00",
            crate::dec::ber_length,
        ).parse_next(input)?.0 as usize;
            // .map(|bl| bl as usize)
            // .parse_next(input)?;

        let mut packet = take(packet_len).parse_next(input)?;
        let packet: &mut &[u8] = &mut packet;
        
        let mut checksum: Option<u16> = None;
        let mut precision_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut mission_id: Option<String> = None;
        let mut platform_tail_numer: Option<String> = None;
        let mut platform_heading_angle: Option<u16> = None;
        let mut platform_pitch_angle: Option<i16> = None;
        let mut platform_roll_angle: Option<i16> = None;
        let mut platform_true_airspeed: Option<u8> = None;
        let mut platform_indicated_airspeed: Option<u8> = None;
        let mut platform_designation: Option<String> = None;
        let mut image_source_sensor: Option<String> = None;
        let mut image_coordinate_system: Option<String> = None;
        let mut sensor_latitude: Option<i32> = None;
        let mut sensor_longitude: Option<i32> = None;
        let mut sensor_true_altitude: Option<u16> = None;
        let mut sensor_hfov: Option<u16> = None;
        let mut sensor_vfov: Option<u16> = None;
        let mut sensor_relative_azimuth_angle: Option<u32> = None;

        loop {
            match (crate::defaults::dec::ber_oid::<u64>, crate::defaults::dec::ber_length).parse_next(packet) {
                Ok((key, len)) => match (key, len) {
                    (0x01, _) => checksum = winnow::binary::be_u16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x02, _) => precision_timestamp = crate::misb::dec::precision_timestamp(packet).ok(),
                    (0x03, len) => mission_id = crate::defaults::dec::to_string_parser(packet, len).ok(),
                    (0x04, len) => platform_tail_numer = crate::defaults::dec::to_string_parser(packet, len).ok(),
                    (0x05, _) => platform_heading_angle = winnow::binary::be_u16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x06, _) => platform_pitch_angle = winnow::binary::be_i16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x07, _) => platform_roll_angle = winnow::binary::be_i16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x08, _) => platform_true_airspeed = winnow::binary::be_u8::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x09, _) => platform_indicated_airspeed = winnow::binary::be_u8::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x0a, len) => platform_designation = crate::defaults::dec::to_string_parser(packet, len).ok(),
                    (0x0b, len) => image_source_sensor = crate::defaults::dec::to_string_parser(packet, len).ok(),
                    (0x0c, len) => image_coordinate_system = crate::defaults::dec::to_string_parser(packet, len).ok(),
                    (0x0d, _) => sensor_latitude = winnow::binary::be_i32::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x0e, _) => sensor_longitude = winnow::binary::be_i32::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x0f, _) => sensor_true_altitude = winnow::binary::be_u16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x10, _) => sensor_hfov = winnow::binary::be_u16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x11, _) => sensor_vfov = winnow::binary::be_u16::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (0x12, _) => sensor_relative_azimuth_angle = winnow::binary::be_u32::<&[u8], winnow::error::ContextError>(packet).ok(),
                    (_, len) => { let _ = take(len).parse_next(packet)?; },
                }
                Err(_) => break,
            }
        }

        println!("checksum: {:?}", checksum);
        println!("precision_timestamp: {:?}", precision_timestamp);
        println!("mission_id: {:?}", mission_id);
        println!("platform_tail_numer: {:?}", platform_tail_numer);
        println!("platform_heading_angle: {:?}", platform_heading_angle);
        println!("platform_pitch_angle: {:?}", platform_pitch_angle);
        println!("platform_roll_angle: {:?}", platform_roll_angle);
        println!("platform_true_airspeed: {:?}", platform_true_airspeed);
        println!("platform_indicated_airspeed: {:?}", platform_indicated_airspeed);
        println!("platform_designation: {:?}", platform_designation);
        println!("image_source_sensor: {:?}", image_source_sensor);
        println!("image_coordinate_system: {:?}", image_coordinate_system);
        println!("sensor_latitude: {:?}", sensor_latitude);
        println!("sensor_longitude: {:?}", sensor_longitude);
        println!("sensor_true_altitude: {:?}", sensor_true_altitude);
        println!("sensor_hfov: {:?}", sensor_hfov);
        println!("sensor_vfov: {:?}", sensor_vfov);
        println!("sensor_relative_azimuth_angle: {:?}", sensor_relative_azimuth_angle);

        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new()
            .add_context(input, &checkpoint, 
                winnow::error::StrContext::Label("Misb0601")
            )
        ))
    }
}