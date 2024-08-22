use tinyklv::Klv;

#[derive(Klv)]
#[klv(
    // sentinel = b"\x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00",
    key(enc = ::tinyklv::enc::ber_oid, dec = ::tinyklv::dec::ber_oid),
    len(enc = ::tinyklv::enc::ber_length, dec = ::tinyklv::dec::ber_length),
    default(ty = String, dyn = true, dec = ::tinyklv::dec::to_string),
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

fn main() {
    let size = b"\x01".len();
    println!("size: {}", size);
}
