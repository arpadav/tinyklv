// --------------------------------------------------
// external
// --------------------------------------------------
use thisenum::Const;

// --------------------------------------------------
// tinyklv
// --------------------------------------------------
use tinyklv::Klv;
use tinyklv::prelude::*;

#[cfg(any(
    feature = "misb0102-12",
))]
#[derive(Klv, Debug)]
#[klv(
    stream = &[u8],
    sentinel = b"\x06\x0E\x2B\x34\x02\x01\x01\x01\x02\x08\x02\x00\x00\x00\x00\x00",
    key(enc = tinyklv::codecs::ber::enc::ber_oid,
        dec = tinyklv::codecs::ber::dec::ber_oid::<u64>),
    len(enc = tinyklv::codecs::ber::enc::ber_length,
        dec = tinyklv::codecs::ber::dec::ber_length),
    default(ty = u16, dec = tinyklv::codecs::binary::dec::be_u16),
    default(ty = String, dyn = true, dec = tinyklv::codecs::binary::dec::to_string_ascii),
)]
/// Security Metadata Universal and Local Sets for Motion Imagery Data
/// 
/// MISB Standard 0102
/// 
/// For more information, see [Motion Imagery Standards Board (MISB)](https://nsgreg.nga.mil/misb.jsp)
pub struct Misb0102LocalSet {
    #[klv(key = 0x01, dec = SecurityClassification::decode)]
    /// (Mandatory) The Security Classification metadata element represents 
    /// the overall security classification of the Motion Imagery Data 
    /// in accordance with U.S. and NATO classification guidance. Values
    /// allowed include: TOP SECRET, SECRET, CONFIDENTIAL, RESTRICTED, and
    /// UNCLASSIFIED (all caps) followed by a double forward slash “//”.
    /// This is a mandatory entry in a Security Metadata set.
    /// 
    /// See [`SecurityClassification`]
    pub security_classification: SecurityClassification,

    #[klv(key = 0x02, dec = CountryCodingMethod::decode_tag_02)]
    /// (Mandatory) This metadata element identifies the country coding method
    /// for the Classifying Country (Par. 6.1.3) and Releasing Instructions
    /// (Par. 6.1.6) metadata. The Country Coding Method value allows GEC two-letter
    /// or four-letter alphabetic country code (legacy systems only); ISO-3166 [15] [16]
    /// two-letter, three-letter, or three-digit numeric; STANAG 1059 [17] two-letter
    /// or threeletter codes; and GENC two-letter, three-letter or three-digit numeric.
    /// GENC administrative subdivision codes are not applicable
    /// 
    /// See [`CountryCodingMethod`]
    pub country_coding_method: CountryCodingMethod,

    #[klv(key = 0x03)]
    /// (Mandatory) The Classifying Country metadata element contains
    /// a value for the classifying country code preceded by a double slash "//."
    /// 
    /// Example of classifying country:
    /// 
    /// ```text
    /// //CZE (Example of GENC code)
    /// //GB (Example of ISO-3166 code)
    /// ```
    pub classifying_country: String,

    #[klv(key = 0x04)]
    /// (Contextual) Sensitive Compartmented Information (SCI) / Special
    /// Handling Instructions (SHI) Information
    pub sci_shi_information: Option<String>,

    #[klv(key = 0x05)]
    /// (Contextual) The Caveats metadata element represents pertinent
    /// caveats (or code words) from each category of the appropriate
    /// security entity register. Entries in this field may be abbreviated
    /// or spelled out as free text entries.
    pub caveats: Option<String>,

    #[klv(key = 0x06)]
    /// (Contextual) The Releasing Instructions metadata element contains
    /// a list of country codes to indicate the countries to which the
    /// Motion Imagery Data is releasable.
    /// 
    /// The use of blank spaces to separate country codes, instead of semi-colons
    /// or other characters, is to comply with security guidelines, and to
    /// allow parsing of fields by automated security screening systems. Various
    /// countries and international organizations have differing requirements
    /// MISB ST 0102.12 Security Metadata Universal and Local Sets for Motion
    /// Imagery Data 22 June 2017 Motion Imagery Standards Board 7 regarding
    /// the proper encoding of releasing instructions. Systems need to follow
    /// the security guidelines appropriate to their mission.
    pub releasing_instructions: Option<String>,

    #[klv(key = 0x07)]
    /// (Contextual) The Classified By metadata element identifies the name and
    /// type of authority used to classify the Motion Imagery Data. The metadata
    /// element is free text and can contain either the original classification
    /// authority name and position or personal identifier, or the title of the
    /// document or security classification guide used to classify the data.
    pub classified_by: Option<String>,

