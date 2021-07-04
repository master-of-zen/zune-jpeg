//! Contains most common errors that may be encountered in decoding a Decoder image
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::misc::{
    START_OF_FRAME_EXT_AR, START_OF_FRAME_EXT_SEQ, START_OF_FRAME_LOS_SEQ,
    START_OF_FRAME_LOS_SEQ_AR, START_OF_FRAME_PROG_DCT, START_OF_FRAME_PROG_DCT_AR,
};

/// Common Decode errors
#[allow(clippy::module_name_repetitions)]
pub enum DecodeErrors {
    Format(String),
    /// Illegal Magic Bytes
    IllegalMagicBytes(u16),
    /// problems with the Huffman Tables in a Decoder file
    HuffmanDecode(String),
    /// Image has zero width
    ZeroError,
    /// Discrete Quantization Tables error
    DqtError(String),
    /// Start of scan errors
    SosError(String),
    /// Start of frame errors
    SofError(String),
    /// UnsupportedImages
    Unsupported(UnsupportedSchemes),
    /// Unset tables
    UnsetValues(String),
    /// MCU errors
    MCUError(String),
    /// Exhausted data
    ExhaustedData,
}
impl Debug for DecodeErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Format(ref a) => write!(f, "{:?}", a),
            Self::HuffmanDecode(ref reason) => {
                write!(f, "Error decoding huffman tables.Reason:{}", reason)
            }
            Self::ZeroError => write!(f, "Image width or height is set to zero, cannot continue"),
            Self::DqtError(ref reason) => write!(f, "Error parsing DQT segment. Reason:{}", reason),
            Self::SosError(ref reason) => write!(f, "Error parsing SOS Segment. Reason:{}", reason),
            Self::SofError(ref reason) => write!(f, "Error parsing SOF segment. Reason:{}", reason),
            Self::IllegalMagicBytes(bytes) => {
                write!(f, "Error parsing image. Illegal start bytes:{}", bytes)
            }
            Self::MCUError(ref reason) => write!(f, "Error in decoding MCU. Reason {}", reason),
            Self::Unsupported(ref image_type) => {
                write!(f, "{:?}", image_type)
            }
            Self::ExhaustedData => write!(f, "Exhausted data in the image"),
            Self::UnsetValues(ref values) => write!(f, "{}", values),
        }
    }
}
impl Display for DecodeErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Format(ref a) => write!(f, "{}", a),
            Self::HuffmanDecode(ref reason) => {
                write!(f, "Error decoding huffman tables.Reason:{}", reason)
            }
            Self::ZeroError => write!(f, "Image width or height is set to zero, cannot continue"),
            Self::DqtError(ref reason) => write!(f, "Error parsing DQT segment. Reason:{}", reason),
            Self::SosError(ref reason) => write!(f, "Error parsing SOS Segment. Reason:{}", reason),
            Self::SofError(ref reason) => write!(f, "Error parsing SOF segment. Reason:{}", reason),
            Self::IllegalMagicBytes(bytes) => {
                write!(f, "Error parsing image. Illegal start bytes:{}", bytes)
            }
            Self::Unsupported(ref image_type) => {
                write!(f, "{:?}", image_type)
            }
            Self::MCUError(ref reason) => write!(f, "Error in decoding MCU. Reason {}", reason),
            Self::ExhaustedData => write!(f, "Exhausted data in the image"),
            Self::UnsetValues(ref values) => write!(f, "{}", values),
        }
    }
}
impl Error for DecodeErrors {}

/// Contains Unsupported/Yet-to-be supported Decoder image encoding types.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum UnsupportedSchemes {
    /// SOF_1 Extended sequential DCT,Huffman coding
    ExtendedSequentialHuffman,
    /// Progressive DCT, Huffman coding
    ProgressiveDctHuffman,
    /// Lossless (sequential), huffman coding,
    LosslessHuffman,
    /// Extended sequential DEC, arithmetic coding
    ExtendedSequentialDctArithmetic,
    /// Progressive DCT, arithmetic coding,
    ProgressiveDctArithmetic,
    /// Lossless ( sequential), arithmetic coding
    LosslessArithmetic,
}
impl Debug for UnsupportedSchemes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::ExtendedSequentialHuffman => {
                write!(f,"The library cannot yet decode images encoded using Extended Sequential Huffman  encoding scheme yet.")
            }
            Self::ProgressiveDctHuffman => {
                write!(f, "The library cannot yet decode images encoded using Progressive Huffman Encoding scheme.")
            }
            Self::LosslessHuffman => {
                write!(f,"The library cannot yet decode images encoded with Lossless Huffman encoding scheme")
            }
            Self::ExtendedSequentialDctArithmetic => {
                write!(f,"The library cannot yet decode Images Encoded with Extended Sequential DCT Arithmetic scheme")
            }
            Self::ProgressiveDctArithmetic => {
                write!(f,"The library cannot yet decode images encoded with Progressive DCT Arithmetic scheme")
            }
            Self::LosslessArithmetic => {
                write!(f,"The library cannot yet decode images encoded with Lossless Arithmetic encoding scheme")
            }
        }
    }
}
impl UnsupportedSchemes {
    #[must_use]
    pub fn from_int(int: u8) -> Option<UnsupportedSchemes> {
        let int = u16::from_be_bytes([0xff, int]);

        match int {
            START_OF_FRAME_PROG_DCT => Some(Self::ProgressiveDctHuffman),
            START_OF_FRAME_PROG_DCT_AR => Some(Self::ProgressiveDctArithmetic),
            START_OF_FRAME_LOS_SEQ => Some(Self::LosslessHuffman),
            START_OF_FRAME_LOS_SEQ_AR => Some(Self::LosslessArithmetic),
            START_OF_FRAME_EXT_SEQ => Some(Self::ExtendedSequentialHuffman),
            START_OF_FRAME_EXT_AR => Some(Self::ExtendedSequentialDctArithmetic),
            _ => None,
        }
    }
}
