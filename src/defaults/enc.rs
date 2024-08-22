use super::codecs::ber;

pub fn ber_length<T: ber::OfBerLength>(input: &T) -> Vec<u8> {
    ber::BerLength::<T>::encode(input)
}

pub fn ber_oid<T: ber::OfBerOid>(input: &T) -> Vec<u8> {
    ber::BerOid::<T>::encode(input)
}