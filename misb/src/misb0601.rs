#![allow(non_upper_case_globals)]
// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::TimeZone;
// use winnow::prelude::*;
// use winnow::stream::Stream;
// use winnow::error::AddContext;

// --------------------------------------------------
// tinyklv
// --------------------------------------------------
use tinyklv::Klv;
use tinyklv::prelude::*;

#[cfg(any(
    feature = "misb0601-19",
))]
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
    default(ty = i8, dec = tinyklv::codecs::binary::dec::be_i8),
    default(ty = String, dec = tinyklv::codecs::binary::dec::to_string_utf8, dyn = true),
)]
/// UAS Datalink Local Set
/// 
/// MISB Standard 0601
/// 
/// For more information, see [Motion Imagery Standards Board (MISB)](https://nsgreg.nga.mil/misb.jsp)
pub struct Misb0601 {
    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x01)]
    /// (Mandatory) Checksum used to detect errors within a UAS Datalink LS packet
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub checksum: u16,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x02, dec = to_precision_timestamp)]
    /// (Mandatory) Timestamp for all metadata in this Local Set; used to coordinate with Motion Imagery
    /// 
    /// Units: Microseconds (μs)
    /// 
    /// Resolution: 1 μs
    pub precision_timestamp: chrono::DateTime<chrono::Utc>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x03)]
    /// (Optional) Descriptive mission identifier to distinguish event or sortie
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub mission_id: Option<String>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x04)]
    /// (Optional) Identifier of platform as posted
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub platform_tail_number: Option<String>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x05, dec = to_platform_heading_angle)]
    /// (Optional) Aircraft heading angle
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~5.5 millidegrees
    pub platform_heading_angle: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x06, dec = to_platform_pitch_angle)]
    /// (Optional) Aircraft pitch angle
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~610 microdegrees
    pub platform_pitch_angle: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x07, dec = to_platform_roll_angle)]
    /// (Optional) Platform roll angle
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1525 microdegrees
    pub platform_roll_angle: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x08)]
    /// (Optional) True airspeed (TAS) of platform
    /// 
    /// Units: Meters per second (m/s)
    /// 
    /// Resolution: 1 m/s
    pub platform_true_airspeed: Option<u8>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x09)]
    /// (Optional) Indicated airspeed (IAS) of platform
    /// 
    /// Units: Meters per second (m/s)
    /// 
    /// Resolution: 1 m/s
    pub platform_indicated_airspeed: Option<u8>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0a)]
    /// (Optional) Model name for the platform
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub platform_designation: Option<String>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0b)]
    /// (Optional) Name of currently active sensor
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub image_source_sensor: Option<String>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0c)]
    /// (Optional) Name of the image coordinate system used
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub image_coordinate_system: Option<String>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0d, dec = to_lat)]
    /// (Optional) Sensor latitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~42 nanodegrees
    pub sensor_latitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0e, dec = to_lon)]
    /// (Optional) Sensor longitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_longitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0f, dec = to_alt)]
    /// (Optional) Altitude of sensor above from Mean Sea Level (MSL)
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.3 meters
    pub sensor_true_altitude: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x10, dec = to_sensor_hvfov)]
    /// (Optional) Horizontal field of view of selected imaging sensor
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~2.7 millidegrees
    pub sensor_hfov: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x11, dec = to_sensor_hvfov)]
    /// (Optional) Vertical field of view of selected imaging sensor
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~2.7 millidegrees
    pub sensor_vfov: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x12, dec = to_sensor_relative_azimuth_angle)]
    /// (Optional) Relative rotation angle of sensor to platform longitudinal axis
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_relative_azimuth_angle: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x13, dec = to_sensor_relative_elevation_angle)]
    /// (Optional) Relative elevation angle of sensor to platform longitudinal-transverse plane
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_relative_elevation_angle: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x14, dec = to_sensor_relative_roll_angle)]
    /// (Optional) Relative roll angle of sensor to aircraft platform
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_relative_roll_angle: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x15, dec = to_slant_range)]
    /// (Optional) Slant range in meters
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~1.2 millimeters
    pub slant_range: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x16, dec = to_target_width)]
    /// (Optional) Target width within sensor field of view
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.16 meters
    pub target_width: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x17, dec = to_lat)]
    /// (Optional) Terrain latitude of frame center
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~42 nanodegrees
    pub frame_center_latitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x18, dec = to_lon)]
    /// (Optional) Terrain longitude of frame center
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub frame_center_longitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x19, dec = to_alt)]
    /// (Optional) Terrain elevation at frame center relative to Mean Sea Level (MSL)
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: 0.3 meters
    pub frame_center_elevation: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1a, dec = to_offset_ll)]
    /// (Optional) Frame latitude offset for upper left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p1: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1b, dec = to_offset_ll)]
    /// (Optional) Frame longitude offset for upper left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p1: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1c, dec = to_offset_ll)]
    /// (Optional) Frame latitude offset for upper right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p2: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1d, dec = to_offset_ll)]
    /// (Optional) Frame longitude offset for upper right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p2: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1e, dec = to_offset_ll)]
    /// (Optional) Frame latitude offset for lower right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p3: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1f, dec = to_offset_ll)]
    /// (Optional) Frame longitude offset for lower right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p3: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x20, dec = to_offset_ll)]
    /// (Optional) Frame latitude offset for lower left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p4: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x21, dec = to_offset_ll)]
    /// (Optional) Frame longitude offset for lower left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p4: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x22, dec = to_icing_detected)]
    /// (Optional) Flag for icing detected at aircraft location
    /// 
    /// Units: Icing Code (code)
    /// 
    /// Resolution: N/A
    pub icing_detected: Option<Icing>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x23, dec = to_wind_direction)]
    /// (Optional) Wind direction at aircraft location
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~5.5 millidegrees
    pub wind_direction: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x24, dec = to_wind_speed)]
    /// (Optional) Wind speed at aircraft location
    /// 
    /// Units: Meters per second (m/s)
    /// 
    /// Resolution: ~0.4 m/s
    pub wind_speed: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x25, dec = to_mbar_pressure)]
    /// (Optional) Static pressure at aircraft location
    /// 
    /// Units: Millibars (mbar)
    /// 
    /// Resolution: ~0.01 mbar
    pub static_pressure: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x26, dec = to_alt)]
    /// (Optional) Density altitude at aircraft location
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.3 meters
    pub density_altitude: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x27)]
    /// (Optional) Temperature outside of aircraft
    /// 
    /// Units: Celsius (°C)
    /// 
    /// Resolution: 1 °C
    pub outside_air_temperature: Option<i8>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x28, dec = to_lat)]
    /// (Optional) Calculated target latitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~42 nanodegrees
    pub target_location_latitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x29, dec = to_lon)]
    /// (Optional) Calculated target longitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub target_location_longitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2a, dec = to_alt)]
    /// (Optional) Calculated target altitude
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.3 meters
    pub target_location_elevation: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2b, dec = to_target_track_gate_hw)]
    /// (Optional) Tracking gate width (x value) of tracked target within field of view
    /// 
    /// Units: Pixels
    /// 
    /// Resolution: 2 pixels
    pub target_track_gate_width: Option<u16>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2c, dec = to_target_track_gate_hw)]

    /// (Optional) Tracking gate height (y value) of tracked target within field of view
    /// 
    /// Units: Pixels
    /// 
    /// Resolution: 2 pixels
    pub target_track_gate_height: Option<u16>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2d, dec = to_error_estimate)]
    /// (Optional) Circular error 90 (CE90) is the estimated error distance in the horizontal direction
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.0624 meters
    pub target_error_estimate_ce90: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2e, dec = to_error_estimate)]
    /// (Optional) Lateral error 90 (LE90) is the estimated error distance in the vertical (or lateral) direction
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: 0.0625 meters
    pub target_error_estimate_le90: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2f, dec = GenericFlagData::decode)]
    /// (Optional) Generic metadata flags
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub generic_flag_data: Option<GenericFlagData>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x30, dec = crate::misb0102::Misb0102LocalSet::decode)]
    /// (Optional) MISB ST 0102 local let Security Metadata items
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub security_local_set: Option<crate::misb0102::Misb0102LocalSet>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x31, dec = to_mbar_pressure)]
    /// (Optional) Differential pressure at aircraft location
    /// 
    /// Units: Millibar (mbar)
    /// 
    /// Resolution: ~0.08 mbar
    pub differential_pressure: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x32, dec = to_platform_pitch_angle)]
    /// (Optional) Platform attack angle
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~610 microdegrees
    pub platform_angle_of_attack: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x33, dec = to_platform_vertical_speed)]
    /// (Optional) Vertical speed of the aircraft relative to zenith
    /// 
    /// Units: Meters per second (m/s)
    /// 
    /// Resolution: ~0.0055 m/s
    pub platform_vertical_speed: Option<f32>,

    // #[cfg(any(
    //     feature = "misb0601-19",
    // ))]
    // #[klv(key = 0x34)]
    // pub platform_sideslip_angle: Option<f32>,
}

