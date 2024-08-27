use tinyklv::Klv;
use tinyklv::prelude::*;

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
    #[feature("misb0601-19")]
    #[klv(key = 0x01)]
    /// Checksum used to detect errors within a UAS Datalink LS packet
    pub checksum: u16,

    #[feature("misb0601-19")]
    #[klv(key = 0x02, dec = crate::dec::to_precision_timestamp)]
    /// Timestamp for all metadata in this Local Set; used to coordinate with Motion Imagery
    pub precision_timestamp: Option<chrono::DateTime<chrono::Utc>>,

    #[feature("misb0601-19")]
    #[klv(key = 0x03)]
    /// Descriptive mission identifier to distinguish event or sortie
    pub mission_id: Option<String>,

    #[feature("misb0601-19")]
    #[klv(key = 0x04)]
    /// Identifier of platform as posted
    pub platform_tail_number: Option<String>,

    #[feature("misb0601-19")]
    #[klv(key = 0x05, dec = crate::dec::to_platform_heading)]
    /// Aircraft heading angle
    pub platform_heading_angle: Option<f64>,

    #[feature("misb0601-19")]
    #[klv(key = 0x06, dec = crate::dec::to_platform_pitch)]
    /// Aircraft pitch angle
    pub platform_pitch_angle: Option<f64>,

    #[feature("misb0601-19")]
    #[klv(key = 0x07, dec = crate::dec::to_platform_roll)]
    /// Platform roll angle
    pub platform_roll_angle: Option<f64>,

    #[klv(key = 0x08)]
    /// True airspeed (TAS) of platform
    pub platform_true_airspeed: Option<u8>,

    #[klv(key = 0x09)]
    /// Indicated airspeed (IAS) of platform
    pub platform_indicated_airspeed: Option<u8>,

    #[klv(key = 0x0a)]
    /// Model name for the platform
    pub platform_designation: Option<String>,

    #[klv(key = 0x0b)]
    /// Name of currently active sensor
    pub image_source_sensor: Option<String>,

    #[klv(key = 0x0c)]
    /// Name of the image coordinate system used
    pub image_coordinate_system: Option<String>,

    #[klv(key = 0x0d, dec = crate::dec::to_lat)]
    /// Sensor latitude
    pub sensor_latitude: Option<f64>,

    #[klv(key = 0x0e, dec = crate::dec::to_lon)]
    /// Sensor longitude
    pub sensor_longitude: Option<f64>,

    #[klv(key = 0x0f, dec = crate::dec::to_true_alt)]
    /// Altitude of sensor above from Mean Sea Level (MSL)
    pub sensor_true_altitude: Option<f64>,

    #[klv(key = 0x10, dec = crate::dec::to_sensor_hvfov)]
    /// Horizontal field of view of selected imaging sensor
    pub sensor_hfov: Option<f64>,

    #[klv(key = 0x11, dec = crate::dec::to_sensor_hvfov)]
    /// Vertical field of view of selected imaging sensor
    pub sensor_vfov: Option<f64>,

    #[klv(key = 0x12, dec = crate::dec::to_sensor_rel_azm_rll_angle)]
    /// Relative rotation angle of sensor to platform longitudinal axis
    pub sensor_relative_azimuth_angle: Option<f64>,

    #[klv(key = 0x13, dec = crate::dec::to_sensor_rel_elv_angle)]
    /// Relative elevation angle of sensor to platform longitudinal-transverse plane
    pub sensor_relative_elevation_angle: Option<f64>,

    #[klv(key = 0x14, dec = crate::dec::to_sensor_rel_azm_rll_angle)]
    /// Relative roll angle of sensor to aircraft platform
    pub sensor_relative_roll_angle: Option<f64>,

    #[klv(key = 0x15, dec = crate::dec::to_slant_range)]
    /// Slant range in meters
    pub slant_range: Option<f64>,

    #[klv(key = 0x16, dec = crate::dec::to_target_width)]
    /// Target width within sensor field of view
    pub target_width: Option<f64>,

    #[klv(key = 0x17)]
    /// Terrain latitude of frame center
    pub frame_center_latitude: Option<i32>,

    #[klv(key = 0x18)]
    /// Terrain longitude of frame center
    pub frame_center_longitude: Option<i32>,

    #[klv(key = 0x19)]
    /// Terrain elevation at frame center relative to Mean Sea Level (MSL)
    pub frame_center_elevation: Option<u16>,

    // #[klv(key = 0x20)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x21)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x22)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x23)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x24)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x25)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x26)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x27)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x28)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x29)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = 0x30)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,

    // #[klv(key = xxxxxxxx)]
    // /// 
    // pub namenamenamenamename: Option<typetypetypetypetype>,
}