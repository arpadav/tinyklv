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
const SFT_2_LAT: f64 = 4294967294.0 / 180.0;
const KLV_2_LAT: f64 = 180.0 / 4294967294.0;
const SFT_2_LON: f64 = 4294967294.0 / 360.0;
const KLV_2_LON: f64 = 360.0 / 4294967294.0;
const KLV_2_PLATFORM_HEADING: f64 = 360.0 / 65535.0;
const SFT_2_PLATFORM_HEADING: f64 = 65535.0 / 360.0;
const KLV_2_PLATFORM_PITCH: f64 = 40.0 / 65534.0;
const SFT_2_PLATFORM_PITCH: f64 = 65534.0 / 40.0;
const KLV_2_PLATFORM_ROLL: f64 = 100.0 / 65534.0;
const SFT_2_PLATFORM_ROLL: f64 = 65534.0 / 100.0;
const KLV_2_SENSOR_TRUE_ALT_P1: f64 = 19900.0 / 65535.0;
const SFT_2_SENSOR_TRUE_ALT_P1: f64 = 65535.0 / 19900.0;
const SENSOR_TRUE_ALT_OFFSET_P2: f64 = 900.0;
const SFT_2_SENSOR_HVFOV: f64 = 65535.0 / 180.0;
const KLV_2_SENSOR_HVFOV: f64 = 180.0 / 65535.0;
const SFT_2_SENSOR_REL_AZM_RLL_ANGLE: f64 = 4294967295.0 / 360.0;
const KLV_2_SENSOR_REL_AZM_RLL_ANGLE: f64 = 360.0 / 4294967295.0;
const SFT_2_SENSOR_REL_ELV_ANGLE: f64 = 4294967294.0 / 360.0;
const KLV_2_SENSOR_REL_ELV_ANGLE: f64 = 360.0 / 4294967294.0;
const SFT_2_SLANT_RANGE: f64 = 4294967295.0 / 5_000_000.0;
const KLV_2_SLANT_RANGE: f64 = 5_000_000.0 / 4294967295.0;
const SFT_2_TARGET_WIDTH: f64 = 65535.0 / 10_000.0;
const KLV_2_TARGET_WIDTH: f64 = 10_000.0 / 65535.0;

#[inline(always)]
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
        chrono::LocalResult::None => Err(tinyklv::blank_err!().add_context(
            input,
            &checkpoint,
            winnow::error::StrContext::Label("Invalid timestamp")
        )),
        chrono::LocalResult::Ambiguous(_, _) => Err(tinyklv::blank_err!()),
    }
}

#[inline(always)]
pub(crate) fn to_lat(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i32.parse_next(input)?;
    // "Reserved" - keep for backwards compatibility
    if value as u32 == 0x8000_0000 { return Err(tinyklv::blank_err!()) }
    Ok((value as f64) * KLV_2_LAT)
}

#[inline(always)]
pub(crate) fn to_lon(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i32.parse_next(input)?;
    // "Reserved" - keep for backwards compatibility
    if value as u32 == 0x8000_0000 { return Err(tinyklv::blank_err!()) }
    Ok((value as f64) * KLV_2_LON)
}

#[inline(always)]
pub(crate) fn to_platform_heading(input: &mut &[u8]) -> winnow::PResult<f64> {
    tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING)
}

#[inline(always)]
pub(crate) fn to_platform_pitch(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    // "Out of Range" - keep for backwards compatibility
    if value as u32 == 0x8000 { return Err(tinyklv::blank_err!()) }
    Ok((value as f64) * KLV_2_PLATFORM_PITCH)
}

#[inline(always)]
pub(crate) fn to_platform_roll(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    // "Out of Range" - keep for backwards compatibility
    if value as u32 == 0x8000 { return Err(tinyklv::blank_err!()) }
    Ok((value as f64) * KLV_2_PLATFORM_ROLL)
}

#[inline(always)]
pub(crate) fn to_true_alt(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_u16.parse_next(input)?;
    Ok((value as f64 * KLV_2_SENSOR_TRUE_ALT_P1) - SENSOR_TRUE_ALT_OFFSET_P2)
}

#[inline(always)]
pub(crate) fn to_sensor_hvfov(input: &mut &[u8]) -> winnow::PResult<f64> {
    tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_SENSOR_HVFOV)
}

#[inline(always)]
pub(crate) fn to_sensor_rel_azm_rll_angle(input: &mut &[u8]) -> winnow::PResult<f64> {
    tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_u32, f64, KLV_2_SENSOR_REL_AZM_RLL_ANGLE)
}

#[inline(always)]
pub(crate) fn to_sensor_rel_elv_angle(input: &mut &[u8]) -> winnow::PResult<f64> {
    tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_i32, f64, KLV_2_SENSOR_REL_ELV_ANGLE)
}

#[inline(always)]
pub(crate) fn to_slant_range(input: &mut &[u8]) -> winnow::PResult<f64> {
    tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_u32, f64, KLV_2_SLANT_RANGE)
}

#[inline(always)]
pub(crate) fn to_target_width(input: &mut &[u8]) -> winnow::PResult<f64> {
    tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_TARGET_WIDTH)
}