#[derive(Debug, PartialEq)]
/// Icing status on the aircraft (i.e., the wings). Icing on
/// wings can affect the continuation of the mission
pub enum Icing {
    DetectorOff,
    NoIcingDetected,
    IcingDetected,
}

#[derive(Debug, PartialEq)]
/// IR sensor images use either black values indicating
/// hot or white values indicating hot
pub enum IrPolarity {
    BlackHot,
    WhiteHot,
}

#[derive(Debug, PartialEq)]
/// Slant range is measured (i.e., using Laser Range
/// Finder) or calculated using gimbal/aircraft position
/// and angles
pub enum SlantRangeSource {
    Measured,
    Calculated,
}
#[derive(Debug, PartialEq)]
/// See [`crate::misb0601::Misb0601::generic_flag_data`]
pub struct GenericFlagData {
    /// Laser Range Finder can be used to aid in geopositioning
    /// 
    /// Indicates whether or not laser range finder is on
    pub laser_range_on: bool,
    /// Sensor steering is automatically controlled by onboard tracking system
    /// 
    /// Indicates whether or not sensor steering is on
    pub auto_track_on: bool,
    /// Indicates IR polarity
    pub ir_polarity: IrPolarity,
    /// Indicates icing status
    pub icing_status: Icing,
    /// Indicates if slant range is measured or calculated
    pub slant_range_source: SlantRangeSource,
    /// An invalid image may result from a lens change,
    /// bad focus or other camera issues which
    /// significantly degrades the image
    /// 
    /// Indicates if image is invalid
    pub is_image_invalid: bool,
}
#[cfg(feature = "misb0601-19")]
/// [`GenericFlagData`] implementation of [`tinyklv::prelude::Decode`]
impl tinyklv::prelude::Decode<&[u8]> for GenericFlagData {
    /// See [`crate::misb0601::Misb0601::generic_flag_data`]
    fn decode(input: &mut &[u8]) -> winnow::PResult<Self> {
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
}

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
pub const SFT_2_MBAR_PRESSURE: f32 = 65535.0 / 5000.0;
pub const KLV_2_MBAR_PRESSURE: f32 = 5000.0 / 65535.0;
pub const SFT_2_ERROR_ESTIMATE: f32 = 65535.0 / 4095.0;
pub const KLV_2_ERROR_ESTIMATE: f32 = 4095.0 / 65535.0;
pub const SFT_2_PLATFORM_VERT_SPEED: f32 = 65534.0 / 360.0;
pub const KLV_2_PLATFORM_VERT_SPEED: f32 = 360.0 / 65534.0;

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
/// use misb::misb0601::to_precision_timestamp;
/// let mut val1: &[u8] = &(0x0004_59F4_A6AA_4AA8 as u64).to_be_bytes();
/// let result1 = to_precision_timestamp(&mut val1);
/// assert_eq!(result1, Ok(chrono::Utc.with_ymd_and_hms(2008, 10, 24, 0, 13, 29).unwrap() + chrono::Duration::milliseconds(913)));
/// ```
pub fn to_precision_timestamp(input: &mut &[u8]) -> winnow::PResult<chrono::DateTime<chrono::Utc>> {
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
/// * [`crate::misb0601::Misb0601::sensor_latitude`]
/// * [`crate::misb0601::Misb0601::frame_center_latitude`]
pub(crate) fn to_lat(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i32.parse_next(input)?;
    if value as u32 == 0x8000_0000 { return Err(tinyklv::err!()) } // "Reserved" - keep for backwards compatibility
    Ok((value as f64) * KLV_2_LAT)
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::sensor_longitude`]
/// * [`crate::misb0601::Misb0601::frame_center_longitude`]
pub(crate) fn to_lon(input: &mut &[u8]) -> winnow::PResult<f64> {
    let value = tinyklv::codecs::binary::dec::be_i32.parse_next(input)?;
    if value as u32 == 0x8000_0000 { return Err(tinyklv::err!()) } // "Reserved" - keep for backwards compatibility
    Ok((value as f64) * KLV_2_LON)
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::sensor_true_altitude`]
/// * [`crate::misb0601::Misb0601::frame_center_elevation`]
pub(crate) fn to_alt(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_u16.parse_next(input)?;
    Ok((value as f32 * KLV_2_SENSOR_TRUE_ALT_P1) - SENSOR_TRUE_ALT_OFFSET_P2)
}


#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::platform_heading_angle`]
pub(crate) const to_platform_heading_angle: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_PLATFORM_HEADING
);

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::platform_pitch_angle`]
pub(crate) fn to_platform_pitch_angle(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    if value as u32 == 0x8000 { return Err(tinyklv::err!()) } // "Out of Range" - keep for backwards compatibility
    Ok((value as f32) * KLV_2_PLATFORM_PITCH)
}

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::platform_roll_angle`]
pub(crate) fn to_platform_roll_angle(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    if value as u32 == 0x8000 { return Err(tinyklv::err!()) } // "Out of Range" - keep for backwards compatibility
    Ok((value as f32) * KLV_2_PLATFORM_ROLL)
}

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::sensor_hfov`]
/// * [`crate::misb0601::Misb0601::sensor_vfov`]
pub(crate) const to_sensor_hvfov: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_SENSOR_HVFOV
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::sensor_relative_azimuth_angle`]
/// 
/// Same as [`to_sensor_relative_roll_angle`]
pub(crate) const to_sensor_relative_azimuth_angle: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u32,
    f64,
    KLV_2_SENSOR_REL_AZM_RLL_ANGLE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::sensor_relative_elevation_angle`]
pub(crate) const to_sensor_relative_elevation_angle: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_i32,
    f64,
    KLV_2_SENSOR_REL_ELV_ANGLE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::sensor_relative_roll_angle`]