    #[klv(key = 0x08)]
    /// (Contextual) The Derived From metadata element contains information about
    /// the original source of data from which the classification was derived. The
    /// metadata element is free text.
    pub derived_from: Option<String>,

    #[klv(key = 0x09)]
    /// (Contextual) The Classification Reason metadata element contains the reason
    /// for classification or a citation from a document. The metadata element is
    /// free text.
    pub classification_reason: Option<String>,

    #[klv(key = 0x0A, dyn = true, dec = tinyklv::as_date!(
        tinyklv::dec::binary::to_string_ascii,
        "%Y%m%d",
    ))]
    /// (Contextual) The Declassification Date metadata element provides a date when
    /// the classified material may be automatically declassified.
    pub declassification_date: Option<chrono::NaiveDate>,

    #[klv(key = 0x0B)]
    /// (Contextual) The Classification and Marking System metadata element identifies
    /// the classification or marking system used in the Security Metadata set as
    /// determined by the appropriate security entity for the country originating.
    /// the data. 
    pub classification_and_marking_system: Option<String>,

    #[klv(key = 0x0C, dec = CountryCodingMethod::decode_tag_0c)]
    /// (Mandatory) The Object Country Coding Method metadata element identifies
    /// the coding method for the Object Country Code (Par. 6.1.13) metadata. This
    /// element allows use of GEC two-letter or fourletter alphabetic country code
    /// (legacy systems only); ISO-3166 two-letter, three-letter, or three-digit
    /// numeric; STANAG 1059 two-letter or three-letter codes; and GENC two-letter,
    /// three-letter, three-digit numeric or administrative subdivisions.
    /// 
    /// Use of this element in version 6 of this Standard and later is mandatory.
    /// In version 5 and earlier, it was optional; its absence indicates the default
    /// GENC two-letter coding method was used in the Object Country Code element.
    pub object_country_coding_method: Option<CountryCodingMethod>,

    #[klv(key = 0x0D, dec = tinyklv::codecs::binary::dec::to_string_utf16)]
    /// (Mandatory) The Object Country Codes metadata element contains a value
    /// identifying the country (or countries), which is the object of the Motion
    /// Imagery Data.
    /// 
    /// The object country codes of other represented geographic regions may be
    /// included in addition to the country code of the geographic region under
    /// the center of the image frame. Note: The use of the semi-colon to separate
    /// country codes, instead of blanks or other characters, is to allow processing
    /// by current, automated imagery processing and management tools.
    pub object_country_codes: Option<String>,

    #[klv(key = 0x0E)]
    /// (Optional) The Classification Comments metadata element allows for security
    /// related comments and format changes necessary in the future. This field may
    /// be used in addition to those required by appropriate security entity and is
    /// optional.
    pub classification_comments: Option<String>,

    #[klv(key = 0x16)]
    /// (Mandatory) The Version metadata element indicates the version number
    /// of MISB ST 0102 referenced.
    pub version: u16,

    #[klv(key = 0x17, dyn = true, dec = tinyklv::as_date!(
        tinyklv::dec::binary::to_string_ascii,
        "%Y-%m-%d",
    ))]
    /// (Optional) This metadata element provides the effective date (promulgation
    /// date) of the source (FIPS 10-4, ISO 3166, GENC 2.0, or STANAG 1059) used
    /// for the Classifying Country and Releasing Instructions Country Coding Method.
    /// As ISO 3166 is updated by dated circulars, not by version revision, the
    /// ISO 8601 YYYY-MM-DD formatted date is used.
    /// 
    /// See [`Misb0102LocalSet::country_coding_method`]
    pub country_coding_method_version_date: chrono::NaiveDate,

    #[klv(key = 0x18, dyn = true, dec = tinyklv::as_date!(
        tinyklv::dec::binary::to_string_ascii,
        "%Y-%m-%d",
    ))]
    /// (Optional) The Object Country Coding Method Version Date metadata element
    /// is the effective date (promulgation date) of the source (FIPS 10-4, ISO 3166,
    /// GENC 2.0, or STANAG 1059) used for the Object Country Coding Method. As
    /// ISO 3166 is updated by dated circulars, not by version revision, the ISO 8601
    /// YYYY-MM-DD formatted date is used.
    /// 
    /// See [`Misb0102LocalSet::object_country_coding_method`]
    pub object_country_coding_method_version_date: chrono::NaiveDate,
}

