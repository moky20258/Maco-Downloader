use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MusicItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)] // 为将来扩展预留
pub struct PlayInfo {
    pub url: String,
    #[serde(rename = "type")]
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<MusicItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UrlResponse {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)] // 为将来扩展预留
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LyricsResponse {
    pub lyrics: String, // LRC格式的歌词文本
    pub has_lyrics: bool,
}
