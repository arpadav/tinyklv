//! serde_klv x nested record: the whole reason this adapter exists.
//!
//! serde_klv cannot derive a nested KLV record - its serializer flattens every struct's
//! keys into one namespace and collides, and `Vec<struct>` repeats keys. So nesting it at
//! all means hand-writing `Serialize`/`Deserialize`: flatten the coordinate into unique
//! keys and byte-pack the sensor `Vec` through a private wire struct. All of this is the
//! cost serde_klv pays for nesting - versus the single derive line tinyklv uses.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::SerdeKlv;
use crate::suite::records::{GpsCoord, Platform, Reading};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Flat wire form of [`Platform`] that serde_klv can derive
///
/// serde_klv cannot handle nested structs or `Vec<struct>` - its serializer
/// maps every key into one flat namespace, so collisions are inevitable.
/// This struct flattens the `GpsCoord` sub-packet into `lat`/`lon` scalars
/// and byte-packs the `Vec<Reading>` sensor run through a `serde_bytes` blob.
/// Both directions hand-convert between `Platform` and this wire form
#[derive(Serialize, Deserialize)]
#[serde(rename = "PLATFORM00000000")]
struct PlatformWire {
    #[serde(rename = "1")]
    id: u32,
    #[serde(rename = "2")]
    lat: f64,
    #[serde(rename = "3")]
    lon: f64,
    #[serde(rename = "4")]
    vx: i16,
    #[serde(rename = "5")]
    vy: i16,
    #[serde(rename = "6")]
    vz: i16,
    #[serde(rename = "7")]
    altitude: f64,
    #[serde(rename = "8")]
    heading: f32,
    #[serde(rename = "9")]
    mode: u8,
    #[serde(rename = "10", with = "serde_bytes")]
    sensors: Vec<u8>,
}

/// [`Platform`] implementation of [`Serialize`]
impl Serialize for Platform {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // --------------------------------------------------
        // convert Platform to flat wire form and delegate to serde_klv
        // --------------------------------------------------
        PlatformWire {
            id: self.id,
            lat: self.coord.lat,
            lon: self.coord.lon,
            vx: self.vx,
            vy: self.vy,
            vz: self.vz,
            altitude: self.altitude,
            heading: self.heading,
            mode: self.mode,
            sensors: Reading::pack(&self.sensors),
        }
        .serialize(serializer)
    }
}

/// [`Platform`] implementation of [`Deserialize`]
impl<'de> Deserialize<'de> for Platform {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // --------------------------------------------------
        // deserialize flat wire form
        // --------------------------------------------------
        let w = PlatformWire::deserialize(deserializer)?;
        // --------------------------------------------------
        // unpack sensor byte run into typed readings
        // --------------------------------------------------
        let sensors =
            Reading::unpack(&w.sensors).ok_or_else(|| D::Error::custom("malformed sensor run"))?;
        // --------------------------------------------------
        // reconstruct nested Platform from flat wire fields
        // --------------------------------------------------
        Ok(Platform {
            id: w.id,
            coord: GpsCoord {
                lat: w.lat,
                lon: w.lon,
            },
            vx: w.vx,
            vy: w.vy,
            vz: w.vz,
            altitude: w.altitude,
            heading: w.heading,
            mode: w.mode,
            sensors,
        })
    }
}

/// [`SerdeKlv`] implementation of [`Codec`] for [`Platform`]
impl Codec<Platform> for SerdeKlv {
    fn encode(rec: &Platform) -> Vec<u8> {
        ::serde_klv::to_bytes(rec).unwrap()
    }

    fn decode(body: &[u8]) -> Option<Platform> {
        ::serde_klv::from_bytes::<Platform>(body).ok()
    }
}
