mod level {
    use tinyklv::Klv;

    #[derive(Klv)]
    #[klv(
        key(dec = tinyklv::dec::binary::u8),
        len(dec = tinyklv::dec::binary::u8_as_usize),
        allow_unimplemented_encode,
    )]
    pub struct Level {
        #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
        level: u8,
    }
}

fn main() {
    let _ = level::LevelPartialPacket { level: Some(200) };
}
