#[cfg(test)]
pub mod test {

    use tinyklv::Klv;

    #[derive(Klv)]
    #[klv(key_ser_with = Self::serialize)]
    #[klv(key_deser_with = Self::deserialize)]
    #[klv(len_ser_with = Self::serialize)]
    #[klv(len_deser_with = Self::deserialize)]
    pub struct MyStruct {
        #[klv(key = b"\x01")]
        #[klv(len = 2)]
        #[klv(ser_with = Self::serialize)]
        #[klv(deser_with = Self::deserialize)]
        pub a: u32,
        pub b: u32,
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}