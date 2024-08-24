use winnow::Parser;

use misb::test_data::data;

const UHL: &'static [u8] = &[0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01, 0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00];

fn uhl_take<'a>(input: &mut &'a [u8]) -> winnow::PResult<&'a [u8]> {
    winnow::token::literal(UHL).parse_next(input)
}

use tinyklv::prelude::*;
use misb::misbtest::Misb0601;
use rand::Rng;

fn main() {
    let mut rng = rand::thread_rng();
    let binding = {
        let data = data();
        let idx = rng.gen_range(0..data.len());
        data[idx].clone()
    };
    let input = &mut binding.as_slice();
    
    let _ = Misb0601::decode(input);

    println!("debug point");
}