/// 
/// Same as [`to_sensor_relative_azimuth_angle`]
pub(crate) const to_sensor_relative_roll_angle: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u32,
    f64,
    KLV_2_SENSOR_REL_AZM_RLL_ANGLE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::slant_range`]
pub(crate) const to_slant_range: fn(&mut &[u8]) -> winnow::PResult<f64> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u32,
    f64,
    KLV_2_SLANT_RANGE
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::target_width`]
pub(crate) const to_target_width: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_TARGET_WIDTH
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::offset_corner_lat_p1`]
/// * [`crate::misb0601::Misb0601::offset_corner_lon_p1`]
/// * [`crate::misb0601::Misb0601::offset_corner_lat_p2`]
/// * [`crate::misb0601::Misb0601::offset_corner_lon_p2`]
/// * [`crate::misb0601::Misb0601::offset_corner_lat_p3`]
/// * [`crate::misb0601::Misb0601::offset_corner_lon_p3`]
/// * [`crate::misb0601::Misb0601::offset_corner_lat_p4`]
/// * [`crate::misb0601::Misb0601::offset_corner_lon_p4`]
pub(crate) const to_offset_ll: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_i16,
    f32,
    KLV_2_OFFSET_LL,
);

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::icing_detected`]
pub(crate) fn to_icing_detected(input: &mut &[u8]) -> winnow::PResult<crate::misb0601::Icing> {
    match tinyklv::codecs::binary::dec::be_u8.parse_next(input)? {
        0 => Ok(crate::misb0601::Icing::DetectorOff),
        1 => Ok(crate::misb0601::Icing::NoIcingDetected),
        2 => Ok(crate::misb0601::Icing::IcingDetected),
        _ => Err(tinyklv::err!()),
    }
}

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::wind_direction`]
pub(crate) const to_wind_direction: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_WIND_DIRECTION
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::wind_speed`]
pub(crate) const to_wind_speed: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u8,
    f32,
    KLV_2_WIND_SPEED,
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::static_pressure`]
/// * [`crate::misb0601::Misb0601::differential_pressure`]
pub(crate) const to_mbar_pressure: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_MBAR_PRESSURE,
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::target_track_gate_width`]
/// * [`crate::misb0601::Misb0601::target_track_gate_height`]
pub(crate) const to_target_track_gate_hw: fn(&mut &[u8]) -> winnow::PResult<u16> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u8,
    u16,
    2,
);

#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601`]
/// 
/// * [`crate::misb0601::Misb0601::target_error_estimate_ce90`]
/// * [`crate::misb0601::Misb0601::target_error_estimate_le90`]
pub(crate) const to_error_estimate: fn(&mut &[u8]) -> winnow::PResult<f32> = tinyklv::scale!(
    tinyklv::codecs::binary::dec::be_u16,
    f32,
    KLV_2_ERROR_ESTIMATE,
);

#[inline(always)]
#[cfg(feature = "misb0601-19")]
/// See [`crate::misb0601::Misb0601::platform_vertical_speed`]
pub(crate) fn to_platform_vertical_speed(input: &mut &[u8]) -> winnow::PResult<f32> {
    let value = tinyklv::codecs::binary::dec::be_i16.parse_next(input)?;
    if value as u32 == 0x8000 { return Err(tinyklv::err!()) } // "Out of Range" - keep for backwards compatibility
    Ok((value as f32) * KLV_2_PLATFORM_VERT_SPEED)
}