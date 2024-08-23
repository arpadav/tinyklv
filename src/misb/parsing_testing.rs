use winnow::Parser;
use crate::codecs::ber::BerLength;

use super::test_data::data;

const UHL: &'static [u8] = &[0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01, 0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00];

fn uhl_take<'a>(input: &mut &'a [u8]) -> winnow::PResult<&'a [u8]> {
    winnow::token::literal(UHL).parse_next(input)
}

// fn key_decode(input: &mut &[u8]) -> winnow::PResult<u64> {
//     match crate::defaults::dec::ber_oid::<u64>.parse_next(input) {
//         Ok(x) => Ok(x.value),
//         Err(x) => Err(x),
//     }
// }

// fn len_decode(input: &mut &[u8]) -> winnow::PResult<u64> {
//     match crate::defaults::dec::ber_length::<u64>.parse_next(input) {
//         Ok(x) => match x {
//             BerLength::Short(x) => Ok(x as u64),
//             BerLength::Long(x) => Ok(x),
//         },
//         Err(x) => Err(x),
//     }
// }

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::misbtest::Misb0601;

    use super::*;
    use rand::Rng;

    #[test]
    fn test_misb() {
        let mut rng = rand::thread_rng();
        let binding = {
            let data = data();
            let idx = rng.gen_range(0..data.len());
            data[idx].clone()
        };
        let input = &mut binding.as_slice();
        
        Misb0601::decode(input).unwrap();

        println!("debug point");
    }
}