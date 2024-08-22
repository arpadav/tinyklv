pub mod dec;
pub mod enc;

pub struct Misb0601 {
    #[klv(
        key = b"\x01",
        len = 2,
        dec = ::winnow::binary::be_u16,
    )]
    pub checksum: u16,

    #[klv(
        key = b"\x02",
        len = 8,
        dec = dec::precision_timestamp,
    )]
    pub precision_timestamp: chrono::DateTime<chrono::Utc>,
}