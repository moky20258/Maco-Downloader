use tauri::command;
use reqwest::Client;
use serde_json;
use crate::api_types::{MusicItem, SearchResponse, UrlResponse};
use std::collections::HashMap;

const JBSOU_BASE: &str = "https://www.jbsou.cn/";
const VKEYS_BASE: &str = "https://api.vkeys.cn";
// TODO: 后续实现这些provider时启用
// const GEQUHAI_BASE: &str = "https://www.gequhai.com";
// const LIVEPOO_BASE: &str = "https://www.livepoo.cn";

#[command]
pub async fn search_music(query: String, provider: String) -> Result<SearchResponse, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    // 根据 provider 选择不同的搜索实现
    if provider.starts_with("jianbin") {
        search_jianbin(&client, &query, &provider).await
    } else if provider == "gequbao" {
        search_gequbao(&client, &query).await
    } else if provider == "gequhai" {
        search_gequhai(&client, &query).await
    } else if provider == "bugu" {
        search_bugu(&client, &query).await
    } else if provider == "qq" {
        search_qq(&client, &query).await
    } else if provider == "qqmp3" {
        search_qqmp3(&client, &query).await
    } else if provider == "migu" {
        search_migu(&client, &query).await
    } else if provider == "livepoo" {
        search_livepoo(&client, &query).await
    } else {
        // 默认使用 bugu
        search_bugu(&client, &query).await
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

async fn search_gequbao(_client: &Client, _query: &str) -> Result<SearchResponse, String> {
    // gequbao 需要爬取网页，暂时返回空
    // TODO: 实现网页爬取逻辑
    Ok(SearchResponse { items: vec![] })
}

async fn search_gequhai(_client: &Client, _query: &str) -> Result<SearchResponse, String> {
    // gequhai 需要爬取网页，暂时返回空
    // TODO: 实现网页爬取逻辑
    Ok(SearchResponse { items: vec![] })
}

async fn search_bugu(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let response = client.get("https://a.buguyy.top/newapi/search.php")
        .query(&[("keyword", query)])
        .header("accept", "application/json, text/plain, */*")
        .header("origin", "https://buguyy.top")
        .header("referer", "https://buguyy.top/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    let list = json.get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = list.iter()
        .filter_map(|item| {
            // id 可能是数字或字符串，需要兼容处理
            let id = if let Some(s) = item.get("id").and_then(|v| v.as_str()) {
                s.to_string()
            } else if let Some(n) = item.get("id").and_then(|v| v.as_i64()) {
                n.to_string()
            } else {
                return None;
            };
            
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("未知歌曲");
            let artist = item.get("singer").and_then(|v| v.as_str()).unwrap_or("未知歌手");
            let album = item.get("album").and_then(|v| v.as_str()).map(|s| s.to_string());
            let cover = item.get("picurl").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            Some(MusicItem {
                id,
                title: title.to_string(),
                artist: artist.to_string(),
                album,
                cover,
                duration: None,
                provider: "bugu".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

async fn search_qq(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let response = client.get(&format!("{}/v2/music/tencent/search/song", VKEYS_BASE))
        .query(&[("word", query)])
        .header("accept", "application/json, text/plain, */*")
        .header("origin", "https://y.qq.com")
        .header("referer", "https://y.qq.com/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    let list = json.get("data")
        .and_then(|d| d.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = list.iter()
        .filter_map(|item| {
            let id = item.get("mid")?.as_str()?;
            let title = item.get("song").and_then(|v| v.as_str()).unwrap_or("未知歌曲");
            let artist = item.get("singer").and_then(|v| v.as_str()).unwrap_or("未知歌手");
            let album = item.get("album").and_then(|v| v.as_str()).map(|s| s.to_string());
            let cover = item.get("cover").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            Some(MusicItem {
                id: id.to_string(),
                title: title.to_string(),
                artist: artist.to_string(),
                album,
                cover,
                duration: None,
                provider: "qq".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

async fn search_qqmp3(_client: &Client, _query: &str) -> Result<SearchResponse, String> {
    // TODO: 实现qqmp3搜索逻辑
    Ok(SearchResponse { items: vec![] })
}

async fn search_migu(_client: &Client, _query: &str) -> Result<SearchResponse, String> {
    // TODO: 实现migu搜索逻辑
    Ok(SearchResponse { items: vec![] })
}

async fn search_livepoo(_client: &Client, _query: &str) -> Result<SearchResponse, String> {
    // TODO: 实现livepoo搜索逻辑
    Ok(SearchResponse { items: vec![] })
}

#[command]
pub async fn get_music_url(id: String, _provider: String) -> Result<UrlResponse, String> {
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
pub async fn download_music(id: String, _filename: String, provider: String) -> Result<Vec<u8>, String> {
    // 获取音乐 URL
    let url_response = get_music_url(id, provider).await?;
    let url = url_response.url;
    
    // 使用 reqwest 下载文件
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;
    
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    
    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Read bytes failed: {}", e))?;
    
    Ok(bytes.to_vec())
}
