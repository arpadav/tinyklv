use crate::prelude::*;
use super::codecs::ber;

pub fn ber_length<'s, T: ber::OfBerLength>(input: &mut crate::Stream<'s>) -> winnow::PResult<ber::BerLength<T>> {
    ber::BerLength::<T>::decode(input)
}

pub fn ber_oid<'s, T: ber::OfBerOid>(input: &mut crate::Stream<'s>) -> winnow::PResult<ber::BerOid<T>> {
    ber::BerOid::<T>::decode(input)
}