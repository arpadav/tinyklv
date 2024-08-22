#[derive(Klv)]
pub struct Misb0601 {
    #[klv(key = b"\x01", dec = winnow::binary::be_u16)]
    pub checksum: u16,

    #[klv(key = b"\x02", dec = dec::precision_timestamp)]
    pub precision_timestamp: chrono::DateTime<chrono::Utc>,

    #[klv(key = b"\x03", dec = tinyklv::defaults::dec::to_string)]
    pub mission_id: String,

    #[klv(key = b"\x04", dec = tinyklv::defaults::dec::to_string)]
    pub platform_tail_numer: String,

    #[klv(key = b"\x05", dec = winnow::binary::be_u16)]
    pub platform_heading_angle: u16,

    #[klv(key = b"\x06", dec = winnow::binary::be_i16)]
    pub platform_pitch_angle: i16,

    #[klv(key = b"\x07", dec = winnow::binary::be_i16)]
    pub platform_roll_angle: i16,

    #[klv(key = b"\x08", dec = winnow::binary::be_u8)]
    pub platform_true_airspeed: u8,

    #[klv(key = b"\x09", dec = winnow::binary::be_u8)]
    pub platform_indicated_airspeed: u8,

    #[klv(key = b"\x0a", dec = tinyklv::defaults::dec::to_string)]
    pub platform_designation: String,

    #[klv(key = b"\x0b", dec = tinyklv::defaults::dec::to_string)]
    pub image_source_sensor: String,

    #[klv(key = b"\x0c", dec = tinyklv::defaults::dec::to_string)]
    pub image_coordinate_system: String,

    #[klv(key = b"\x0d", dec = winnow::binary::be_i32)]
    pub sensor_latitude: i32,

    #[klv(key = b"\x0e", dec = winnow::binary::be_i32)]
    pub sensor_longitude: i32,

    #[klv(key = b"\x0f", dec = winnow::binary::be_u16)]
    /// Altitude of sensor above MSL (mean sea level).
    pub sensor_true_altitude: u16,

    #[klv(key = b"\x10", dec = winnow::binary::be_u16)]
    /// Horizontal field of view of selected imaging sensor
    pub sensor_hfov: u16,

    #[klv(key = b"\x11", dec = winnow::binary::be_u16)]
    /// Vertical field of view of selected imaging sensor
    pub sensor_vfov: u16,

    #[klv(key = b"\x12", dec = winnow::binary::be_u16)]
    /// Relative rotation angle of sensor to platform longitudinal axis
    pub sensor_relative_azimuth_angle: u32,
}