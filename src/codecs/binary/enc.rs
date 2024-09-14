
macro_rules! impl_encode {
    ($ty:ty) => { paste::paste!{
        #[inline(always)]
        pub fn [<be_ $ty>](input: $ty) -> Vec<u8> {
            input.to_be_bytes().to_vec()
        }

        #[inline(always)]
        pub fn [<le_ $ty>](input: $ty) -> Vec<u8> {
            input.to_le_bytes().to_vec()
        }

        #[inline(always)]
        pub fn [<$ty>](input: $ty) -> Vec<u8> {
            #[cfg(target_endian = "big")]
            { input.to_be_bytes().to_vec() }
            #[cfg(target_endian = "little")]
            { input.to_le_bytes().to_vec() }
        }

        #[inline(always)]
        pub fn [<be_ $ty _lengthed>](len: usize) -> impl Fn($ty) -> Vec<u8> {
            move |input: $ty| {
                let bytes = input.to_be_bytes();
                let start = bytes.len().saturating_sub(len);
                let mut result = Vec::with_capacity(len);
                result.extend_from_slice(&bytes[start..]);
                result.splice(0..0, std::iter::repeat(0).take(len - result.len()));
                result
            }
        }

        // #[inline(always)]
        // pub fn [<le_ $ty _lengthed2>](len: usize) -> impl Fn($ty) -> Vec<u8> {
        //     move |input: $ty| {
        //         let mut v = input.to_le_bytes().to_vec();
        //         v.reverse();
        //         v.resize(len, 0);
        //         v.reverse();
        //         v
        //     }
        // }

        #[inline(always)]
        pub fn [<le_ $ty _lengthed>](len: usize) -> impl Fn($ty) -> Vec<u8> {
            move |input: $ty| {
                let mut v = input.to_le_bytes().to_vec();
                v.resize(len, 0);
                v
            }
        }
    }};
}
impl_encode!(u8);
impl_encode!(u16);
impl_encode!(u32);
impl_encode!(u64);
impl_encode!(u128);
impl_encode!(i8);
impl_encode!(i16);
impl_encode!(i32);
impl_encode!(i64);
impl_encode!(i128);
impl_encode!(f32);
impl_encode!(f64);

#[test]
fn bruh() {
    // let mut input1: &[u8] = &[0x00, 0x01, 0xE0, 0xFF, 0xFF];
    // let mut input2: &[u8] = &[0x00, 0x01, 0xE0, 0xFF];
    // let mut input3: &[u8] = &[0x01, 0xE0, 0xFF];
    // assert_eq!(tinyklv::dec::binary::be_u32_lengthed(&mut input1, 5), Ok(0x01E0FFFF));
    // assert_eq!(tinyklv::dec::binary::be_u32_lengthed(&mut input2, 4), Ok(0x0001E0FF));
    // assert_eq!(tinyklv::dec::binary::be_u32_lengthed(&mut input3, 3), Ok(0x0001E0FF));
    let output1 = vec![0x00, 0x01, 0xE0, 0xFF, 0xFF];
    let output2 = vec![0x00, 0x01, 0xE0, 0xFF];
    let output3 = vec![0x01, 0xE0, 0xFF];
    assert_eq!(be_u32_lengthed(5)(0x01E0FFFF), output1);
    assert_eq!(be_u32_lengthed(4)(0x0001E0FF), output2);
    assert_eq!(be_u32_lengthed(3)(0x0001E0FF), output3);

    // let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
    // let mut input2: &[u8] = &[0x01];
    // let num1 = tinyklv::dec::binary::le_u16_lengthed(&mut input1, 5);
    // let num2 = tinyklv::dec::binary::le_u16_lengthed(&mut input2, 1);
    // assert_eq!(num1, Ok(480));
    // assert_eq!(num2, Ok(1));
    let output1 = vec![0xE0, 0x01, 0xFF, 0xFF, 0xFF];
    let output2 = vec![0x01];
    assert_eq!(le_u16_lengthed(5)(480)[..2], output1[..2]);
    assert_eq!(le_u16_lengthed(1)(1)[..1], output2[..1]);

    // let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
    // let mut input2: &[u8] = &[0x01];
    // let mut input3: &[u8] = &[0x01, 0x02, 0x03];
    // let num1 = tinyklv::dec::binary::le_u32_lengthed(&mut input1, 5);
    // let num2 = tinyklv::dec::binary::le_u32_lengthed(&mut input2, 1);
    // let num3 = tinyklv::dec::binary::le_u32_lengthed(&mut input3, 3);
    // assert_eq!(num1, Ok(4_294_902_240));
    // assert_eq!(num2, Ok(1));
    // assert_eq!(num3, Ok(197_121));
    let output1 = vec![0xE0, 0x01, 0xFF, 0xFF, 0xFF];
    let output2 = vec![0x01];
    let output3 = vec![0x01, 0x02, 0x03];
    assert_eq!(le_u32_lengthed(5)(4_294_902_240)[..4], output1[..4]);
    assert_eq!(le_u32_lengthed(1)(1)[..1], output2[..1]);
    assert_eq!(le_u32_lengthed(3)(197_121)[..3], output3[..3]);

    // let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    // let mut input2: &[u8] = &[0x01];
    // let mut input3: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    // let num1 = tinyklv::dec::binary::le_u64_lengthed(&mut input1, 16);
    // let num2 = tinyklv::dec::binary::le_u64_lengthed(&mut input2, 1);
    // let num3 = tinyklv::dec::binary::le_u64_lengthed(&mut input3, 7);
    // assert_eq!(num1, Ok(1_099_511_562_720));
    // assert_eq!(num2, Ok(1));
    // assert_eq!(num3, Ok(1_976_943_448_883_713));
    let output1 = vec![0xE0, 0x01, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let output2 = vec![0x01];
    let output3 = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    assert_eq!(le_u64_lengthed(16)(1_099_511_562_720)[..16], output1[..16]);
    assert_eq!(le_u64_lengthed(1)(1)[..1], output2[..1]);
    assert_eq!(le_u64_lengthed(7)(1_976_943_448_883_713)[..7], output3[..7]);

    // let mut input1: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
    // let mut input2: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00];
    // let mut input3: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00];
    // assert_eq!(tinyklv::dec::binary::be_u64_lengthed(&mut input1, 9), Ok(0x00_01_E0_FF_FF_00_00_00));
    // assert_eq!(tinyklv::dec::binary::be_u64_lengthed(&mut input2, 8), Ok(0x00_00_01_E0_FF_FF_00_00));
    // assert_eq!(tinyklv::dec::binary::be_u64_lengthed(&mut input3, 7), Ok(0x00_00_00_01_E0_FF_FF_00));
    let output1 = vec![0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
    let output2 = vec![0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00];
    let output3 = vec![0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00];
    assert_eq!(be_u64_lengthed(9)(0x00_01_E0_FF_FF_00_00_00)[..9], output1[..9]);
    assert_eq!(be_u64_lengthed(8)(0x00_00_01_E0_FF_FF_00_00)[..8], output2[..8]);
    assert_eq!(be_u64_lengthed(7)(0x00_00_00_01_E0_FF_FF_00)[..7], output3[..7]);
}