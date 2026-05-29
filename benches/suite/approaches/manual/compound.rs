//! manual x compound record: the flat machine plus a nested coordinate sub-parser and the
//! packed sensor run. Both directions share the key constants below.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{Manual, put};
use crate::suite::Codec;
use crate::suite::records::{GpsCoord, Compound, Reading};

mod key {
    /// Key for field `id` (`u32`)
    pub const ID: u8 = 0x01;
    /// Key for the nested `GpsCoord` sub-packet
    pub const COORD: u8 = 0x02;
    /// Key for field `vx` (`i16`)
    pub const VX: u8 = 0x03;
    /// Key for field `vy` (`i16`)
    pub const VY: u8 = 0x04;
    /// Key for field `vz` (`i16`)
    pub const VZ: u8 = 0x05;
    /// Key for field `altitude` (`f64`)
    pub const ALTITUDE: u8 = 0x06;
    /// Key for field `heading` (`f32`)
    pub const HEADING: u8 = 0x07;
    /// Key for field `mode` (`u8`)
    pub const MODE: u8 = 0x08;
    /// Key for field `sensors` (packed `Vec<Reading>`)
    pub const SENSORS: u8 = 0x09;
}

mod coord_key {
    /// Key for the latitude field inside the nested coordinate sub-packet (`f64`)
    pub const LAT: u8 = 0x01;
    /// Key for the longitude field inside the nested coordinate sub-packet (`f64`)
    pub const LON: u8 = 0x02;
}

/// [`Manual`] implementation of [`Codec`] for [`Compound`]
impl Codec<Compound> for Manual {
    fn encode(rec: &Compound) -> Vec<u8> {
        // --------------------------------------------------
        // encode nested coordinate sub-packet
        // --------------------------------------------------
        let mut coord = Vec::new();
        put(&mut coord, coord_key::LAT, &rec.coord.lat.to_be_bytes());
        put(&mut coord, coord_key::LON, &rec.coord.lon.to_be_bytes());
        // --------------------------------------------------
        // push all fields as key/length/value triples
        // --------------------------------------------------
        let mut out = Vec::new();
        put(&mut out, key::ID, &rec.id.to_be_bytes());
        put(&mut out, key::COORD, &coord);
        put(&mut out, key::VX, &rec.vx.to_be_bytes());
        put(&mut out, key::VY, &rec.vy.to_be_bytes());
        put(&mut out, key::VZ, &rec.vz.to_be_bytes());
        put(&mut out, key::ALTITUDE, &rec.altitude.to_be_bytes());
        put(&mut out, key::HEADING, &rec.heading.to_be_bytes());
        put(&mut out, key::MODE, &[rec.mode]);
        put(&mut out, key::SENSORS, &Reading::pack(&rec.sensors));
        out
    }

    fn decode(body: &[u8]) -> Option<Compound> {
        // --------------------------------------------------
        // initialise field accumulators
        // --------------------------------------------------
        let mut id = None;
        let mut coord = None;
        let mut vx = None;
        let mut vy = None;
        let mut vz = None;
        let mut altitude = None;
        let mut heading = None;
        let mut mode = None;
        let mut sensors = None;
        // --------------------------------------------------
        // walk key/length/value triples and dispatch by tag
        // --------------------------------------------------
        let mut j = 0;
        while j + 2 <= body.len() {
            let tag = body[j];
            let len = usize::from(body[j + 1]);
            j += 2;
            let val = body.get(j..j + len)?;
            j += len;
            match tag {
                key::ID => id = Some(u32::from_be_bytes(val.try_into().ok()?)),
                key::COORD => coord = Some(decode_coord(val)?),
                key::VX => vx = Some(i16::from_be_bytes(val.try_into().ok()?)),
                key::VY => vy = Some(i16::from_be_bytes(val.try_into().ok()?)),
                key::VZ => vz = Some(i16::from_be_bytes(val.try_into().ok()?)),
                key::ALTITUDE => altitude = Some(f64::from_be_bytes(val.try_into().ok()?)),
                key::HEADING => heading = Some(f32::from_be_bytes(val.try_into().ok()?)),
                key::MODE => mode = Some(*val.first()?),
                key::SENSORS => sensors = Some(Reading::unpack(val)?),
                _ => {}
            }
        }
        // --------------------------------------------------
        // construct record, propagating None on any missing field
        // --------------------------------------------------
        Some(Compound {
            id: id?,
            coord: coord?,
            vx: vx?,
            vy: vy?,
            vz: vz?,
            altitude: altitude?,
            heading: heading?,
            mode: mode?,
            sensors: sensors?,
        })
    }
}

/// Decodes the nested coordinate sub-packet from its raw value body
///
/// Walks the KLV triples in `body` using the [`coord_key`] constants and
/// populates `lat` and `lon`. Unknown keys are skipped silently
///
/// # Arguments
///
/// * `body` - the raw value bytes for the coordinate sub-packet field
///
/// # Returns
///
/// `Some(GpsCoord)` when both `lat` and `lon` were found, or `None` if either
/// field is absent or a slice conversion fails
fn decode_coord(body: &[u8]) -> Option<GpsCoord> {
    // --------------------------------------------------
    // initialise coordinate field accumulators
    // --------------------------------------------------
    let mut lat = None;
    let mut lon = None;
    // --------------------------------------------------
    // walk sub-packet triples and dispatch by tag
    // --------------------------------------------------
    let mut j = 0;
    while j + 2 <= body.len() {
        let tag = body[j];
        let len = usize::from(body[j + 1]);
        j += 2;
        let val = body.get(j..j + len)?;
        j += len;
        match tag {
            coord_key::LAT => lat = Some(f64::from_be_bytes(val.try_into().ok()?)),
            coord_key::LON => lon = Some(f64::from_be_bytes(val.try_into().ok()?)),
            _ => {}
        }
    }
    // --------------------------------------------------
    // construct coordinate, propagating None on any missing field
    // --------------------------------------------------
    Some(GpsCoord {
        lat: lat?,
        lon: lon?,
    })
}
