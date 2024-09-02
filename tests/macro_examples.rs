use tinyklv::prelude::*;

#[test]
fn op_macro() {
    let mut input: &[u8] = &[0x00, 0x01];
    let input = &mut input;

    // let bruh = tinyklv::op!(input, tinyklv::dec::binary::be_u8, f64, * 100.0, - 10.0)(input);
    // assert_eq!(bruh, Ok(0x01));
    // expands to:
    // let _ = (parser.parse_next(input)? * 100.0) - 10.0;

    // super::op!(input, parser, f64, * 100.0, - 10.0, + 12.0, / 2.0, + 1.0);
    // expands to:
    // let _ = (((parser.parse_next(input)? * 100.0) - 10.0 + 12.0) / 2.0) + 1.0;
}