#[derive(Const)]
#[armtype(u8)]
/// MISB Standard 0102 Security Classification
/// 
/// See [`Misb0102LocalSet::security_classification`]
pub enum SecurityClassification {
    #[value = 1]
    Unclassified,
    #[value = 2]
    Restricted,
    #[value = 3]
    Confidential,
    #[value = 4]
    Secret,
    #[value = 5]
    TopSecret,
}
/// [`SecurityClassification`] implementation of [`tinyklv::prelude::Decode`]
impl tinyklv::prelude::Decode<&[u8]> for SecurityClassification {
    fn decode(input: &mut &[u8]) -> winnow::PResult<Self> {
        match SecurityClassification::try_from(
            tinyklv::dec::binary::be_u8.parse_next(input)?
        ) {
            Ok(v) => Ok(v),
            Err(_) => Err(tinyklv::err!()),
        }
    }
}
/// [`SecurityClassification`] implementation of [`tinyklv::prelude::Encode`]
impl tinyklv::prelude::Encode<Vec<u8>> for SecurityClassification {
    fn encode(&self) -> Vec<u8> {
        return vec![*self.value()]
    }
}
/// [`SecurityClassification`] implementation of [`std::string::ToString`]
impl std::string::ToString for SecurityClassification {
    fn to_string(&self) -> String {
        match self {
            Self::Unclassified => String::from("UNCLASSIFIED//"),
            Self::Restricted => String::from("RESTRICTED//"),
            Self::Confidential => String::from("CONFIDENTIAL//"),
            Self::Secret => String::from("SECRET//"),
            Self::TopSecret => String::from("TOP SECRET//"),
        }
    }
}

#[derive(Const)]
#[armtype(u8)]
/// MISB Standard 0102 Country Coding Method
/// 
/// See [`Misb0102LocalSet::country_coding_method`]
pub enum CountryCodingMethod {
    #[value = 0x01]
    Iso3166TwoLetter,
    #[value = 0x02]
    Iso3166ThreeLetter,
    #[value = 0x03]
    Fips104TwoLetter,
    #[value = 0x04]
    Fips104FourLetter,
    #[value = 0x05]
    Iso3166Numeric,
    #[value = 0x06]
    Stanag1059TwoLetter,
    #[value = 0x07]
    Stanag1059ThreeLetter,
    #[value = 0x0A]
    Fips104Mixed,
    #[value = 0x0B]
    Iso3166Mixed,
    #[value = 0x0C]
    Stanag1059Mixed,
    #[value = 0x0D]
    GencTwoLetter,
    #[value = 0x0E]
    GencThreeLetter,
    #[value = 0x0F]
    GencNumeric,
    #[value = 0x10]
    GencMixed,
    #[value = 0x40]
    GencAdminSub,
}
/// [`CountryCodingMethod`] implementation
impl CountryCodingMethod {
    /// Decodes the tag 0x02 of [`Misb0102LocalSet`], where some values
    /// are omitted
    pub(crate) fn decode_tag_02(input: &mut &[u8]) -> winnow::PResult<Self> {
        match CountryCodingMethod::try_from(
            tinyklv::dec::binary::be_u8.parse_next(input)?
        ) {
            Ok(v) => match v {
                // These values are omitted
                CountryCodingMethod::GencAdminSub => Err(tinyklv::err!()),
                _ => Ok(v),
            },
            Err(_) => Err(tinyklv::err!()),
        }
    }
    /// Decodes the tag 0x0C of [`Misb0102LocalSet`], where some values
    /// are omitted
    pub(crate) fn decode_tag_0c(input: &mut &[u8]) -> winnow::PResult<Self> {
        match CountryCodingMethod::try_from(
            tinyklv::dec::binary::be_u8.parse_next(input)?
        ) {
            Ok(v) => match v {
                // These values are omitted
                CountryCodingMethod::Fips104Mixed => Err(tinyklv::err!()),
                CountryCodingMethod::Iso3166Mixed => Err(tinyklv::err!()),
                CountryCodingMethod::Stanag1059Mixed => Err(tinyklv::err!()),
                _ => Ok(v),
            },
            Err(_) => Err(tinyklv::err!()),
        }
    }
}
/// [`CountryCodingMethod`] implementation of [`tinyklv::prelude::Decode`]
impl tinyklv::prelude::Decode<&[u8]> for CountryCodingMethod {
    fn decode(input: &mut &[u8]) -> winnow::PResult<Self> {
        CountryCodingMethod::try_from(
            tinyklv::dec::binary::be_u8.parse_next(input)?
        ).map_err(|_| tinyklv::err!())
    }
}
/// [`CountryCodingMethod`] implementation of [`tinyklv::prelude::Encode`]
impl tinyklv::prelude::Encode<Vec<u8>> for CountryCodingMethod {
    fn encode(&self) -> Vec<u8> {
        return vec![*self.value()]
    }
}

// mod test {
//     use super::Misb0102LocalSet;
//     use tinyklv::prelude::*;


//     #[test]
//     fn test() {
//         let mut input: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18];
//         let input = &mut input;
//         Misb0102LocalSet::decode(input);
//     }
// }