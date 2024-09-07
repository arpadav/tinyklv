#![allow(dead_code)]
#![allow(non_upper_case_globals)]

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::TimeZone;
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::error::AddContext;

// --------------------------------------------------
// constants
// --------------------------------------------------
// it is known that some values are repeated
// --------------------------------------------------
pub const SFT_2_LAT: f64 = 4294967294.0 / 180.0;
pub const KLV_2_LAT: f64 = 180.0 / 4294967294.0;
pub const SFT_2_LON: f64 = 4294967294.0 / 360.0;
pub const KLV_2_LON: f64 = 360.0 / 4294967294.0;
pub const KLV_2_PLATFORM_HEADING: f32 = 360.0 / 65535.0;
pub const SFT_2_PLATFORM_HEADING: f32 = 65535.0 / 360.0;
pub const KLV_2_PLATFORM_PITCH: f32 = 40.0 / 65534.0;
pub const SFT_2_PLATFORM_PITCH: f32 = 65534.0 / 40.0;
pub const KLV_2_PLATFORM_ROLL: f32 = 100.0 / 65534.0;
pub const SFT_2_PLATFORM_ROLL: f32 = 65534.0 / 100.0;
pub const KLV_2_SENSOR_TRUE_ALT_P1: f32 = 19900.0 / 65535.0;
pub const SFT_2_SENSOR_TRUE_ALT_P1: f32 = 65535.0 / 19900.0;
pub const SENSOR_TRUE_ALT_OFFSET_P2: f32 = 900.0;
pub const SFT_2_SENSOR_HVFOV: f32 = 65535.0 / 180.0;
pub const KLV_2_SENSOR_HVFOV: f32 = 180.0 / 65535.0;
pub const SFT_2_SENSOR_REL_AZM_RLL_ANGLE: f64 = 4294967295.0 / 360.0;
pub const KLV_2_SENSOR_REL_AZM_RLL_ANGLE: f64 = 360.0 / 4294967295.0;
pub const SFT_2_SENSOR_REL_ELV_ANGLE: f64 = 4294967294.0 / 360.0;
pub const KLV_2_SENSOR_REL_ELV_ANGLE: f64 = 360.0 / 4294967294.0;
pub const SFT_2_SLANT_RANGE: f64 = 4294967295.0 / 5_000_000.0;
pub const KLV_2_SLANT_RANGE: f64 = 5_000_000.0 / 4294967295.0;
pub const SFT_2_TARGET_WIDTH: f32 = 65535.0 / 10_000.0;
pub const KLV_2_TARGET_WIDTH: f32 = 10_000.0 / 65535.0;
pub const SFT_2_OFFSET_LL: f32 = 65534.0 / 0.15;
pub const KLV_2_OFFSET_LL: f32 = 0.15 / 65534.0;
pub const SFT_2_WIND_DIRECTION: f32 = 65535.0 / 360.0;
pub const KLV_2_WIND_DIRECTION: f32 = 360.0 / 65535.0;
pub const SFT_2_WIND_SPEED: f32 = 255.0 / 100.0;
pub const KLV_2_WIND_SPEED: f32 = 100.0 / 255.0;
pub const SFT_2_STATIC_PRESSURE: f32 = 65535.0 / 5000.0;
pub const KLV_2_STATIC_PRESSURE: f32 = 5000.0 / 65535.0;
pub const SFT_2_ERROR_ESTIMATE: f32 = 65535.0 / 4095.0;
pub const KLV_2_ERROR_ESTIMATE: f32 = 4095.0 / 65535.0;

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// Represents the number of MICROSECONDS elapsed since midnight
/// (00:00:00), January 1, 1970, not including leap seconds.
/// 
/// # Example
/// 
/// ```
/// use chrono::TimeZone;
/// use tinyklv::prelude::*;
/// use tinyklv::misb::dec::to_precision_timestamp;
/// let mut val1: &[u8] = &(0x0004_59F4_A6AA_4AA8 as u64).to_be_bytes();
/// let result1 = to_precision_timestamp(&mut val1);
/// assert_eq!(result1, Ok(chrono::Utc.with_ymd_and_hms(2008, 10, 24, 0, 13, 29).unwrap() + chrono::Duration::milliseconds(913)));
/// ```
pub(crate) fn to_precision_timestamp(input: &mut &[u8]) -> winnow::PResult<chrono::DateTime<chrono::Utc>> {
    let checkpoint = input.checkpoint();
    // time in microseconds
    let ts = winnow::binary::be_u64.parse_next(input)?; 
    // time in seconds, time in nanoseconds
    let (ts, tns) = (ts / 1_000_000, (ts % 1_000_000) * 1_000);
    // convert to UTC
    match chrono::Utc.timestamp_opt(ts as i64, tns as u32) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        chrono::LocalResult::None => Err(tinyklv::err!().add_context(
            input,
            &checkpoint,
            winnow::error::StrContext::Label("Invalid timestamp")
        )),
        chrono::LocalResult::Ambiguous(_, _) => Err(tinyklv::err!()),
    }
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `sensor_latitude`
/// * `frame_center_latitude`
pub(crate) fn to_lat(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i32.parse_next(input)?;
    if value as u32 == 0x8000_0000 { return Err(tinyklv::err!()) } // "Reserved" - keep for backwards compatibility
    Ok((value as f64) * KLV_2_LAT)
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `sensor_longitude`
/// * `frame_center_longitude`
pub(crate) fn to_lon(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i32.parse_next(input)?;
    if value as u32 == 0x8000_0000 { return Err(tinyklv::err!()) } // "Reserved" - keep for backwards compatibility
    Ok((value as f64) * KLV_2_LON)
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `sensor_true_altitude`
/// * `frame_center_elevation`
pub(crate) fn to_alt(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_u16.parse_next(input)?;
    Ok((value as f32 * KLV_2_SENSOR_TRUE_ALT_P1) - SENSOR_TRUE_ALT_OFFSET_P2)
}


#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `platform_heading_angle`
pub(crate) const to_platform_heading_angle: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_PLATFORM_HEADING
);

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `platform_pitch_angle`
pub(crate) fn to_platform_pitch_angle(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    if value as u32 == 0x8000 { return Err(tinyklv::err!()) } // "Out of Range" - keep for backwards compatibility
    Ok((value as f32) * KLV_2_PLATFORM_PITCH)
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `platform_roll_angle`
pub(crate) fn to_platform_roll_angle(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    if value as u32 == 0x8000 { return Err(tinyklv::err!()) } // "Out of Range" - keep for backwards compatibility
    Ok((value as f32) * KLV_2_PLATFORM_ROLL)
}

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `sensor_hfov`
/// * `sensor_vfov`
pub(crate) const to_sensor_hvfov: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_SENSOR_HVFOV
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `sensor_relative_azimuth_angle`
/// 
/// Same as [`to_sensor_relative_roll_angle`]
pub(crate) const to_sensor_relative_azimuth_angle: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u32,
    f64,
    KLV_2_SENSOR_REL_AZM_RLL_ANGLE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `sensor_relative_elevation_angle`
// pub(crate) fn to_sensor_relative_elevation_angle(input: &mut &[u8]) -> winnow::PResult<f64> {
//     tinyklv::scale!(
//         tinyklv::codecs::binary::dec::be_i32,
//         f64,
//         KLV_2_SENSOR_REL_ELV_ANGLE
//     )(input)
// }
pub(crate) const to_sensor_relative_elevation_angle: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_i32,
    f64,
    KLV_2_SENSOR_REL_ELV_ANGLE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `sensor_relative_roll_angle`
/// 
/// Same as [`to_sensor_relative_azimuth_angle`]
pub(crate) const to_sensor_relative_roll_angle: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u32,
    f64,
    KLV_2_SENSOR_REL_AZM_RLL_ANGLE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `slant_range`
pub(crate) const to_slant_range: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u32,
    f64,
    KLV_2_SLANT_RANGE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `target_width`
pub(crate) const to_target_width: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_TARGET_WIDTH
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `offset_corner_lat_p1`
/// * `offset_corner_lon_p1`
/// * `offset_corner_lat_p2`
/// * `offset_corner_lon_p2`
/// * `offset_corner_lat_p3`
/// * `offset_corner_lon_p3`
/// * `offset_corner_lat_p4`
/// * `offset_corner_lon_p4`
pub(crate) const to_offset_ll: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_i16,
    f32,
    KLV_2_OFFSET_LL,
);

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `icing_detected`
pub(crate) fn to_icing_detected(input: &mut &[u8]) -> winnow::PResult<crate::misb0601::Icing> {
    match tinyklv::codecs::binary::dec::be_u8.parse_next(input)? {
        0 => Ok(crate::misb0601::Icing::DetectorOff),
        1 => Ok(crate::misb0601::Icing::NoIcingDetected),
        2 => Ok(crate::misb0601::Icing::IcingDetected),
        _ => Err(tinyklv::err!()),
    }
}

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `wind_direction`
pub(crate) const to_wind_direction: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_WIND_DIRECTION
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `wind_speed`
pub(crate) const to_wind_speed: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u8,
    f32,
    KLV_2_WIND_SPEED,
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `static_pressure`
pub(crate) const to_static_pressure: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_STATIC_PRESSURE,
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `target_track_gate_width`
/// * `target_track_gate_height`
pub(crate) const to_target_track_gate_hw: fn(&mut &[u8]) -> winnow::PResult<u16> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u8,
    u16,
    2,
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * `target_error_estimate_ce90`
/// * `target_error_estimate_le90`
pub(crate) const to_error_estimate: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_ERROR_ESTIMATE,
);

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`] `generic_flag_data`
pub(crate) fn to_generic_flag_data(input: &mut &[u8]) -> winnow::PResult<crate::misb0601::GenericFlagData> {
    let value = tinyklv::codecs::binary::dec::be_u8.parse_next(input)?;
    Ok(crate::misb0601::GenericFlagData {
        laser_range_on: (value >> 0) & 1 == 1,
        auto_track_on: (value >> 1) & 1 == 1,
        ir_polarity: match (value >> 2) & 1 == 1 {
            false => crate::misb0601::IrPolarity::WhiteHot,
            true => crate::misb0601::IrPolarity::BlackHot,
        },
        icing_status: match (value >> 3) & 1 == 1 {
            false => crate::misb0601::Icing::NoIcingDetected,
            true => crate::misb0601::Icing::IcingDetected,
        },
        slant_range_source: match (value >> 4) & 1 == 1 {
            false => crate::misb0601::SlantRangeSource::Calculated,
            true => crate::misb0601::SlantRangeSource::Measured,
        },
        is_image_invalid: (value >> 5) & 1 == 1,
    })
}