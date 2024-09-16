macro_rules! impl_encode {
    ($ty:ty) => {
        impl_encode!(simple $ty);
        impl_encode!(with_endianness $ty);
    };

    (simple $ty:ty) => { paste::paste! {
        #[inline(always)]
        pub fn [<$ty>](input: $ty) -> Vec<u8> {
            #[cfg(target_endian = "big")]
            { input.to_be_bytes().to_vec() }
            #[cfg(target_endian = "little")]
            { input.to_le_bytes().to_vec() }
        }

        #[inline(always)]
        pub fn [<$ty _from_usize>](input: usize) -> Vec<u8> {
            #[cfg(target_endian = "big")]
            { (input as $ty).to_be_bytes().to_vec() }
            #[cfg(target_endian = "little")]
            { (input as $ty).to_le_bytes().to_vec() }
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

        #[inline(always)]
        pub fn [<le_ $ty _lengthed>](len: usize) -> impl Fn($ty) -> Vec<u8> {
            move |input: $ty| {
                let mut v = input.to_le_bytes().to_vec();
                v.resize(len, 0);
                v
            }
        }
    }};

    (with_endianness $ty:ty) => { paste::paste! {
        #[inline(always)]
        pub fn [<be_ $ty>](input: $ty) -> Vec<u8> {
            input.to_be_bytes().to_vec()
        }

        #[inline(always)]
        pub fn [<be_ $ty _from_usize>](input: usize) -> Vec<u8> {
            (input as $ty).to_be_bytes().to_vec()
        }

        #[inline(always)]
        pub fn [<le_ $ty>](input: $ty) -> Vec<u8> {
            input.to_le_bytes().to_vec()
        }

        #[inline(always)]
        pub fn [<le_ $ty _from_usize>](input: usize) -> Vec<u8> {
            (input as $ty).to_le_bytes().to_vec()
        }
    }};
}

impl_encode!(simple u8);
impl_encode!(u16);
impl_encode!(u32);
impl_encode!(u64);
impl_encode!(u128);
impl_encode!(simple i8);
impl_encode!(i16);
impl_encode!(i32);
impl_encode!(i64);
impl_encode!(i128);
impl_encode!(f32);
impl_encode!(f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_some() {
        let output1 = vec![0x00, 0x01, 0xE0, 0xFF, 0xFF];
        let output2 = vec![0x00, 0x01, 0xE0, 0xFF];
        let output3 = vec![0x01, 0xE0, 0xFF];
        assert_eq!(be_u32_lengthed(5)(0x01E0FFFF), output1);
        assert_eq!(be_u32_lengthed(4)(0x0001E0FF), output2);
        assert_eq!(be_u32_lengthed(3)(0x0001E0FF), output3);
        
        let output1 = vec![0xE0, 0x01, 0xFF, 0xFF, 0xFF];
        let output2 = vec![0x01];
        assert_eq!(le_u16_lengthed(5)(480)[..2], output1[..2]);
        assert_eq!(le_u16_lengthed(1)(1)[..1], output2[..1]);
        
        let output1 = vec![0xE0, 0x01, 0xFF, 0xFF, 0xFF];
        let output2 = vec![0x01];
        let output3 = vec![0x01, 0x02, 0x03];
        assert_eq!(le_u32_lengthed(5)(4_294_902_240)[..4], output1[..4]);
        assert_eq!(le_u32_lengthed(1)(1)[..1], output2[..1]);
        assert_eq!(le_u32_lengthed(3)(197_121)[..3], output3[..3]);
        
        let output1 = vec![0xE0, 0x01, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let output2 = vec![0x01];
        let output3 = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(le_u64_lengthed(16)(1_099_511_562_720)[..16], output1[..16]);
        assert_eq!(le_u64_lengthed(1)(1)[..1], output2[..1]);
        assert_eq!(le_u64_lengthed(7)(1_976_943_448_883_713)[..7], output3[..7]);
        
        let output1 = vec![0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
        let output2 = vec![0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00];
        let output3 = vec![0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00];
        assert_eq!(be_u64_lengthed(9)(0x00_01_E0_FF_FF_00_00_00)[..9], output1[..9]);
        assert_eq!(be_u64_lengthed(8)(0x00_00_01_E0_FF_FF_00_00)[..8], output2[..8]);
        assert_eq!(be_u64_lengthed(7)(0x00_00_00_01_E0_FF_FF_00)[..7], output3[..7]);
    }

    use rand;
    use rand_distr::Distribution;
    use crate::codecs::binary::dec;

    #[test]
    fn randoms() {
        const TRIALS: usize = 10;

        const NBYTES_8: usize = 1;
        const NBYTES_16: usize = 2;
        const NBYTES_32: usize = 4;
        const NBYTES_64: usize = 8;
        const NBYTES_128: usize = 16;

        let random_usize: Box<dyn Fn(usize) -> Vec<usize>> = Box::new(|max| {
            let pareto = rand_distr::Pareto::new(2.0, 1.0).unwrap();
            let mut rng = rand::thread_rng();
            (0..TRIALS)
                .map(|_| (pareto.sample(&mut rng) as usize).clamp(1, max))
                .collect()
        });
        let random_f32: Box<dyn Fn() -> Vec<f32>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<f32>()).collect());
        let random_f64: Box<dyn Fn() -> Vec<f64>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<f64>()).collect());
        let random_u8: Box<dyn Fn() -> Vec<u8>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<u8>()).collect());
        let random_u16: Box<dyn Fn() -> Vec<u16>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<u16>()).collect());
        let random_u32: Box<dyn Fn() -> Vec<u32>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<u32>()).collect());
        let random_u64: Box<dyn Fn() -> Vec<u64>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<u64>()).collect());
        let random_u128: Box<dyn Fn() -> Vec<u128>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<u128>()).collect());
        let random_i8: Box<dyn Fn() -> Vec<i8>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<i8>()).collect());
        let random_i16: Box<dyn Fn() -> Vec<i16>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<i16>()).collect());
        let random_i32: Box<dyn Fn() -> Vec<i32>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<i32>()).collect());
        let random_i64: Box<dyn Fn() -> Vec<i64>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<i64>()).collect());
        let random_i128: Box<dyn Fn() -> Vec<i128>> = Box::new(|| (0..TRIALS).map(|_| rand::random::<i128>()).collect());

        random_f32().iter().for_each(|x| assert_eq!(x, &dec::f32(&mut f32(*x).as_slice()).unwrap()));
        random_f32().iter().for_each(|x| assert_eq!(x, &dec::be_f32(&mut be_f32(*x).as_slice()).unwrap()));
        random_f32().iter().for_each(|x| assert_eq!(x, &dec::le_f32(&mut le_f32(*x).as_slice()).unwrap()));
        random_f32().iter().for_each(|x| assert_eq!(x, &dec::be_f32_lengthed(NBYTES_32)(&mut be_f32_lengthed(NBYTES_32)(*x).as_slice()).unwrap()));
        random_f32().iter().for_each(|x| assert_eq!(x, &dec::le_f32_lengthed(NBYTES_32)(&mut le_f32_lengthed(NBYTES_32)(*x).as_slice()).unwrap()));
        random_f32().iter().for_each(|x| random_usize(NBYTES_32 * NBYTES_32).iter()
        .for_each(|y| { match *y < NBYTES_32 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_f32_lengthed(*y)(*x),
                    be_f32_lengthed(*y)(dec::be_f32_lengthed(*y)(&mut be_f32_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_f32_lengthed(*y)(*x),
                    le_f32_lengthed(*y)(dec::le_f32_lengthed(*y)(&mut le_f32_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_f32_lengthed(*y)(&mut be_f32_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_f32_lengthed(*y)(&mut le_f32_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_f64().iter().for_each(|x| assert_eq!(x, &dec::f64(&mut f64(*x).as_slice()).unwrap()));
        random_f64().iter().for_each(|x| assert_eq!(x, &dec::be_f64(&mut be_f64(*x).as_slice()).unwrap()));
        random_f64().iter().for_each(|x| assert_eq!(x, &dec::le_f64(&mut le_f64(*x).as_slice()).unwrap()));
        random_f64().iter().for_each(|x| assert_eq!(x, &dec::be_f64_lengthed(NBYTES_64)(&mut be_f64_lengthed(NBYTES_64)(*x).as_slice()).unwrap()));
        random_f64().iter().for_each(|x| assert_eq!(x, &dec::le_f64_lengthed(NBYTES_64)(&mut le_f64_lengthed(NBYTES_64)(*x).as_slice()).unwrap()));
        random_f64().iter().for_each(|x| random_usize(NBYTES_64 * NBYTES_64).iter()
        .for_each(|y| { match *y < NBYTES_64 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_f64_lengthed(*y)(*x),
                    be_f64_lengthed(*y)(dec::be_f64_lengthed(*y)(&mut be_f64_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_f64_lengthed(*y)(*x),
                    le_f64_lengthed(*y)(dec::le_f64_lengthed(*y)(&mut le_f64_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_f64_lengthed(*y)(&mut be_f64_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_f64_lengthed(*y)(&mut le_f64_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_u8().iter().for_each(|x| assert_eq!(x, &dec::u8(&mut u8(*x).as_slice()).unwrap()));
        random_u8().iter().for_each(|x| assert_eq!(x, &dec::be_u8_lengthed(NBYTES_8)(&mut be_u8_lengthed(NBYTES_8)(*x).as_slice()).unwrap()));
        random_u8().iter().for_each(|x| assert_eq!(x, &dec::le_u8_lengthed(NBYTES_8)(&mut le_u8_lengthed(NBYTES_8)(*x).as_slice()).unwrap()));
        random_u8().iter().for_each(|x| random_usize(4).iter()
        .for_each(|y| {
            assert_eq!(x, &dec::be_u8_lengthed(*y)(&mut be_u8_lengthed(*y)(*x).as_slice()).unwrap());
            assert_eq!(x, &dec::le_u8_lengthed(*y)(&mut le_u8_lengthed(*y)(*x).as_slice()).unwrap());
        }));

        random_u16().iter().for_each(|x| assert_eq!(x, &dec::u16(&mut u16(*x).as_slice()).unwrap()));
        random_u16().iter().for_each(|x| assert_eq!(x, &dec::be_u16(&mut be_u16(*x).as_slice()).unwrap()));
        random_u16().iter().for_each(|x| assert_eq!(x, &dec::le_u16(&mut le_u16(*x).as_slice()).unwrap()));
        random_u16().iter().for_each(|x| assert_eq!(x, &dec::be_u16_lengthed(NBYTES_16)(&mut be_u16_lengthed(NBYTES_16)(*x).as_slice()).unwrap()));
        random_u16().iter().for_each(|x| assert_eq!(x, &dec::le_u16_lengthed(NBYTES_16)(&mut le_u16_lengthed(NBYTES_16)(*x).as_slice()).unwrap()));
        random_u16().iter().for_each(|x| random_usize(NBYTES_16 * NBYTES_16).iter()
        .for_each(|y| { match *y < NBYTES_16 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_u16_lengthed(*y)(*x),
                    be_u16_lengthed(*y)(dec::be_u16_lengthed(*y)(&mut be_u16_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_u16_lengthed(*y)(*x),
                    le_u16_lengthed(*y)(dec::le_u16_lengthed(*y)(&mut le_u16_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_u16_lengthed(*y)(&mut be_u16_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_u16_lengthed(*y)(&mut le_u16_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_u32().iter().for_each(|x| assert_eq!(x, &dec::u32(&mut u32(*x).as_slice()).unwrap()));
        random_u32().iter().for_each(|x| assert_eq!(x, &dec::be_u32(&mut be_u32(*x).as_slice()).unwrap()));
        random_u32().iter().for_each(|x| assert_eq!(x, &dec::le_u32(&mut le_u32(*x).as_slice()).unwrap()));
        random_u32().iter().for_each(|x| assert_eq!(x, &dec::be_u32_lengthed(NBYTES_32)(&mut be_u32_lengthed(NBYTES_32)(*x).as_slice()).unwrap()));
        random_u32().iter().for_each(|x| assert_eq!(x, &dec::le_u32_lengthed(NBYTES_32)(&mut le_u32_lengthed(NBYTES_32)(*x).as_slice()).unwrap()));
        random_u32().iter().for_each(|x| random_usize(NBYTES_32 * NBYTES_32).iter()
        .for_each(|y| { match *y < NBYTES_32 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_u32_lengthed(*y)(*x),
                    be_u32_lengthed(*y)(dec::be_u32_lengthed(*y)(&mut be_u32_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_u32_lengthed(*y)(*x),
                    le_u32_lengthed(*y)(dec::le_u32_lengthed(*y)(&mut le_u32_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_u32_lengthed(*y)(&mut be_u32_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_u32_lengthed(*y)(&mut le_u32_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_u64().iter().for_each(|x| assert_eq!(x, &dec::u64(&mut u64(*x).as_slice()).unwrap()));
        random_u64().iter().for_each(|x| assert_eq!(x, &dec::be_u64(&mut be_u64(*x).as_slice()).unwrap()));
        random_u64().iter().for_each(|x| assert_eq!(x, &dec::le_u64(&mut le_u64(*x).as_slice()).unwrap()));
        random_u64().iter().for_each(|x| assert_eq!(x, &dec::be_u64_lengthed(NBYTES_64)(&mut be_u64_lengthed(NBYTES_64)(*x).as_slice()).unwrap()));
        random_u64().iter().for_each(|x| assert_eq!(x, &dec::le_u64_lengthed(NBYTES_64)(&mut le_u64_lengthed(NBYTES_64)(*x).as_slice()).unwrap()));
        random_u64().iter().for_each(|x| random_usize(NBYTES_64 * NBYTES_64).iter()
        .for_each(|y| { match *y < NBYTES_64 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_u64_lengthed(*y)(*x),
                    be_u64_lengthed(*y)(dec::be_u64_lengthed(*y)(&mut be_u64_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_u64_lengthed(*y)(*x),
                    le_u64_lengthed(*y)(dec::le_u64_lengthed(*y)(&mut le_u64_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_u64_lengthed(*y)(&mut be_u64_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_u64_lengthed(*y)(&mut le_u64_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_u128().iter().for_each(|x| assert_eq!(x, &dec::u128(&mut u128(*x).as_slice()).unwrap()));
        random_u128().iter().for_each(|x| assert_eq!(x, &dec::be_u128(&mut be_u128(*x).as_slice()).unwrap()));
        random_u128().iter().for_each(|x| assert_eq!(x, &dec::le_u128(&mut le_u128(*x).as_slice()).unwrap()));
        random_u128().iter().for_each(|x| assert_eq!(x, &dec::be_u128_lengthed(NBYTES_128)(&mut be_u128_lengthed(NBYTES_128)(*x).as_slice()).unwrap()));
        random_u128().iter().for_each(|x| assert_eq!(x, &dec::le_u128_lengthed(NBYTES_128)(&mut le_u128_lengthed(NBYTES_128)(*x).as_slice()).unwrap()));
        random_u128().iter().for_each(|x| random_usize(NBYTES_128 * NBYTES_128).iter()
        .for_each(|y| { match *y < NBYTES_128 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_u128_lengthed(*y)(*x),
                    be_u128_lengthed(*y)(dec::be_u128_lengthed(*y)(&mut be_u128_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_u128_lengthed(*y)(*x),
                    le_u128_lengthed(*y)(dec::le_u128_lengthed(*y)(&mut le_u128_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_u128_lengthed(*y)(&mut be_u128_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_u128_lengthed(*y)(&mut le_u128_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_i8().iter().for_each(|x| assert_eq!(x, &dec::i8(&mut i8(*x).as_slice()).unwrap()));
        random_i8().iter().for_each(|x| assert_eq!(x, &dec::be_i8_lengthed(NBYTES_8)(&mut be_i8_lengthed(NBYTES_8)(*x).as_slice()).unwrap()));
        random_i8().iter().for_each(|x| assert_eq!(x, &dec::le_i8_lengthed(NBYTES_8)(&mut le_i8_lengthed(NBYTES_8)(*x).as_slice()).unwrap()));
        random_i8().iter().for_each(|x| random_usize(4).iter()
        .for_each(|y| {
            assert_eq!(x, &dec::be_i8_lengthed(*y)(&mut be_i8_lengthed(*y)(*x).as_slice()).unwrap());
            assert_eq!(x, &dec::le_i8_lengthed(*y)(&mut le_i8_lengthed(*y)(*x).as_slice()).unwrap());
        }));

        random_i16().iter().for_each(|x| assert_eq!(x, &dec::i16(&mut i16(*x).as_slice()).unwrap()));
        random_i16().iter().for_each(|x| assert_eq!(x, &dec::be_i16(&mut be_i16(*x).as_slice()).unwrap()));
        random_i16().iter().for_each(|x| assert_eq!(x, &dec::le_i16(&mut le_i16(*x).as_slice()).unwrap()));
        random_i16().iter().for_each(|x| assert_eq!(x, &dec::be_i16_lengthed(NBYTES_16)(&mut be_i16_lengthed(NBYTES_16)(*x).as_slice()).unwrap()));
        random_i16().iter().for_each(|x| assert_eq!(x, &dec::le_i16_lengthed(NBYTES_16)(&mut le_i16_lengthed(NBYTES_16)(*x).as_slice()).unwrap()));
        random_i16().iter().for_each(|x| random_usize(NBYTES_16 * NBYTES_16).iter()
        .for_each(|y| { match *y < NBYTES_16 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_i16_lengthed(*y)(*x),
                    be_i16_lengthed(*y)(dec::be_i16_lengthed(*y)(&mut be_i16_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_i16_lengthed(*y)(*x),
                    le_i16_lengthed(*y)(dec::le_i16_lengthed(*y)(&mut le_i16_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_i16_lengthed(*y)(&mut be_i16_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_i16_lengthed(*y)(&mut le_i16_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_i32().iter().for_each(|x| assert_eq!(x, &dec::i32(&mut i32(*x).as_slice()).unwrap()));
        random_i32().iter().for_each(|x| assert_eq!(x, &dec::be_i32(&mut be_i32(*x).as_slice()).unwrap()));
        random_i32().iter().for_each(|x| assert_eq!(x, &dec::le_i32(&mut le_i32(*x).as_slice()).unwrap()));
        random_i32().iter().for_each(|x| assert_eq!(x, &dec::be_i32_lengthed(NBYTES_32)(&mut be_i32_lengthed(NBYTES_32)(*x).as_slice()).unwrap()));
        random_i32().iter().for_each(|x| assert_eq!(x, &dec::le_i32_lengthed(NBYTES_32)(&mut le_i32_lengthed(NBYTES_32)(*x).as_slice()).unwrap()));
        random_i32().iter().for_each(|x| random_usize(NBYTES_32 * NBYTES_32).iter()
        .for_each(|y| { match *y < NBYTES_32 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_i32_lengthed(*y)(*x),
                    be_i32_lengthed(*y)(dec::be_i32_lengthed(*y)(&mut be_i32_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_i32_lengthed(*y)(*x),
                    le_i32_lengthed(*y)(dec::le_i32_lengthed(*y)(&mut le_i32_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_i32_lengthed(*y)(&mut be_i32_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_i32_lengthed(*y)(&mut le_i32_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_i64().iter().for_each(|x| assert_eq!(x, &dec::i64(&mut i64(*x).as_slice()).unwrap()));
        random_i64().iter().for_each(|x| assert_eq!(x, &dec::be_i64(&mut be_i64(*x).as_slice()).unwrap()));
        random_i64().iter().for_each(|x| assert_eq!(x, &dec::le_i64(&mut le_i64(*x).as_slice()).unwrap()));
        random_i64().iter().for_each(|x| assert_eq!(x, &dec::be_i64_lengthed(NBYTES_64)(&mut be_i64_lengthed(NBYTES_64)(*x).as_slice()).unwrap()));
        random_i64().iter().for_each(|x| assert_eq!(x, &dec::le_i64_lengthed(NBYTES_64)(&mut le_i64_lengthed(NBYTES_64)(*x).as_slice()).unwrap()));
        random_i64().iter().for_each(|x| random_usize(NBYTES_64 * NBYTES_64).iter()
        .for_each(|y| { match *y < NBYTES_64 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_i64_lengthed(*y)(*x),
                    be_i64_lengthed(*y)(dec::be_i64_lengthed(*y)(&mut be_i64_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_i64_lengthed(*y)(*x),
                    le_i64_lengthed(*y)(dec::le_i64_lengthed(*y)(&mut le_i64_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_i64_lengthed(*y)(&mut be_i64_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_i64_lengthed(*y)(&mut le_i64_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));

        random_i128().iter().for_each(|x| assert_eq!(x, &dec::i128(&mut i128(*x).as_slice()).unwrap()));
        random_i128().iter().for_each(|x| assert_eq!(x, &dec::be_i128(&mut be_i128(*x).as_slice()).unwrap()));
        random_i128().iter().for_each(|x| assert_eq!(x, &dec::le_i128(&mut le_i128(*x).as_slice()).unwrap()));
        random_i128().iter().for_each(|x| assert_eq!(x, &dec::be_i128_lengthed(NBYTES_128)(&mut be_i128_lengthed(NBYTES_128)(*x).as_slice()).unwrap()));
        random_i128().iter().for_each(|x| assert_eq!(x, &dec::le_i128_lengthed(NBYTES_128)(&mut le_i128_lengthed(NBYTES_128)(*x).as_slice()).unwrap()));
        random_i128().iter().for_each(|x| random_usize(NBYTES_128 * NBYTES_128).iter()
        .for_each(|y| { match *y < NBYTES_128 {
            // there is information loss here, due to not enough bytes in the input
            true => {
                assert_eq!(
                    be_i128_lengthed(*y)(*x),
                    be_i128_lengthed(*y)(dec::be_i128_lengthed(*y)(&mut be_i128_lengthed(*y)(*x).as_slice()).unwrap()),
                );
                assert_eq!(
                    le_i128_lengthed(*y)(*x),
                    le_i128_lengthed(*y)(dec::le_i128_lengthed(*y)(&mut le_i128_lengthed(*y)(*x).as_slice()).unwrap()),
                );
            },
            // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
            false => {
                assert_eq!(x, &dec::be_i128_lengthed(*y)(&mut be_i128_lengthed(*y)(*x).as_slice()).unwrap());
                assert_eq!(x, &dec::le_i128_lengthed(*y)(&mut le_i128_lengthed(*y)(*x).as_slice()).unwrap());
            },
        }}));
    }
}