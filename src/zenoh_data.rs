use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumCount, EnumIter};
use zenoh::{
    bytes::{Encoding, ZBytes},
    qos::{CongestionControl, Priority, Reliability},
    query::{ConsolidationMode, QueryConsolidation, QueryTarget},
    sample::Locality,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, IntoPrimitive, EnumIter, EnumCount)]
#[repr(u16)]
pub enum KnownEncoding {
    ZBytes = 0,
    ZString = 1,
    ZSerialized = 2,
    AppOctetStream = 3,
    TextPlain = 4,
    AppJson = 5,
    TextJson = 6,
    AppCdr = 7,
    AppCbor = 8,
    AppYaml = 9,
    TextYaml = 10,
    TextJson5 = 11,
    AppPythonSerializedObject = 12,
    AppProtobuf = 13,
    AppJavaSerializedObject = 14,
    AppOpenMetricsText = 15,
    ImagePng = 16,
    ImageJpeg = 17,
    ImageGif = 18,
    ImageBmp = 19,
    ImageWebP = 20,
    AppXml = 21,
    AppXWwwFormUrlencoded = 22,
    TextHtml = 23,
    TextXml = 24,
    TextCss = 25,
    TextJavascript = 26,
    TextMarkdown = 27,
    TextCsv = 28,
    AppSql = 29,
    AppCoapPayload = 30,
    AppJsonPathJson = 31,
    AppJsonSeq = 32,
    AppJsonPath = 33,
    AppJwt = 34,
    AppMp4 = 35,
    AppSoapXml = 36,
    AppYang = 37,
    AudioAac = 38,
    AudioFlac = 39,
    AudioMp4 = 40,
    AudioOgg = 41,
    AudioVorbis = 42,
    VideoH261 = 43,
    VideoH263 = 44,
    VideoH264 = 45,
    VideoH265 = 46,
    VideoH266 = 47,
    VideoMp4 = 48,
    VideoOgg = 49,
    VideoRaw = 50,
    VideoVp8 = 51,
    VideoVp9 = 52,
    #[num_enum(catch_all)]
    Other(u16),
}

impl KnownEncoding {
    pub fn from_encoding(encoding: &Encoding) -> KnownEncoding {
        let id: u16 = encoding.id();
        KnownEncoding::from(id)
    }

    pub fn to_encoding(&self) -> Encoding {
        let id: u16 = (*self).into();
        Encoding::new(id, None)
    }

    pub fn to_u16(&self) -> u16 {
        let id: u16 = (*self).into();
        id
    }
}

pub fn zenoh_value_abstract(encoding: &Encoding, data: &ZBytes) -> Result<String, String> {
    let parse_error = Err("parse error".to_string());
    let dot_ok = Ok("...".to_string());
    let get_str_front = |z_bytes: &ZBytes| -> Result<String, String> {
        if z_bytes.len() < 30 {
            String::from_utf8(z_bytes.to_bytes().to_vec()).map_or(parse_error.clone(), |s| Ok(s))
        } else {
            dot_ok.clone()
        }
    };

    let known_encoding = KnownEncoding::from_encoding(encoding);
    match known_encoding {
        KnownEncoding::ZString => get_str_front(data),
        KnownEncoding::ZBytes => dot_ok,
        KnownEncoding::ZSerialized => dot_ok,
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

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr, EnumIter, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZCongestionControl {
    Drop,
    Block,
    BlockFirst,
}

impl From<CongestionControl> for ZCongestionControl {
    fn from(value: CongestionControl) -> Self {
        match value {
            CongestionControl::Block => ZCongestionControl::Block,
            CongestionControl::Drop => ZCongestionControl::Drop,
            CongestionControl::BlockFirst => ZCongestionControl::BlockFirst,
        }
    }
}

impl Into<CongestionControl> for ZCongestionControl {
    fn into(self) -> CongestionControl {
        match self {
            ZCongestionControl::Block => CongestionControl::Block,
            ZCongestionControl::Drop => CongestionControl::Drop,
            ZCongestionControl::BlockFirst => CongestionControl::BlockFirst,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr, EnumIter, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr, EnumIter, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Copy, AsRefStr, EnumIter, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZQueryTarget {
    BestMatching,
    All,
    AllComplete,
}

impl From<QueryTarget> for ZQueryTarget {
    fn from(value: QueryTarget) -> Self {
        match value {
            QueryTarget::BestMatching => ZQueryTarget::BestMatching,
            QueryTarget::All => ZQueryTarget::All,
            QueryTarget::AllComplete => ZQueryTarget::AllComplete,
        }
    }
}

impl Into<QueryTarget> for ZQueryTarget {
    fn into(self) -> QueryTarget {
        match self {
            ZQueryTarget::BestMatching => QueryTarget::BestMatching,
            ZQueryTarget::All => QueryTarget::All,
            ZQueryTarget::AllComplete => QueryTarget::AllComplete,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, EnumIter, Eq, PartialEq, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZConsolidation {
    Auto,
    None,
    Monotonic,
    Latest,
}

impl From<QueryConsolidation> for ZConsolidation {
    fn from(value: QueryConsolidation) -> Self {
        match value.mode() {
            ConsolidationMode::None => ZConsolidation::None,
            ConsolidationMode::Monotonic => ZConsolidation::Monotonic,
            ConsolidationMode::Latest => ZConsolidation::Latest,
            ConsolidationMode::Auto => ZConsolidation::Auto,
        }
    }
}

impl Into<QueryConsolidation> for ZConsolidation {
    fn into(self) -> QueryConsolidation {
        match self {
            ZConsolidation::Auto => QueryConsolidation::AUTO,
            ZConsolidation::None => QueryConsolidation::from(ConsolidationMode::None),
            ZConsolidation::Monotonic => QueryConsolidation::from(ConsolidationMode::Monotonic),
            ZConsolidation::Latest => QueryConsolidation::from(ConsolidationMode::Latest),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, EnumIter, Eq, PartialEq, AsRefStr, Default)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ZLocality {
    SessionLocal,
    Remote,
    #[default]
    Any,
}

impl From<Locality> for ZLocality {
    fn from(value: Locality) -> Self {
        match value {
            Locality::SessionLocal => ZLocality::SessionLocal,
            Locality::Remote => ZLocality::Remote,
            Locality::Any => ZLocality::Any,
        }
    }
}

impl Into<Locality> for ZLocality {
    fn into(self) -> Locality {
        match self {
            ZLocality::SessionLocal => Locality::SessionLocal,
            ZLocality::Remote => Locality::Remote,
            ZLocality::Any => Locality::Any,
        }
    }
}
