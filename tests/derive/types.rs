//! Shared domain types for advanced derive tests
//!
//! Realistic types that implement Decode + tinyklv::EncodeValue manually,
//! used as field types in `#[derive(Klv)]` test structs throughout
//! the `advanced_*` test modules
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
/// 2-byte big-endian color discriminant
pub enum Color {
    Red,
    Green,
    Blue,
    Alpha,
    Unknown(u16),
}
impl tinyklv::DecodeValue<&[u8]> for Color {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = decb::be_u16(input)?;
        Ok(match v {
            0x0001 => Color::Red,
            0x0002 => Color::Green,
            0x0003 => Color::Blue,
            0x0004 => Color::Alpha,
            other => Color::Unknown(other),
        })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Color {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            Color::Red => 0x0001_u16,
            Color::Green => 0x0002,
            Color::Blue => 0x0003,
            Color::Alpha => 0x0004,
            Color::Unknown(v) => *v,
        };
        encb::be_u16(v)
    }
}
/// User-side escape hatch for the `&` sigil: a `Copy` enum is cheapest to
/// pass by value, so [`EncodeAs::Borrowed`] resolves to `Self`.
impl tinyklv::EncodeAs for Color {
    type Borrowed<'a> = Color;
    #[inline(always)]
    fn encode_as(&self) -> Color {
        *self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// 1-byte priority level
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}
impl tinyklv::DecodeValue<&[u8]> for Priority {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = decb::u8(input)?;
        match v {
            0 => Ok(Priority::Low),
            1 => Ok(Priority::Medium),
            2 => Ok(Priority::High),
            3 => Ok(Priority::Critical),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Priority {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            Priority::Low => 0_u8,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        };
        encb::u8(v)
    }
}
/// User-side escape hatch for the `&` sigil: a `Copy` enum is cheapest to
/// pass by value, so [`EncodeAs::Borrowed`] resolves to `Self`.
impl tinyklv::EncodeAs for Priority {
    type Borrowed<'a> = Priority;
    #[inline(always)]
    fn encode_as(&self) -> Priority {
        *self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// 2-byte material class
pub enum Material {
    Steel,
    Aluminum,
    Composite,
    Ceramic,
}
impl tinyklv::DecodeValue<&[u8]> for Material {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = decb::be_u16(input)?;
        match v {
            0x0001 => Ok(Material::Steel),
            0x0002 => Ok(Material::Aluminum),
            0x0003 => Ok(Material::Composite),
            0x0004 => Ok(Material::Ceramic),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Material {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            Material::Steel => 0x0001_u16,
            Material::Aluminum => 0x0002,
            Material::Composite => 0x0003,
            Material::Ceramic => 0x0004,
        };
        encb::be_u16(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// 1-byte operational mode
pub enum OpMode {
    Standby,
    Active,
    Degraded,
    Emergency,
}
impl tinyklv::DecodeValue<&[u8]> for OpMode {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = decb::u8(input)?;
        match v {
            0 => Ok(OpMode::Standby),
            1 => Ok(OpMode::Active),
            2 => Ok(OpMode::Degraded),
            3 => Ok(OpMode::Emergency),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for OpMode {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            OpMode::Standby => 0_u8,
            OpMode::Active => 1,
            OpMode::Degraded => 2,
            OpMode::Emergency => 3,
        };
        encb::u8(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Sensor kind discriminant (1 byte)
pub enum SensorKind {
    Temperature,
    Pressure,
    Humidity,
    Vibration,
}
impl tinyklv::DecodeValue<&[u8]> for SensorKind {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = decb::u8(input)?;
        match v {
            0 => Ok(SensorKind::Temperature),
            1 => Ok(SensorKind::Pressure),
            2 => Ok(SensorKind::Humidity),
            3 => Ok(SensorKind::Vibration),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for SensorKind {
    fn encode_value(&self) -> Vec<u8> {
        let v = match self {
            SensorKind::Temperature => 0_u8,
            SensorKind::Pressure => 1,
            SensorKind::Humidity => 2,
            SensorKind::Vibration => 3,
        };
        encb::u8(v)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// GPS-like coordinate: lat + lon as f64 (16 bytes)
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}
impl tinyklv::DecodeValue<&[u8]> for Coordinate {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let lat = decb::be_f64(input)?;
        let lon = decb::be_f64(input)?;
        Ok(Coordinate { lat, lon })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Coordinate {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = encb::be_f64(self.lat);
        v.extend(encb::be_f64(self.lon));
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Velocity vector: dx, dy, dz as i16 (6 bytes)
pub struct Velocity {
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
}
impl tinyklv::DecodeValue<&[u8]> for Velocity {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let dx = decb::be_i16(input)?;
        let dy = decb::be_i16(input)?;
        let dz = decb::be_i16(input)?;
        Ok(Velocity { dx, dy, dz })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Velocity {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = encb::be_i16(self.dx);
        v.extend(encb::be_i16(self.dy));
        v.extend(encb::be_i16(self.dz));
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Attitude angles: roll, pitch, yaw as f32 (12 bytes)
pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}
impl tinyklv::DecodeValue<&[u8]> for Attitude {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let roll = decb::be_f32(input)?;
        let pitch = decb::be_f32(input)?;
        let yaw = decb::be_f32(input)?;
        Ok(Attitude { roll, pitch, yaw })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Attitude {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = encb::be_f32(self.roll);
        v.extend(encb::be_f32(self.pitch));
        v.extend(encb::be_f32(self.yaw));
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Timestamp: seconds (u32) + sub-second nanos (u16) = 6 bytes
pub struct Timestamp {
    pub seconds: u32,
    pub nanos: u16,
}
impl tinyklv::DecodeValue<&[u8]> for Timestamp {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let seconds = decb::be_u32(input)?;
        let nanos = decb::be_u16(input)?;
        Ok(Timestamp { seconds, nanos })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Timestamp {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = encb::be_u32(self.seconds);
        v.extend(encb::be_u16(self.nanos));
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Status flags decoded from a u16 bitfield (2 bytes)
///
/// Bit layout: `[active:1][armed:1][locked:1][mode:5][unused:8]`
pub struct StatusFlags {
    pub active: bool,
    pub armed: bool,
    pub locked: bool,
    pub mode: u8,
}
impl tinyklv::DecodeValue<&[u8]> for StatusFlags {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let raw = decb::be_u16(input)?;
        Ok(StatusFlags {
            active: (raw >> 15) & 1 == 1,
            armed: (raw >> 14) & 1 == 1,
            locked: (raw >> 13) & 1 == 1,
            mode: ((raw >> 8) & 0x1F) as u8,
        })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for StatusFlags {
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
        encb::be_u16(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Sensor reading: kind (1 byte) + value f32 (4 bytes) = 5 bytes
pub struct SensorReading {
    pub kind: SensorKind,
    pub value: f32,
}
impl tinyklv::DecodeValue<&[u8]> for SensorReading {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let kind = SensorKind::decode_value(input)?;
        let value = decb::be_f32(input)?;
        Ok(SensorReading { kind, value })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for SensorReading {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = self.kind.encode_value();
        v.extend(encb::be_f32(self.value));
        v
    }
}

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
            readings.push(SensorReading::decode_value(input)?);
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
