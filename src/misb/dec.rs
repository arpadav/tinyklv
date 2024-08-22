use chrono::{DateTime, TimeZone};

use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::error::AddContext;
use winnow::combinator::repeat;
use winnow::stream::Checkpoint;

/// Represents the number of MICROSECONDS elapsed since midnight
/// (00:00:00), January 1, 1970, not including leap seconds.
/// 
/// Since microseconds (1e-6 seconds)
/// 
/// # Example
/// 
/// ```
/// use chrono::TimeZone;
/// 
/// use tinyklv::prelude::*;
/// use tinyklv::misb::dec::precision_timestamp;
/// 
/// let mut val1: &[u8] = &(0x0004_59F4_A6AA_4AA8 as u64).to_be_bytes();
/// 
/// let result1 = precision_timestamp(&mut val1);
/// 
/// assert_eq!(result1, Ok(chrono::Utc.with_ymd_and_hms(2008, 10, 24, 0, 13, 29).unwrap() + chrono::Duration::nanoseconds(913_000_000)));
/// ```
pub fn precision_timestamp(input: &mut crate::Stream, _: usize) -> winnow::PResult<chrono::DateTime<chrono::Utc>> {
    let checkpoint = input.checkpoint();
    let ts = winnow::binary::be_u64.parse_next(input)?;
    let (secs, nanos) = (ts / 1_000_000, (ts % 1_000_000) * 1_000);
    match chrono::Utc.timestamp_opt(secs as i64, nanos as u32) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        chrono::LocalResult::None => Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new().add_context(input, &checkpoint, winnow::error::StrContext::Label("Invalid timestamp")),
        )),
        chrono::LocalResult::Ambiguous(_, _) => Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())),
    }
}

/// UTF-8 encoded string for the mission ID
/// 
/// # Example
/// 
/// ```
/// use tinyklv::prelude::*;
/// use tinyklv::misb::dec::mission_id;
/// 
/// let mut val1 = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// 
/// let result1 = mission_id(&mut val1, 10);
/// 
/// assert_eq!(result1, Ok(String::from("MISSION01")));
/// ```
pub fn mission_id(input: &mut crate::Stream, len: usize) -> winnow::PResult<String> {
    let checkpoint = input.checkpoint();
    winnow::token::take(len)
        .map(|s: &[u8]| match String::from_utf8(s.to_vec()) {
            Ok(s) => s,
            Err(e) => return Err(
                winnow::error::ErrMode::Backtrack(
                    winnow::error::ContextError::new().add_context(input, &checkpoint, winnow::error::StrContext::Label(e.to_string()
                ))
            )),
        })
        .parse_next(input)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp() {
        let mut val: &[u8] = &(0x0004_59F4_A6AA_4AA8 as u64).to_be_bytes();
        let val = precision_timestamp.parse_next(&mut val);
        assert_eq!(val, Ok(chrono::Utc.with_ymd_and_hms(2008, 10, 24, 0, 13, 29).unwrap() + chrono::Duration::nanoseconds(913_000_000)));
    }
}