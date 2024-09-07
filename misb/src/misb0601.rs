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
    default(ty = u32, dec = tinyklv::codecs::binary::dec::be_u32),
    default(ty = i8, dec = tinyklv::codecs::binary::dec::be_i8),
    default(ty = i16, dec = tinyklv::codecs::binary::dec::be_i16),
    default(ty = i32, dec = tinyklv::codecs::binary::dec::be_i32),
    default(ty = String, dec = tinyklv::codecs::binary::dec::to_string, dyn = true),
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
    #[klv(key = 0x02, dec = crate::dec::to_precision_timestamp)]
    /// (Mandatory) Timestamp for all metadata in this Local Set; used to coordinate with Motion Imagery
    /// 
    /// Units: Microseconds (μs)
    /// 
    /// Resolution: 1 μs
    pub precision_timestamp: Option<chrono::DateTime<chrono::Utc>>,

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
    #[klv(key = 0x05, dec = crate::dec::to_platform_heading_angle)]
    /// (Optional) Aircraft heading angle
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~5.5 millidegrees
    pub platform_heading_angle: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x06, dec = crate::dec::to_platform_pitch_angle)]
    /// (Optional) Aircraft pitch angle
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~610 microdegrees
    pub platform_pitch_angle: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x07, dec = crate::dec::to_platform_roll_angle)]
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
    #[klv(key = 0x0d, dec = crate::dec::to_lat)]
    /// (Optional) Sensor latitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~42 nanodegrees
    pub sensor_latitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0e, dec = crate::dec::to_lon)]
    /// (Optional) Sensor longitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_longitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x0f, dec = crate::dec::to_alt)]
    /// (Optional) Altitude of sensor above from Mean Sea Level (MSL)
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.3 meters
    pub sensor_true_altitude: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x10, dec = crate::dec::to_sensor_hvfov)]
    /// (Optional) Horizontal field of view of selected imaging sensor
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~2.7 millidegrees
    pub sensor_hfov: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x11, dec = crate::dec::to_sensor_hvfov)]
    /// (Optional) Vertical field of view of selected imaging sensor
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~2.7 millidegrees
    pub sensor_vfov: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x12, dec = crate::dec::to_sensor_relative_azimuth_angle)]
    /// (Optional) Relative rotation angle of sensor to platform longitudinal axis
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_relative_azimuth_angle: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x13, dec = crate::dec::to_sensor_relative_elevation_angle)]
    /// (Optional) Relative elevation angle of sensor to platform longitudinal-transverse plane
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_relative_elevation_angle: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x14, dec = crate::dec::to_sensor_relative_roll_angle)]
    /// (Optional) Relative roll angle of sensor to aircraft platform
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub sensor_relative_roll_angle: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x15, dec = crate::dec::to_slant_range)]
    /// (Optional) Slant range in meters
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~1.2 millimeters
    pub slant_range: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x16, dec = crate::dec::to_target_width)]
    /// (Optional) Target width within sensor field of view
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.16 meters
    pub target_width: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x17, dec = crate::dec::to_lat)]
    /// (Optional) Terrain latitude of frame center
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~42 nanodegrees
    pub frame_center_latitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x18, dec = crate::dec::to_lon)]
    /// (Optional) Terrain longitude of frame center
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub frame_center_longitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x19, dec = crate::dec::to_alt)]
    /// (Optional) Terrain elevation at frame center relative to Mean Sea Level (MSL)
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: 0.3 meters
    pub frame_center_elevation: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1a, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame latitude offset for upper left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p1: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1b, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame longitude offset for upper left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p1: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1c, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame latitude offset for upper right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p2: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1d, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame longitude offset for upper right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p2: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1e, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame latitude offset for lower right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p3: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x1f, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame longitude offset for lower right corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p3: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x20, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame latitude offset for lower left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lat_p4: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x21, dec = crate::dec::to_offset_ll)]
    /// (Optional) Frame longitude offset for lower left corner
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~1.2 microdegrees
    pub offset_corner_lon_p4: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x22, dec = crate::dec::to_icing_detected)]
    /// (Optional) Flag for icing detected at aircraft location
    /// 
    /// Units: Icing Code (code)
    /// 
    /// Resolution: N/A
    pub icing_detected: Option<Icing>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x23, dec = crate::dec::to_wind_direction)]
    /// (Optional) Wind direction at aircraft location
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~5.5 millidegrees
    pub wind_direction: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x24, dec = crate::dec::to_wind_speed)]
    /// (Optional) Wind speed at aircraft location
    /// 
    /// Units: Meters per second (m/s)
    /// 
    /// Resolution: ~0.4 m/s
    pub wind_speed: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x25, dec = crate::dec::to_static_pressure)]
    /// (Optional) Static pressure at aircraft location
    /// 
    /// Units: Millibars (mbar)
    /// 
    /// Resolution: ~0.01 mbar
    pub static_pressure: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x26, dec = crate::dec::to_alt)]
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
    #[klv(key = 0x28, dec = crate::dec::to_lat)]
    /// (Optional) Calculated target latitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~42 nanodegrees
    pub target_location_latitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x29, dec = crate::dec::to_lon)]
    /// (Optional) Calculated target longitude
    /// 
    /// Units: Degrees (°)
    /// 
    /// Resolution: ~84 nanodegrees
    pub target_location_longitude: Option<f64>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2a, dec = crate::dec::to_alt)]
    /// (Optional) Calculated target altitude
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.3 meters
    pub target_location_elevation: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2b, dec = crate::dec::to_target_track_gate_hw)]
    /// (Optional) Tracking gate width (x value) of tracked target within field of view
    /// 
    /// Units: Pixels
    /// 
    /// Resolution: 2 pixels
    pub target_track_gate_width: Option<u16>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2c, dec = crate::dec::to_target_track_gate_hw)]

    /// (Optional) Tracking gate height (y value) of tracked target within field of view
    /// 
    /// Units: Pixels
    /// 
    /// Resolution: 2 pixels
    pub target_track_gate_height: Option<u16>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2d, dec = crate::dec::to_error_estimate)]
    /// (Optional) Circular error 90 (CE90) is the estimated error distance in the horizontal direction
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: ~0.0624 meters
    pub target_error_estimate_ce90: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2d, dec = crate::dec::to_error_estimate)]
    /// (Optional) Lateral error 90 (LE90) is the estimated error distance in the vertical (or lateral) direction
    /// 
    /// Units: Meters (m)
    /// 
    /// Resolution: 0.0625 meters
    pub target_error_estimate_le90: Option<f32>,

    #[cfg(any(
        feature = "misb0601-19",
    ))]
    #[klv(key = 0x2e, dec = crate::dec::to_generic_flag_data)]
    /// (Optional) Generic metadata flags
    /// 
    /// Units: None
    /// 
    /// Resolution: N/A
    pub generic_flag_data: Option<GenericFlagData>,
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
/// See [`crate::misb0601::Misb0601`] `generic_flag_data`
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
    pub slant_range_source: SlantRangeSource,
    pub is_image_invalid: bool,
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