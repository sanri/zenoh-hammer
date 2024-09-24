use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;
use zenoh::bytes::{ZBytes, ZDeserializeError};
use zenoh::{
    bytes::Encoding,
    qos::{CongestionControl, Priority, Reliability},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u16)]
pub enum KnownEncoding {
    ZBytes = 0,
    ZInt8 = 1,
    ZInt16 = 2,
    ZInt32 = 3,
    ZInt64 = 4,
    ZInt128 = 5,
    ZUint8 = 6,
    ZUint16 = 7,
    ZUint32 = 8,
    ZUint64 = 9,
    ZUint128 = 10,
    ZFloat32 = 11,
    ZFloat64 = 12,
    ZBool = 13,
    ZString = 14,
    ZError = 15,
    AppOctetStream = 16,
    TextPlain = 17,
    AppJson = 18,
    TextJson = 19,
    AppCdr = 20,
    AppCbor = 21,
    AppYaml = 22,
    TextYaml = 23,
    TextJson5 = 24,
    AppPythonSerializedObject = 25,
    AppProtobuf = 26,
    AppJavaSerializedObject = 27,
    AppOpenMetricsText = 28,
    ImagePng = 29,
    ImageJpeg = 30,
    ImageGif = 31,
    ImageBmp = 32,
    ImageWebP = 33,
    AppXml = 34,
    AppXWwwFormUrlencoded = 35,
    TextHtml = 36,
    TextXml = 37,
    TextCss = 38,
    TextJavascript = 39,
    TextMarkdown = 40,
    TextCsv = 41,
    AppSql = 42,
    AppCoapPayload = 43,
    AppJsonPathJson = 44,
    AppJsonSeq = 45,
    AppJsonPath = 46,
    AppJwt = 47,
    AppMp4 = 48,
    AppSoapXml = 49,
    AppYang = 50,
    AudioAac = 51,
    AudioFlac = 52,
    AudioMp4 = 53,
    AudioOgg = 54,
    AudioVorbis = 55,
    VideoH261 = 56,
    VideoH263 = 57,
    VideoH264 = 58,
    VideoH265 = 59,
    VideoH266 = 60,
    VideoMp4 = 61,
    VideoOgg = 62,
    VideoRaw = 63,
    VideoVp8 = 64,
    VideoVp9 = 65,
    #[num_enum(catch_all)]
    Other(u16),
}

impl KnownEncoding {
    pub fn from_encoding(encoding: &Encoding) -> KnownEncoding {
        let id: u16 = encoding.id();
        KnownEncoding::from(id)
    }
}

