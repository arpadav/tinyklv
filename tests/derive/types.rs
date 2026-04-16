//! Shared domain types for advanced derive tests
//!
//! Realistic types that implement Decode + EncodeValue manually,
//! used as field types in `#[derive(Klv)]` test structs throughout
//! the `advanced_*` test modules
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;

// --------------------------------------------------
// enums
// --------------------------------------------------

/// 2-byte big-endian color discriminant
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Red,
    Green,
    Blue,
    Alpha,
    Unknown(u16),
}

impl tinyklv::Decode<&[u8]> for Color {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = tinyklv::dec::binary::be_u16(input)?;
        Ok(match v {
            0x0001 => Color::Red,
            0x0002 => Color::Green,
            0x0003 => Color::Blue,
            0x0004 => Color::Alpha,
            other => Color::Unknown(other),
        })
    }
}

impl EncodeValue<Vec<u8>> for Color {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            Color::Red => 0x0001_u16,
            Color::Green => 0x0002,
            Color::Blue => 0x0003,
            Color::Alpha => 0x0004,
            Color::Unknown(v) => *v,
        };
        tinyklv::enc::binary::be_u16(v)
    }
}

pub fn decode_color(input: &mut &[u8]) -> tinyklv::Result<Color> {
    Color::decode(input)
}

pub fn encode_color(v: &Color) -> Vec<u8> {
    v.encode_value()
}

/// 1-byte priority level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl tinyklv::Decode<&[u8]> for Priority {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = tinyklv::dec::binary::be_u8(input)?;
        match v {
            0 => Ok(Priority::Low),
            1 => Ok(Priority::Medium),
            2 => Ok(Priority::High),
            3 => Ok(Priority::Critical),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

impl EncodeValue<Vec<u8>> for Priority {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            Priority::Low => 0_u8,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        };
        tinyklv::enc::binary::u8(v)
    }
}

pub fn decode_priority(input: &mut &[u8]) -> tinyklv::Result<Priority> {
    Priority::decode(input)
}

pub fn encode_priority(v: &Priority) -> Vec<u8> {
    v.encode_value()
}

/// 2-byte material class
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Material {
    Steel,
    Aluminum,
    Composite,
    Ceramic,
}

impl tinyklv::Decode<&[u8]> for Material {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = tinyklv::dec::binary::be_u16(input)?;
        match v {
            0x0001 => Ok(Material::Steel),
            0x0002 => Ok(Material::Aluminum),
            0x0003 => Ok(Material::Composite),
            0x0004 => Ok(Material::Ceramic),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

impl EncodeValue<Vec<u8>> for Material {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            Material::Steel => 0x0001_u16,
            Material::Aluminum => 0x0002,
            Material::Composite => 0x0003,
            Material::Ceramic => 0x0004,
        };
        tinyklv::enc::binary::be_u16(v)
    }
}

pub fn decode_material(input: &mut &[u8]) -> tinyklv::Result<Material> {
    Material::decode(input)
}

pub fn encode_material(v: &Material) -> Vec<u8> {
    v.encode_value()
}

/// 1-byte operational mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpMode {
    Standby,
    Active,
    Degraded,
    Emergency,
}

impl tinyklv::Decode<&[u8]> for OpMode {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = tinyklv::dec::binary::be_u8(input)?;
        match v {
            0 => Ok(OpMode::Standby),
            1 => Ok(OpMode::Active),
            2 => Ok(OpMode::Degraded),
            3 => Ok(OpMode::Emergency),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

impl EncodeValue<Vec<u8>> for OpMode {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            OpMode::Standby => 0_u8,
            OpMode::Active => 1,
            OpMode::Degraded => 2,
            OpMode::Emergency => 3,
        };
        tinyklv::enc::binary::u8(v)
    }
}

pub fn decode_opmode(input: &mut &[u8]) -> tinyklv::Result<OpMode> {
    OpMode::decode(input)
}

pub fn encode_opmode(v: &OpMode) -> Vec<u8> {
    v.encode_value()
}

/// Sensor kind discriminant (1 byte)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorKind {
    Temperature,
    Pressure,
    Humidity,
    Vibration,
}

