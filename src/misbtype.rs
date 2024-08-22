

struct Misb0601 {

    #[klv(key = b"\x04", dec = crate::misb::to_string)]
    platform_tail_numer: String,

    #[klv(key = b"\x05", dec = winnow::binary::be_u16)]
    platform_heading_angle: u16,

    #[klv(key = b"\x06", dec = winnow::binary::be_i16)]
    platform_pitch_angle: i16,

    #[klv(key = b"\x07", dec = winnow::binary::be_i16)]
    platform_roll_angle: i16,

    #[klv(key = b"\x08", dec = winnow::binary::be_u8)]
    platform_true_airspeed: u8,

    #[klv(key = b"\x09", dec = winnow::binary::be_u8)]
    platform_indicated_airspeed: u8,

    #[klv(key = b"\x0a", dec = crate::misb::to_string)]
    platform_designation: String,

    #[klv(key = b"\x0b", dec = crate::misb::to_string)]
    image_source_sensor: String,

    #[klv(key = b"\x0c", dec = crate::misb::to_string)]
    image_coordinate_system: String,

    #[klv(key = b"\x0d", dec = winnow::binary::be_i32)]
    sensor_latitude: i32,

    #[klv(key = b"\x0e", dec = winnow::binary::be_i32)]
    sensor_longitude: i32,

    #[klv(key = b"\x0f", dec = winnow::binary::be_u16)]
    /// Altitude of sensor above MSL (mean sea level).
    sensor_true_altitude: u16,
}