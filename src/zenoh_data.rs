use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;
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