impl tinyklv::Decode<&[u8]> for SensorKind {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = tinyklv::dec::binary::be_u8(input)?;
        match v {
            0 => Ok(SensorKind::Temperature),
            1 => Ok(SensorKind::Pressure),
            2 => Ok(SensorKind::Humidity),
            3 => Ok(SensorKind::Vibration),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

impl EncodeValue<Vec<u8>> for SensorKind {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            SensorKind::Temperature => 0_u8,
            SensorKind::Pressure => 1,
            SensorKind::Humidity => 2,
            SensorKind::Vibration => 3,
        };
        tinyklv::enc::binary::u8(v)
    }
}

// --------------------------------------------------
// structs
// --------------------------------------------------

/// GPS-like coordinate: lat + lon as f64 (16 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

impl tinyklv::Decode<&[u8]> for Coordinate {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let lat = tinyklv::dec::binary::be_f64(input)?;
        let lon = tinyklv::dec::binary::be_f64(input)?;
        Ok(Coordinate { lat, lon })
    }
}

impl EncodeValue<Vec<u8>> for Coordinate {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = tinyklv::enc::binary::be_f64(self.lat);
        v.extend(tinyklv::enc::binary::be_f64(self.lon));
        v
    }
}

pub fn decode_coordinate(input: &mut &[u8]) -> tinyklv::Result<Coordinate> {
    Coordinate::decode(input)
}

pub fn encode_coordinate(v: &Coordinate) -> Vec<u8> {
    v.encode_value()
}

/// Velocity vector: dx, dy, dz as i16 (6 bytes)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity {
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
}

impl tinyklv::Decode<&[u8]> for Velocity {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let dx = tinyklv::dec::binary::be_i16(input)?;
        let dy = tinyklv::dec::binary::be_i16(input)?;
        let dz = tinyklv::dec::binary::be_i16(input)?;
        Ok(Velocity { dx, dy, dz })
    }
}

impl EncodeValue<Vec<u8>> for Velocity {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = tinyklv::enc::binary::be_i16(self.dx);
        v.extend(tinyklv::enc::binary::be_i16(self.dy));
        v.extend(tinyklv::enc::binary::be_i16(self.dz));
        v
    }
}

pub fn decode_velocity(input: &mut &[u8]) -> tinyklv::Result<Velocity> {
    Velocity::decode(input)
}

pub fn encode_velocity(v: &Velocity) -> Vec<u8> {
    v.encode_value()
}

/// Attitude angles: roll, pitch, yaw as f32 (12 bytes)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl tinyklv::Decode<&[u8]> for Attitude {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let roll = tinyklv::dec::binary::be_f32(input)?;
        let pitch = tinyklv::dec::binary::be_f32(input)?;
        let yaw = tinyklv::dec::binary::be_f32(input)?;
        Ok(Attitude { roll, pitch, yaw })
    }
}

impl EncodeValue<Vec<u8>> for Attitude {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = tinyklv::enc::binary::be_f32(self.roll);
        v.extend(tinyklv::enc::binary::be_f32(self.pitch));
        v.extend(tinyklv::enc::binary::be_f32(self.yaw));
        v
    }
}

pub fn decode_attitude(input: &mut &[u8]) -> tinyklv::Result<Attitude> {
    Attitude::decode(input)
}

pub fn encode_attitude(v: &Attitude) -> Vec<u8> {
    v.encode_value()
}

/// Timestamp: seconds (u32) + sub-second nanos (u16) = 6 bytes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp {
    pub seconds: u32,
    pub nanos: u16,
}

impl tinyklv::Decode<&[u8]> for Timestamp {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let seconds = tinyklv::dec::binary::be_u32(input)?;
        let nanos = tinyklv::dec::binary::be_u16(input)?;
        Ok(Timestamp { seconds, nanos })
    }
}

impl EncodeValue<Vec<u8>> for Timestamp {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = tinyklv::enc::binary::be_u32(self.seconds);
        v.extend(tinyklv::enc::binary::be_u16(self.nanos));
        v
    }
}

pub fn decode_timestamp(input: &mut &[u8]) -> tinyklv::Result<Timestamp> {
    Timestamp::decode(input)
}

pub fn encode_timestamp(v: &Timestamp) -> Vec<u8> {
    v.encode_value()
}

/// Bounding box: min + max Coordinate (32 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    pub min: Coordinate,
    pub max: Coordinate,
}

