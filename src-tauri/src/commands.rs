use tauri::command;
use reqwest::Client;
use serde_json;
use crate::api_types::{MusicItem, SearchResponse, UrlResponse};
use std::collections::HashMap;
use scraper::{Html, Selector};
use regex::Regex;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

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

async fn search_gequbao(client: &Client, query: &str) -> Result<SearchResponse, String> {
    // 访问搜索页面
    let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://www.gequbao.com/s/{}", encoded);
    let response = client.get(&url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("referer", "https://www.gequbao.com/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 解析HTML
    let document = Html::parse_document(&html);
    let link_selector = Selector::parse("a[href^=\"/music/\"]").map_err(|e| format!("Parse selector failed: {}", e))?;
    
    let items: Vec<MusicItem> = document.select(&link_selector)
        .filter_map(|el| {
            let href = el.value().attr("href")?;
            let re = Regex::new(r"/music/(\d+)").ok()?;
            let captures = re.captures(href)?;
            let id = captures.get(1)?.as_str();
            
            let title = el.text().collect::<Vec<_>>().join("").trim().to_string();
            if title.is_empty() || title == "播放&下载" || title == "播放" || title == "下载" {
                return None;
            }
            
            // 尝试从标题中提取歌手
            let mut artist = "未知歌手".to_string();
            let mut clean_title = title.clone();
            if title.contains(" - ") {
                let parts: Vec<&str> = title.splitn(2, " - ").collect();
                if parts.len() == 2 {
                    clean_title = parts[0].trim().to_string();
                    artist = parts[1].trim().to_string();
                }
            }
            
            Some(MusicItem {
                id: id.to_string(),
                title: clean_title,
                artist,
                album: None,
                cover: None,
                duration: None,
                provider: "gequbao".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

async fn search_gequhai(client: &Client, query: &str) -> Result<SearchResponse, String> {
    // 访问搜索页面
    let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://www.gequhai.com/s/{}", encoded);
    let response = client.get(&url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 解析HTML，查找表格中的歌曲
    let document = Html::parse_document(&html);
    let tr_selector = Selector::parse("table#myTables tbody tr").map_err(|e| format!("Parse selector failed: {}", e))?;
    
    let items: Vec<MusicItem> = document.select(&tr_selector)
        .filter_map(|tr| {
            let tds: Vec<_> = tr.select(&Selector::parse("td").ok()?).collect();
            if tds.len() < 3 {
                return None;
            }
            
            let title_cell = &tds[1];
            let artist_cell = &tds[2];
            
            let link = title_cell.select(&Selector::parse("a").ok()?).next()?;
            let title = link.text().collect::<Vec<_>>().join("").trim().to_string();
            let href = link.value().attr("href")?;
            let artist = artist_cell.text().collect::<Vec<_>>().join("").trim().to_string();
            
            let re = Regex::new(r"/play/(\d+)").ok()?;
            let captures = re.captures(href)?;
            let id = captures.get(1)?.as_str();
            
            Some(MusicItem {
                id: id.to_string(),
                title,
                artist: if artist.is_empty() { "未知歌手".to_string() } else { artist },
                album: None,
                cover: None,
                duration: None,
                provider: "gequhai".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
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

async fn search_qqmp3(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let response = client.get("https://api.qqmp3.vip/api/songs.php")
        .query(&[("type", "search"), ("keyword", query)])
        .header("accept", "*/*")
        .header("origin", "https://www.qqmp3.vip")
        .header("referer", "https://www.qqmp3.vip/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let text = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 调试输出
    eprintln!("QQMP3 response: {}", &text[..text.len().min(200)]);

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    // 检查不同的响应格式
    let list = if let Some(data) = json.get("data") {
        if data.is_array() {
            data.as_array().unwrap()
        } else if let Some(items) = data.get("data") {
            // {"data": {"data": [...]}}
            items.as_array().ok_or("Invalid data format")?
        } else {
            return Ok(SearchResponse { items: vec![] });
        }
    } else {
        return Ok(SearchResponse { items: vec![] });
    };

    let items: Vec<MusicItem> = list.iter()
        .filter_map(|item| {
            let id = item.get("rid")?.as_str()?.to_string();
            let title = item.get("name")?.as_str()?.to_string();
            let artist = item.get("artist")?.as_str().unwrap_or("未知歌手").to_string();
            let cover = item.get("pic").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            Some(MusicItem {
                id,
                title,
                artist,
                album: None,
                cover,
                duration: None,
                provider: "qqmp3".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

async fn search_migu(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let search_switch = r#"{"song": 1, "album": 0, "singer": 0, "tagSong": 1, "mvSong": 0, "bestShow": 1}"#;
    let encoded_query = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let encoded_switch = utf8_percent_encode(search_switch, NON_ALPHANUMERIC).to_string();
    let params = format!(
        "text={}&pageNo=1&pageSize=20&isCopyright=1&sort=1&searchSwitch={}",
        encoded_query,
        encoded_switch
    );
    
    let url = format!("https://c.musicapp.migu.cn/v1.0/content/search_all.do?{}", params);
    
    let response = client.get(&url)
        .header("accept", "application/json, text/plain, */*")
        .header("activityid", "v4_zt_2022_music")
        .header("appid", "ce")
        .header("channel", "014X031")
        .header("origin", "https://y.migu.cn")
        .header("referer", "https://y.migu.cn/app/v4/zt/2022/music/index.html")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    let list = json.get("songResultData")
        .and_then(|d| d.get("result"))
        .and_then(|r| r.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = list.iter()
        .filter_map(|item| {
            let content_id = item.get("contentId")?.as_str()?;
            let copyright_id = item.get("copyrightId")?.as_str()?;
            let id = format!("{}_{}", content_id, copyright_id);
            
            let title = item.get("name")?.as_str()?.to_string();
            
            let artists = item.get("singers")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "未知歌手".to_string());
            
            let albums = item.get("albums")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|s| !s.is_empty());
            
            let cover = item.get("imgItems")
                .and_then(|items| items.as_array())
                .and_then(|arr| arr.last())
                .and_then(|img| img.get("img"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            
            Some(MusicItem {
                id,
                title,
                artist: if artists.is_empty() { "未知歌手".to_string() } else { artists },
                album: albums,
                cover,
                duration: None,
                provider: "migu".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

async fn search_livepoo(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = format!(
        "https://www.livepoo.cn/search?keyword={}&page=0",
        encoded
    );
    
    let response = client.get(&url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 解析HTML
    let document = Html::parse_document(&html);
    let link_selector = Selector::parse("ul.tuij_song li.song_item2 a[href]").map_err(|e| format!("Parse selector failed: {}", e))?;
    
    let items: Vec<MusicItem> = document.select(&link_selector)
        .filter_map(|el| {
            let href = el.value().attr("href")?;
            
            // 解析URL获取ID
            let url_obj = url::Url::parse(&format!("https://www.livepoo.cn{}", href)).ok()?;
            let id_param = url_obj.query_pairs().find(|(k, _)| k == "id")?.1;
            let id = id_param.replace("MUSIC_", "");
            
            // 获取li元素（需要重新解析HTML查找li）
            // 由于scraper的限制，我们直接从el向上查找不太方便
            // 改为使用el的文本，然后尝试查找歌手链接
            
            // 尝试从链接文本获取
            let title_text = el.text().collect::<Vec<_>>().join("");
            
            // 清理文本：移除多余空白和干扰词
            let normalized = title_text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
                .replace("播放", "")
                .replace("试听", "")
                .replace("下载", "")
                .replace("分享", "")
                .trim()
                .to_string();
            
            // 解析标题和歌手
            let (title, artist) = if let Some(captures) = Regex::new(r"^\s*(.*?)《(.*?)》\s*$").ok()?.captures(&normalized) {
                (
                    captures.get(2)?.as_str().trim().to_string(),
                    captures.get(1)?.as_str().trim().to_string(),
                )
            } else if normalized.contains(" - ") {
                let parts: Vec<&str> = normalized.splitn(2, " - ").collect();
                if parts.len() == 2 && !parts[0].trim().is_empty() && !parts[1].trim().is_empty() {
                    (parts[0].trim().to_string(), parts[1].trim().to_string())
                } else if normalized.contains('-') {
                    let parts: Vec<&str> = normalized.splitn(2, '-').collect();
                    if parts.len() == 2 && !parts[0].trim().is_empty() && !parts[1].trim().is_empty() {
                        (parts[0].trim().to_string(), parts[1].trim().to_string())
                    } else {
                        (normalized, "未知歌手".to_string())
                    }
                } else {
                    (normalized, "未知歌手".to_string())
                }
            } else {
                (normalized, "未知歌手".to_string())
            };
            
            if id.is_empty() || title.is_empty() {
                return None;
            }
            
            Some(MusicItem {
                id,
                title,
                artist: if artist.is_empty() { "未知歌手".to_string() } else { artist },
                album: None,
                cover: None,
                duration: None,
                provider: "livepoo".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

#[command]
pub async fn get_music_url(id: String, provider: String) -> Result<UrlResponse, String> {
    // 根据provider获取播放URL
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let url = match provider.as_str() {
        "gequbao" => get_gequbao_play_url(&client, &id).await?,
        "gequhai" => get_gequhai_play_url(&client, &id).await?,
        "bugu" => get_bugu_play_url(&client, &id).await?,
        "qq" => get_qq_play_url(&client, &id).await?,
        "qqmp3" => get_qqmp3_play_url(&client, &id).await?,
        "migu" => get_migu_play_url(&client, &id).await?,
        "livepoo" => get_livepoo_play_url(&client, &id).await?,
        _ => {
            // jianbin系列：id就是URL
            let decoded = percent_encoding::percent_decode_str(&id)
                .decode_utf8()
                .map_err(|e| format!("Decode failed: {}", e))?;
            decoded.to_string()
        }
    };

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

// 获取gequbao播放URL
async fn get_gequbao_play_url(client: &Client, id: &str) -> Result<String, String> {
    // 访问音乐页面获取play_id
    let page_url = format!("https://www.gequbao.com/music/{}", id);
    let response = client.get(&page_url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("referer", "https://www.gequbao.com/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 使用正则提取play_id
    let re = Regex::new(r#"window\.appData\s*=\s*JSON\.parse\("([^"]+)"\)"#).map_err(|e| format!("Regex failed: {}", e))?;
    if let Some(captures) = re.captures(&html) {
        if let Some(json_str) = captures.get(1) {
            // 解码JSON字符串
            let decoded = json_str.as_str().replace("\\\"", "\"");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                if let Some(play_id) = json.get("play_id").and_then(|v| v.as_str()) {
                    // 调用API获取播放URL
                    let api_response = client.post("https://www.gequbao.com/api/play-url")
                        .form(&[("id", play_id)])
                        .header("accept", "application/json, text/javascript, */*; q=0.01")
                        .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
                        .header("origin", "https://www.gequbao.com")
                        .header("referer", &page_url)
                        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                        .send()
                        .await
                        .map_err(|e| format!("API request failed: {}", e))?;

                    let api_json: serde_json::Value = api_response.json()
                        .await
                        .map_err(|e| format!("Parse API response failed: {}", e))?;

                    if api_json.get("code").and_then(|v| v.as_i64()) == Some(1) {
                        if let Some(url) = api_json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                            return Ok(url.to_string());
                        }
                    }
                }
            }
        }
    }

    Err("Failed to get play URL".to_string())
}

// 获取gequhai播放URL
async fn get_gequhai_play_url(client: &Client, id: &str) -> Result<String, String> {
    let play_url = format!("https://www.gequhai.com/play/{}", id);
    let response = client.get(&play_url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("referer", "https://www.gequhai.com/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 提取play_id - 尝试多种格式
    let patterns = [
        r#"window\.appData\s*=\s*\{[^}]*"play_id"\s*:\s*"([^"]+)""#,
        r#""play_id"\s*:\s*"([^"]+)""#,
        r#"play_id["']\s*:\s*["']([^"']+)["']"#,
    ];
    
    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(&html) {
                if let Some(play_id) = captures.get(1) {
                    let api_response = client.post("https://www.gequhai.com/api/music")
                        .form(&[("id", play_id.as_str()), ("type", "0")])
                        .header("accept", "application/json, text/javascript, */*; q=0.01")
                        .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
                        .header("origin", "https://www.gequhai.com")
                        .header("referer", &play_url)
                        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                        .send()
                        .await
                        .map_err(|e| format!("API request failed: {}", e))?;

                    let api_json: serde_json::Value = api_response.json()
                        .await
                        .map_err(|e| format!("Parse API response failed: {}", e))?;

                    if let Some(url) = api_json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                        return Ok(url.to_string());
                    }
                }
            }
        }
    }

    Err("Failed to get play URL".to_string())
}

// 获取bugu播放URL
async fn get_bugu_play_url(client: &Client, id: &str) -> Result<String, String> {
    let response = client.get("https://a.buguyy.top/newapi/url.php")
        .query(&[("id", id)])
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

    if let Some(url) = json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
        Ok(url.to_string())
    } else {
        Err("Failed to get play URL".to_string())
    }
}

// 获取qq播放URL
async fn get_qq_play_url(_client: &Client, _id: &str) -> Result<String, String> {
    // QQ音乐需要特殊处理，这里返回示例URL
    Err("QQ music playback not supported directly".to_string())
}

// 获取qqmp3播放URL
async fn get_qqmp3_play_url(client: &Client, id: &str) -> Result<String, String> {
    let response = client.get("https://api.qqmp3.vip/api/kw.php")
        .query(&[("rid", id), ("type", "json"), ("level", "exhigh")])
        .header("accept", "*/*")
        .header("origin", "https://www.qqmp3.vip")
        .header("referer", "https://www.qqmp3.vip/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    if json.get("code").and_then(|v| v.as_i64()) == Some(200) {
        if let Some(url) = json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
            return Ok(url.to_string());
        }
    }

    Err("Failed to get play URL".to_string())
}

// 获取migu播放URL
async fn get_migu_play_url(client: &Client, id: &str) -> Result<String, String> {
    let parts: Vec<&str> = id.split('_').collect();
    if parts.len() != 2 {
        return Err("Invalid migu ID".to_string());
    }
    let content_id = parts[0];
    let copyright_id = parts[1];

    // 获取歌曲详情
    let search_switch = r#"{"song": 1, "album": 0, "singer": 0, "tagSong": 1, "mvSong": 0, "bestShow": 1}"#;
    let encoded_switch = utf8_percent_encode(search_switch, NON_ALPHANUMERIC);
    let params = format!(
        "text={}&pageNo=1&pageSize=1&isCopyright=1&sort=1&searchSwitch={}",
        content_id,
        encoded_switch
    );
    
    let response = client.get(&format!("https://c.musicapp.migu.cn/v1.0/content/search_all.do?{}", params))
        .header("accept", "application/json, text/plain, */*")
        .header("accept-encoding", "gzip, deflate, br, zstd")
        .header("accept-language", "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("activityid", "v4_zt_2022_music")
        .header("appid", "ce")
        .header("channel", "014X031")
        .header("connection", "keep-alive")
        .header("origin", "https://y.migu.cn")
        .header("referer", "https://y.migu.cn/app/v4/zt/2022/music/index.html")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    if let Some(song) = json.get("songResultData")
        .and_then(|d| d.get("result"))
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first()) {
        
        // 尝试newRateFormats，然后是rateFormats
        let rate_formats = song.get("newRateFormats")
            .and_then(|v| v.as_array())
            .or_else(|| song.get("rateFormats").and_then(|v| v.as_array()));
        
        if let Some(rate_formats) = rate_formats {
            // 按质量排序，选择最好的
            for rate in rate_formats {
                if let (Some(format_type), Some(resource_type)) = (
                    rate.get("formatType").and_then(|v| v.as_str()),
                    rate.get("resourceType").and_then(|v| v.as_str())
                ) {
                    let listen_url = format!(
                        "https://c.musicapp.migu.cn/MIGUM3.0/strategy/listen-url/v2.4?resourceType={}&netType=01&scene=&toneFlag={}&contentId={}&copyrightId={}&lowerQualityContentId={}",
                        resource_type, format_type, content_id, copyright_id, content_id
                    );
                    
                    let listen_response = client.get(&listen_url)
                        .header("accept", "application/json, text/plain, */*")
                        .header("activityid", "v4_zt_2022_music")
                        .header("appid", "ce")
                        .header("channel", "014X031")
                        .header("origin", "https://y.migu.cn")
                        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                        .send()
                        .await
                        .map_err(|e| format!("Listen request failed: {}", e))?;

                    let listen_json: serde_json::Value = listen_response.json()
                        .await
                        .map_err(|e| format!("Parse listen response failed: {}", e))?;

                    if let Some(url) = listen_json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                        return Ok(url.to_string());
                    }
                    
                    // 如果data.url不存在，尝试data.androidUrl或iosUrl
                    if let Some(url) = listen_json.get("data")
                        .and_then(|d| d.get("androidUrl").or_else(|| d.get("iosUrl")))
                        .and_then(|v| v.as_str()) {
                        return Ok(url.to_string());
                    }
                }
            }
        }
    }

    Err("Failed to get play URL".to_string())
}

// 获取livepoo播放URL
async fn get_livepoo_play_url(client: &Client, id: &str) -> Result<String, String> {
    let play_url = format!("https://www.livepoo.cn/audio/play?id={}", id);
    let response = client.get(&play_url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let url = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    let trimmed = url.trim();
    if trimmed.starts_with("http") {
        Ok(trimmed.to_string())
    } else {
        Err("Invalid play URL".to_string())
    }
}
