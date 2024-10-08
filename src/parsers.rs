#![allow(unused_doc_comments)]
//! Contains [nom] parsers for various components within a DTED file.

// --------------------------------------------------
// external
// --------------------------------------------------
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::{map, map_res, opt},
    multi::count,
    number::complete::be_u16,
    sequence::{preceded, tuple},
    IResult,
};
use num_traits::{int::PrimInt, Unsigned};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::dted::*;
use crate::primitives::{Angle, AxisElement};

// --------------------------------------------------
// general constants
// --------------------------------------------------
/// Unsigned 16-bit integer sign bit
const U16_SIGN_BIT: u16 = 0x8000;
const U16_DATA_MSK: u16 = 0x7FFF;

/// Parses a byte slice into an unsigned integer
/// - Max precision is 32 bits (4294967296)
///
/// # Arguments
///
/// * `input` - A byte slice
///
/// # Returns
///
/// An option containing an unsigned integer
///
/// # Examples
///
/// ```
/// use dted2::parsers::to_uint;
/// assert_eq!(to_uint::<u32>(b"123"), Some(123 as u32));
/// ```
pub fn to_uint<U>(input: &[u8]) -> Option<U>
where
    U: PrimInt + Unsigned,
{
    U::from(input.iter().fold(0_u32, |acc, b| {
        // assert!(*b >= 0x30 && *b <= 0x39); // is a digit
        (acc * 10) + (*b - 0x30) as u32
    }))
}