impl tinyklv::Decode<&[u8]> for BoundingBox {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let min = Coordinate::decode(input)?;
        let max = Coordinate::decode(input)?;
        Ok(BoundingBox { min, max })
    }
}

impl EncodeValue<Vec<u8>> for BoundingBox {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = self.min.encode_value();
        v.extend(self.max.encode_value());
        v
    }
}

pub fn decode_bounding_box(input: &mut &[u8]) -> tinyklv::Result<BoundingBox> {
    BoundingBox::decode(input)
}

pub fn encode_bounding_box(v: &BoundingBox) -> Vec<u8> {
    v.encode_value()
}

/// Status flags decoded from a u16 bitfield (2 bytes)
///
/// Bit layout: `[active:1][armed:1][locked:1][mode:5][unused:8]`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusFlags {
    pub active: bool,
    pub armed: bool,
    pub locked: bool,
    pub mode: u8,
}

impl tinyklv::Decode<&[u8]> for StatusFlags {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let raw = tinyklv::dec::binary::be_u16(input)?;
        Ok(StatusFlags {
            active: (raw >> 15) & 1 == 1,
            armed: (raw >> 14) & 1 == 1,
            locked: (raw >> 13) & 1 == 1,
            mode: ((raw >> 8) & 0x1F) as u8,
        })
    }
}

impl EncodeValue<Vec<u8>> for StatusFlags {
    fn encode_value(&self) -> Vec<u8> {
        let mut raw: u16 = 0;
        if self.active {
            raw |= 1 << 15;
        }
        if self.armed {
            raw |= 1 << 14;
        }
        if self.locked {
            raw |= 1 << 13;
        }
        raw |= ((self.mode as u16) & 0x1F) << 8;
        tinyklv::enc::binary::be_u16(raw)
    }
}

pub fn decode_status_flags(input: &mut &[u8]) -> tinyklv::Result<StatusFlags> {
    StatusFlags::decode(input)
}

pub fn encode_status_flags(v: &StatusFlags) -> Vec<u8> {
    v.encode_value()
}

/// Sensor reading: kind (1 byte) + value f32 (4 bytes) = 5 bytes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorReading {
    pub kind: SensorKind,
    pub value: f32,
}

impl tinyklv::Decode<&[u8]> for SensorReading {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let kind = SensorKind::decode(input)?;
        let value = tinyklv::dec::binary::be_f32(input)?;
        Ok(SensorReading { kind, value })
    }
}

impl EncodeValue<Vec<u8>> for SensorReading {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = self.kind.encode_value();
        v.extend(tinyklv::enc::binary::be_f32(self.value));
        v
    }
}

pub fn decode_sensor_reading(input: &mut &[u8]) -> tinyklv::Result<SensorReading> {
    SensorReading::decode(input)
}

pub fn encode_sensor_reading(v: &SensorReading) -> Vec<u8> {
    v.encode_value()
}

// --------------------------------------------------
// variable-length decoders
// --------------------------------------------------

/// Decode a variable number of sensor readings based on byte length
///
/// Each reading is 5 bytes (1 kind + 4 value). Reads `len / 5` readings
pub fn decode_sensor_readings(
    len: usize,
) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<SensorReading>> {
    move |input: &mut &[u8]| {
        let count = len / 5;
        let mut readings = Vec::with_capacity(count);
        for _ in 0..count {
            readings.push(SensorReading::decode(input)?);
        }
        Ok(readings)
    }
}

pub fn encode_sensor_readings(v: &Vec<SensorReading>) -> Vec<u8> {
    let mut out = Vec::new();
    for r in v {
        out.extend(r.encode_value());
    }
    out
}

// --------------------------------------------------
// primitive encoder wrappers
// --------------------------------------------------

pub fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}

pub fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}

pub fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

pub fn enc_u64(v: &u64) -> Vec<u8> {
    tinyklv::enc::binary::be_u64(*v)
}

pub fn enc_i16(v: &i16) -> Vec<u8> {
    tinyklv::enc::binary::be_i16(*v)
}

pub fn enc_i32(v: &i32) -> Vec<u8> {
    tinyklv::enc::binary::be_i32(*v)
}

pub fn enc_f32(v: &f32) -> Vec<u8> {
    tinyklv::enc::binary::be_f32(*v)
}

pub fn enc_f64(v: &f64) -> Vec<u8> {
    tinyklv::enc::binary::be_f64(*v)
}
