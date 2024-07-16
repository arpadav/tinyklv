#[cfg(test)]
pub mod test {

    use tinyklv::Klv;

    #[derive(Klv)]
    #[klv(key_dec = Self::serialize)]
    #[klv(key_enc = Self::deserialize)]
    #[klv(len_dec = Self::serialize)]
    #[klv(len_enc = Self::deserialize)]
    #[klv(default_enc = (u8, Self::serialize_u8))]
    pub struct MyStruct {
        #[klv(key = b"\x01")]
        #[klv(len = 2)]
        #[klv(dec = Self::serialize)]
        #[klv(enc = Self::deserialize)]
        pub a: u32,
        
        #[klv(key = b"\x02")]
        #[klv(len = 1)]
        #[klv(dec = Self::serialize)]
        #[klv(enc = Self::deserialize)]
        pub b: u32,
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}