/// Nom parser that parses `count` number of bytes and returns an unsigned integer
///
/// # Arguments
///
/// * `count` - The number of bytes to parse
///
/// # Returns
///
/// A result containing an unsigned integer of length `num`, or an error if
/// the input is invalid
///
/// # Examples
///
/// ```
/// use dted2::parsers::uint_parser;
/// assert_eq!(uint_parser::<u32>(3)(b"123"), Ok((&b""[..], 123 as u32)));
/// ```
pub fn uint_parser<U>(count: usize) -> impl Fn(&[u8]) -> IResult<&[u8], U>
where
    U: PrimInt + Unsigned,
{
    move |input| {
        map_res(take(count), |bytes: &[u8]| {
            to_uint::<U>(bytes).ok_or(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
        })(input)
    }
}

/// Nom parser that parses `count` number of bytes and returns an unsigned integer
/// If `count` is 0, a default value `default` is returned
///
/// # Arguments
///
/// * `count` - The number of bytes to parse
/// * `default` - The default value to return if `count` is 0
///
/// # Returns
///
/// A [std::result::Result] containing an unsigned integer of length `count`, or an error if
/// the input is invalid. If `count` is 0, `default` is returned
///
/// # Examples
///
/// ```
/// use dted2::parsers::uint_parser_with_default;
/// assert_eq!(uint_parser_with_default::<u32>(3, 0)(b"123"), Ok((&b""[..], 123 as u32)));
/// assert_eq!(uint_parser_with_default::<u32>(0, 0)(b"123"), Ok((&b"123"[..], 0 as u32)));
/// ```
pub fn uint_parser_with_default<U>(count: usize, default: U) -> impl Fn(&[u8]) -> IResult<&[u8], U>
where
    U: PrimInt + Unsigned,
{
    move |input| match count {
        0 => Ok((input, default)),
        _ => uint_parser(count)(input),
    }
}

/// Parses a byte slice into a [crate::primitives::Angle]
///
/// # Arguments
///
/// * `input` - A byte slice
/// * `num_deg` - The number of bytes to parse for degrees
/// * `num_min` - The number of bytes to parse for minutes
/// * `num_sec` - The number of bytes to parse for seconds
///
/// # Returns
///
/// An [Option] containing a [crate::primitives::Angle]
///
/// # Examples
///
/// ```
/// use dted2::parsers::to_angle;
/// use dted2::primitives::Angle;
/// assert_eq!(to_angle(b"12345", 3, 1, 1), Ok((&b""[..], Angle::new(123, 4, 5.0, false))));
/// assert_eq!(to_angle(b"12345W", 3, 1, 1), Ok((&b""[..], Angle::new(123, 4, 5.0, true))));
/// ```
pub fn to_angle(
    input: &[u8],
    num_deg: usize,
    num_min: usize,
    num_sec: usize,
) -> IResult<&[u8], Angle> {
    let (input, (deg, min, sec, sign)) = tuple((
        uint_parser_with_default(num_deg, 0u32),
        uint_parser_with_default(num_min, 0u32),
        uint_parser_with_default(num_sec, 0u32),
        opt(alt((
            map(tag("N"), |_| false),
            map(tag("S"), |_| true),
            map(tag("E"), |_| false),
            map(tag("W"), |_| true),
        ))),
    ))(input)?;
    Ok((
        input,
        Angle::new(deg as u16, min as u8, sec as f64, sign.unwrap_or(false)),
    ))
}

/// Nom parser that parses `num_deg`, `num_min`, and `num_sec` number of bytes and returns an angle
///
/// # Arguments
///
/// * `num_deg` - The number of bytes to parse for degrees
/// * `num_min` - The number of bytes to parse for minutes
/// * `num_sec` - The number of bytes to parse for seconds
///
/// # Examples
///
/// ```
/// use dted2::primitives::Angle;
/// use dted2::parsers::angle_parser;
/// assert_eq!(angle_parser(3, 1, 1)(b"12345"), Ok((&b""[..], Angle::new(123, 4, 5.0, false))));
/// assert_eq!(angle_parser(3, 1, 1)(b"12345W"), Ok((&b""[..], Angle::new(123, 4, 5.0, true))));
/// ```
pub fn angle_parser(
    num_deg: usize,
    num_min: usize,
    num_sec: usize,
) -> impl Fn(&[u8]) -> IResult<&[u8], Angle> {
    move |input| to_angle(input, num_deg, num_min, num_sec)
}

/// Parses a byte slice into an unsigned integer,
/// if the value is not a valid NAN DTED value
///
/// # Arguments
///
/// * `input` - A byte slice
///
/// # Returns
///
/// A [Option] containing a unsigned integer. Is None
/// if the value is a valid NAN value
///
/// # Examples
///
/// ```
/// use dted2::parsers::to_nan;
/// assert_eq!(to_nan::<u32>(b"NA$$", 4), Ok((&b""[..], None)));
/// assert_eq!(to_nan::<u32>(b"12345", 4), Ok((&b"5"[..], Some(1234 as u32))));
/// ```
pub fn to_nan<U>(input: &[u8], count: usize) -> IResult<&[u8], Option<U>>
where
    U: PrimInt + Unsigned,
{
    match tag::<_, _, nom::error::Error<_>>(RecognitionSentinel::NA.value())(input) {
        Ok((input, _)) => {
            let (input, _) = take(count - 2)(input)?;
            Ok((input, None))
        }
        Err(e) => match e {
            nom::Err::Error(err_input) => {
                uint_parser::<U>(count)(err_input.input).map(|(input, x)| (input, Some(x)))
            }
            _ => Err(e),
        },
    }
}

/// Nom parser for NAN (either Not a Number or Not Available) values in DTED
/// If not a valid NAN value, then the value (unsigned integer)
/// is returned as [Option::Some], otherwise [Option::None]
///
/// # Arguments
///
/// * `count` - The number of bytes to parse
///
/// # Returns
///
/// An [Option] containing an unsigned integer,
/// otherwise, if a valid NAN, returns [Option::None]
///
/// # Examples
///
/// ```
/// use dted2::parsers::nan_parser;
/// assert_eq!(nan_parser::<u32>(4)(b"NA$$"), Ok((&b""[..], None)));
/// assert_eq!(nan_parser::<u32>(4)(b"12345"), Ok((&b"5"[..], Some(1234 as u32))));
/// ```
pub fn nan_parser<U>(
    count: usize,
) -> impl Fn(&[u8]) -> Result<(&[u8], Option<U>), nom::Err<nom::error::Error<&[u8]>>>
where
    U: PrimInt + Unsigned,
{
    move |input| to_nan(input, count)
}

// // Helper function: Convert signed magnitude int to i16
// fn to_i16(x: u16) -> i16 {
//     if x & U16_SIGN_BIT == U16_SIGN_BIT {
//         -((x & !U16_SIGN_BIT) as i16)
//     } else {
//         x as i16
//     }
// }
/// Convert signed magnitude int to i16
///
/// # Arguments
///
/// * `x` - The signed magnitude int (2 bytes, formatted as u16)
///
/// # Returns
///
/// An i16, converted from the signed magnitude int
///
/// # Examples
///
/// ```
/// use dted2::parsers::to_i16;
/// assert_eq!(to_i16(0x0000), 0);
/// assert_eq!(to_i16(0x0003), 3);
/// assert_eq!(to_i16(0x8003), -3);
/// assert_eq!(to_i16(0x7fff), 32767);
/// assert_eq!(to_i16(0xFFFF), -32767);
/// ```
pub fn to_i16(x: u16) -> i16 {
    let v = (x & U16_DATA_MSK) as i16; // mask out the sign bit and get the value
    let s = ((x & U16_SIGN_BIT) >> 15) as i16; // extract sign bit and extend to i16 directly
    (1 - (s << 1)) * v // branchless negation, return (1 - 2s) * v
}

/// Nom parser for signed magnitude values in DTED
///
/// # Arguments
///
/// * `input` - A byte slice
///
/// # Returns
///
/// An [i16] parsed from the byte slice, using signed magnitude
/// convention
///
/// # Examples
///
/// ```
/// use dted2::parsers::signed_mag_parser;
/// assert_eq!(signed_mag_parser(&[0x00, 0x00]), Ok((&b""[..], 0)));
/// assert_eq!(signed_mag_parser(&[0x00, 0x03]), Ok((&b""[..], 3)));
/// assert_eq!(signed_mag_parser(&[0x80, 0x03]), Ok((&b""[..], -3)));
/// assert_eq!(signed_mag_parser(&[0x7f, 0xff]), Ok((&b""[..], 32767)));
/// assert_eq!(signed_mag_parser(&[0xff, 0xff]), Ok((&b""[..], -32767)));
/// ```
pub fn signed_mag_parser(input: &[u8]) -> IResult<&[u8], i16> {
    map_res(take(2_usize), |bytes: &[u8]| {
        Ok::<i16, nom::Err<nom::error::Error<&[u8]>>>(to_i16(u16::from_be_bytes([
            bytes[0], bytes[1],
        ])))
    })(input)
}

/// Nom parser for a [RawDTEDHeader]
///
/// # Arguments
///
/// * `input` - A byte slice
///
/// # Returns
///
/// A [RawDTEDHeader] parsed from the byte slice
///
/// # Examples
///
/// ```
/// use dted2::dted::RawDTEDHeader;
/// use dted2::primitives::{ Angle, AxisElement };
/// use dted2::parsers::dted_uhl_parser;
/// use dted2::dted::RecognitionSentinel;
///
/// assert_eq!(dted_uhl_parser(b"UHL11234556E8901234W123456789012UUUXXXXXXXXXXXX123445670XXXXXXXXXXXXXXXXXXXXXXXX"), Ok((&b""[..], RawDTEDHeader {
///     origin: AxisElement { lat: Angle::new(890, 12, 34.0, true), lon: Angle::new(123, 45, 56.0, false) },
///     interval_secs_x_10: AxisElement { lat: 5678, lon: 1234 },
///     accuracy: Some(9012),
///     count: AxisElement { lat: 4567, lon: 1234 },
/// })));
/// ```
pub fn dted_uhl_parser(input: &[u8]) -> IResult<&[u8], RawDTEDHeader> {
    // --------------------------------------------------
    // verify is UHL
    // --------------------------------------------------
    let (input, _) = tag(RecognitionSentinel::UHL.value())(input)?;
    // --------------------------------------------------
    // parse header
    // --------------------------------------------------
    let (
        input,
        (
            lon_origin,
            lat_origin,
            lon_interval_s,
            lat_interval_s,
            accuracy,
            _,
            lon_count,
            lat_count,
            _,
        ),
    ) = tuple((
        angle_parser(3, 2, 2),
        angle_parser(3, 2, 2),
        uint_parser(4),
        uint_parser(4),
        nan_parser(4),
        take(15_usize),
        uint_parser(4),
        uint_parser(4),
        take(25_usize),
    ))(input)?;
    // --------------------------------------------------
    // return
    // --------------------------------------------------
    Ok((
        input,
        RawDTEDHeader {
            origin: AxisElement::new(lat_origin, lon_origin),
            interval_secs_x_10: AxisElement::new(lat_interval_s, lon_interval_s),
            accuracy,
            count: AxisElement::new(lat_count, lon_count),
        },
    ))
}

pub fn dted_file_parser(input: &[u8]) -> IResult<&[u8], RawDTEDFile> {
    // --------------------------------------------------
    // get headers and header records
    // --------------------------------------------------
    let (input, (header, _dsi_record, _acc_record)) = tuple((
        dted_uhl_parser,
        // TODO: parse DSI record
        // TODO: parse ACC record
        take(DT2_DSI_RECORD_LENGTH),
        take(DT2_ACC_RECORD_LENGTH),
    ))(input)?;
    // --------------------------------------------------
    // parse the actual data
    // --------------------------------------------------
    let (input, records) = count(
        |input| parse_dted_record(input, header.count.lat as usize),
        header.count.lon as usize,
    )(input)?;
    // --------------------------------------------------
    // return
    // --------------------------------------------------
    Ok((
        input,
        RawDTEDFile {
            header,
            data: records,
            dsi_record: None,
            acc_record: None,
        },
    ))
}

// Parse a DTED record
pub fn parse_dted_record(input: &[u8], line_len: usize) -> IResult<&[u8], RawDTEDRecord> {
    let (input, (block_byte0, block_rest, lon_count, lat_count, elevations, _)) = tuple((
        preceded(
            tag(RecognitionSentinel::DATA.value()),
            take(1_usize), // starting block byte size, will always be 0
        ),
        be_u16,
        be_u16,
        be_u16,
        count(signed_mag_parser, line_len),
        take(4_usize), // checksum
    ))(input)?;
    // --------------------------------------------------
    // return
    // --------------------------------------------------
    Ok((
        input,
        RawDTEDRecord {
            blk_count: block_byte0[0] as u32 * 0x10000 + block_rest as u32,
            lon_count,
            lat_count,
            elevations,
        },
    ))
}
