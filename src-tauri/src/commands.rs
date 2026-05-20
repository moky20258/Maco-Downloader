use tauri::command;
use reqwest::Client;
use serde_json;
use crate::api_types::{MusicItem, SearchResponse, UrlResponse};
use std::collections::HashMap;

const JBSOU_BASE: &str = "https://www.jbsou.cn/";

#[command]
pub async fn search_music(query: String, provider: String) -> Result<SearchResponse, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    // 目前实现 jianbin-kugou provider
    if provider.starts_with("jianbin") {
        search_jianbin(&client, &query, &provider).await
    } else {
        // 默认返回空
        Ok(SearchResponse { items: vec![] })
    }
}

async fn search_jianbin(client: &Client, query: &str, provider: &str) -> Result<SearchResponse, String> {
    let source = match provider {
        "jianbin-kugou" => "kugou",
        "jianbin-qq" => "qq",
        "jianbin-netease" => "netease",
        "jianbin-kuwo" => "kuwo",
        _ => "kugou",
    };

    let mut params = HashMap::new();
    params.insert("input", query);
    params.insert("filter", "name");
    params.insert("type", source);
    params.insert("page", "1");

    let response = client.post(JBSOU_BASE)
        .form(&params)
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36")
        .header("accept", "application/json, text/javascript, */*; q=0.01")
        .header("origin", "https://www.jbsou.cn")
        .header("referer", "https://www.jbsou.cn/")
        .header("x-requested-with", "XMLHttpRequest")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 解析 JSON
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    // 提取 data 数组
    let data = json.get("data")
        .and_then(|d| d.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = data.iter()
        .filter_map(|item| {
            let url = item.get("url")?.as_str()?;
            let absolute_url = to_absolute_url(url);
            
            Some(MusicItem {
                id: percent_encoding::utf8_percent_encode(&absolute_url, percent_encoding::NON_ALPHANUMERIC).to_string(),
                title: item.get("name").and_then(|v| v.as_str()).unwrap_or("未知歌曲").to_string(),
                artist: item.get("artist").and_then(|v| v.as_str()).unwrap_or("未知歌手").to_string(),
                album: item.get("album").and_then(|v| v.as_str()).map(|s| s.to_string()),
                cover: item.get("cover").and_then(|v| v.as_str()).map(to_absolute_url),
                duration: None,
                provider: provider.to_string(),
            })
        })
        .filter(|item| !item.id.is_empty())
        .collect();

    Ok(SearchResponse { items })
}

fn to_absolute_url(url: &str) -> String {
    if url.starts_with("http") {
        url.to_string()
    } else {
        format!("{}{}", JBSOU_BASE, url.trim_start_matches('/'))
    }
}

#[command]
pub async fn get_music_url(id: String, provider: String) -> Result<UrlResponse, String> {
    // 解码 ID
    let decoded = percent_encoding::percent_decode_str(&id)
        .decode_utf8()
        .map_err(|e| format!("Decode failed: {}", e))?;
    
    let url = decoded.to_string();
    
    if !url.starts_with("http") {
        return Err("Invalid URL".to_string());
    }

    // 返回原始 URL（jianbin provider 的 id 就是播放地址）
    Ok(UrlResponse { url })
}

#[command]
pub async fn download_music(id: String, filename: String, provider: String) -> Result<String, String> {
    // 返回下载 API URL，前端会处理实际下载
    get_music_url(id, provider).await.map(|resp| resp.url)
}