pub fn zenoh_value_abstract(encoding: &Encoding, data: &ZBytes) -> Result<String, String> {
    let parse_error = Err("parse error".to_string());
    let dot_ok = Ok("...".to_string());
    let get_str_front = |z_bytes: &ZBytes| -> Result<String, String> {
        if data.len() < 30 {
            data.deserialize::<String>()
                .map_or(parse_error.clone(), |s| Ok(s))
        } else {
            dot_ok.clone()
        }
    };

    let known_encoding = KnownEncoding::from_encoding(encoding);
    match known_encoding {
        KnownEncoding::ZInt8 => data
            .deserialize::<i8>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZInt16 => data
            .deserialize::<i16>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZInt32 => data
            .deserialize::<i32>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZInt64 => data
            .deserialize::<i64>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZInt128 => data
            .deserialize::<i128>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZUint8 => data
            .deserialize::<u8>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZUint16 => data
            .deserialize::<u16>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZUint32 => data
            .deserialize::<u32>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZUint64 => data
            .deserialize::<u64>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZUint128 => data
            .deserialize::<u128>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZFloat32 => data
            .deserialize::<f32>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZFloat64 => data
            .deserialize::<f64>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZBool => data
            .deserialize::<bool>()
            .map_or(parse_error, |i| Ok(format!("{i}"))),
        KnownEncoding::ZString => get_str_front(data),
        KnownEncoding::ZBytes => dot_ok,
        KnownEncoding::ZError => get_str_front(data),
        KnownEncoding::AppOctetStream => dot_ok,
        KnownEncoding::TextPlain => get_str_front(data),
        KnownEncoding::AppJson => get_str_front(data),
        KnownEncoding::TextJson => get_str_front(data),
        KnownEncoding::AppCdr => dot_ok,
        KnownEncoding::AppCbor => dot_ok,
        KnownEncoding::AppYaml => get_str_front(data),
        KnownEncoding::TextYaml => get_str_front(data),
        KnownEncoding::TextJson5 => get_str_front(data),
        KnownEncoding::AppPythonSerializedObject => dot_ok,
        KnownEncoding::AppProtobuf => dot_ok,
        KnownEncoding::AppJavaSerializedObject => dot_ok,
        KnownEncoding::AppOpenMetricsText => dot_ok,
        KnownEncoding::ImagePng => dot_ok,
        KnownEncoding::ImageJpeg => dot_ok,
        KnownEncoding::ImageGif => dot_ok,
        KnownEncoding::ImageBmp => dot_ok,
        KnownEncoding::ImageWebP => dot_ok,
        KnownEncoding::AppXml => get_str_front(data),
        KnownEncoding::AppXWwwFormUrlencoded => dot_ok,
        KnownEncoding::TextHtml => get_str_front(data),
        KnownEncoding::TextXml => get_str_front(data),
        KnownEncoding::TextCss => get_str_front(data),
        KnownEncoding::TextJavascript => get_str_front(data),
        KnownEncoding::TextMarkdown => get_str_front(data),
        KnownEncoding::TextCsv => get_str_front(data),
        KnownEncoding::AppSql => get_str_front(data),
        KnownEncoding::AppCoapPayload => dot_ok,
        KnownEncoding::AppJsonPathJson => get_str_front(data),
        KnownEncoding::AppJsonSeq => get_str_front(data),
        KnownEncoding::AppJsonPath => get_str_front(data),
        KnownEncoding::AppJwt => dot_ok,
        KnownEncoding::AppMp4 => dot_ok,
        KnownEncoding::AppSoapXml => dot_ok,
        KnownEncoding::AppYang => dot_ok,
        KnownEncoding::AudioAac => dot_ok,
        KnownEncoding::AudioFlac => dot_ok,
        KnownEncoding::AudioMp4 => dot_ok,
        KnownEncoding::AudioOgg => dot_ok,
        KnownEncoding::AudioVorbis => dot_ok,
        KnownEncoding::VideoH261 => dot_ok,
        KnownEncoding::VideoH263 => dot_ok,
        KnownEncoding::VideoH264 => dot_ok,
        KnownEncoding::VideoH265 => dot_ok,
        KnownEncoding::VideoH266 => dot_ok,
        KnownEncoding::VideoMp4 => dot_ok,
        KnownEncoding::VideoOgg => dot_ok,
        KnownEncoding::VideoRaw => dot_ok,
        KnownEncoding::VideoVp8 => dot_ok,
        KnownEncoding::VideoVp9 => dot_ok,
        KnownEncoding::Other(_) => dot_ok,
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZCongestionControl {
    Block,
    Drop,
}

impl From<CongestionControl> for ZCongestionControl {
    fn from(value: CongestionControl) -> Self {
        match value {
            CongestionControl::Block => ZCongestionControl::Block,
            CongestionControl::Drop => ZCongestionControl::Drop,
        }
    }
}

impl Into<CongestionControl> for ZCongestionControl {
    fn into(self) -> CongestionControl {
        match self {
            ZCongestionControl::Block => CongestionControl::Block,
            ZCongestionControl::Drop => CongestionControl::Drop,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZPriority {
    RealTime,
    InteractiveHigh,
    InteractiveLow,
    DataHigh,
    Data,
    DataLow,
    Background,
}

impl From<Priority> for ZPriority {
    fn from(value: Priority) -> Self {
        match value {
            Priority::RealTime => ZPriority::RealTime,
            Priority::InteractiveHigh => ZPriority::InteractiveHigh,
            Priority::InteractiveLow => ZPriority::InteractiveLow,
            Priority::DataHigh => ZPriority::DataHigh,
            Priority::Data => ZPriority::Data,
            Priority::DataLow => ZPriority::DataLow,
            Priority::Background => ZPriority::Background,
        }
    }
}

impl Into<Priority> for ZPriority {
    fn into(self) -> Priority {
        match self {
            ZPriority::RealTime => Priority::RealTime,
            ZPriority::InteractiveHigh => Priority::InteractiveHigh,
            ZPriority::InteractiveLow => Priority::InteractiveLow,
            ZPriority::DataHigh => Priority::DataHigh,
            ZPriority::Data => Priority::Data,
            ZPriority::DataLow => Priority::DataLow,
            ZPriority::Background => Priority::Background,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZReliability {
    Reliable,
    BestEffort,
}

impl From<Reliability> for ZReliability {
    fn from(value: Reliability) -> Self {
        match value {
            Reliability::Reliable => ZReliability::Reliable,
            Reliability::BestEffort => ZReliability::BestEffort,
        }
    }
}

impl Into<Reliability> for ZReliability {
    fn into(self) -> Reliability {
        match self {
            ZReliability::Reliable => Reliability::Reliable,
            ZReliability::BestEffort => Reliability::BestEffort,
        }
    }
}
