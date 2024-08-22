use tinyklv::Klv;

// key / len encoder / decoder's will always
// take the entire input buffer
// - e.g. VariableDecoder

// value encoder / decoder's will always
// take a fixed slice of the input buffer,
// determined by the value of the `len` field
// - e.g. FixedDecoder

// key/len xcoder, fixed ALWAYS false
// variants: fixed ALWAYS true
//
// TODO: think about include_self terminology.
//
// include self NEVER in key/len, optional in variant

#[derive(Klv)]
#[klv(
    sentinel = b"\x01",
    key(enc = ::tinyklv::enc::ber_oid,
        dec = ::tinyklv::dec::ber_oid),
    len(enc = ::tinyklv::enc::ber_length,
        dec = ::tinyklv::dec::ber_length),
    default(ty = u16, enc = this, dec = that),
    default(ty = f32, enc = foo, dec = bar),
    default(ty = Vec<f64>, enc = me),
)]
struct Misb0601 {
    #[klv(
        key = b"\x02",
        len = 3,
    )]
    checksum: u16,

    #[klv(
        key = b"\x03",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    val2: String,

    #[klv(
        key = b"\x04",
        len = 3,
        enc = custom_enc,
        dec = my_str_dec,
    )]
    another_val: Vec<f64>,

    #[klv(
        key = b"\x05",
        len = 3,
    )]
    yet_another_val: String,

    #[klv(
        key = b"\x06",
        len = 3,
    )]
    yet_yet_another_val: String,
}

fn main() {
    let size = b"\x01".len();
    println!("size: {}", size);
}
