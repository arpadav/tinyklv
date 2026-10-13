use tinyklv::{
    decoder::DecodeIterError,
    prelude::*,
    traits::{PartialError, PartialIterator},
};

struct Reading;

struct ReadingPartial;

impl Partial for ReadingPartial {
    type Final = Reading;

    fn finalize(self) -> Result<Reading, PartialError> {
        Ok(Reading)
    }
}

struct Feed;

impl PartialIterator<ReadingPartial> for Feed {
    fn next_resume(&mut self) -> Result<Reading, DecodeIterError> {
        Err(DecodeIterError::Eof)
    }

    fn next_fresh(&mut self) -> Result<Reading, DecodeIterError> {
        Err(DecodeIterError::Eof)
    }
}

fn main() {}
