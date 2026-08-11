use tauri::command;
use reqwest::Client;
use serde_json;
use crate::api_types::{MusicItem, SearchResponse, UrlResponse, LyricsResponse};
use std::collections::HashMap;
use scraper::{Html, Selector};
use regex::Regex;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use futures_util::StreamExt;
use tauri::Emitter;

const VKEYS_BASE: &str = "https://api.vkeys.cn";

// 按字节长度安全截断字符串，避免在多字节字符中间切片导致 panic
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
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
        // gequbao.com 已被 Cloudflare 拦截，复用可用的 gequhai 镜像实现
        search_gequhai(&client, &query, "gequbao").await
    } else if provider == "gequhai" {
        search_gequhai(&client, &query, "gequhai").await
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
        // 默认使用 qqmp3（布谷源已失效）
        search_qqmp3(&client, &query).await
    }
}

// jbsou.cn 服务端已失效（所有平台搜索均返回404），煎饼系源改为对应平台官方接口直连
async fn search_jianbin(client: &Client, query: &str, provider: &str) -> Result<SearchResponse, String> {
    eprintln!("[Jianbin] Searching: query='{}', provider='{}'", query, provider);

    let mut resp = match provider {
        "jianbin-kugou" => search_jianbin_kugou(client, query).await?,
        "jianbin-netease" => search_jianbin_netease(client, query).await?,
        "jianbin-kuwo" => search_jianbin_kuwo(client, query).await?,
        // 煎饼-QQ 复用 QQ音乐(vkeys) 搜索实现
        _ => search_qq(client, query).await?,
    };
    for item in resp.items.iter_mut() {
        item.provider = provider.to_string();
    }
    eprintln!("[Jianbin] Returning {} items", resp.items.len());
    Ok(resp)
}

// 去掉接口返回文本中的 HTML 标签（如搜索关键词高亮 <em>）
fn strip_html_tags(s: &str) -> String {
    Regex::new(r"<[^>]+>")
        .map(|re| re.replace_all(s, "").to_string())
        .unwrap_or_else(|_| s.to_string())
}

// 煎饼-酷狗：酷狗移动端搜索接口
async fn search_jianbin_kugou(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let response = client.get("http://mobilecdn.kugou.com/api/v3/search/song")
        .query(&[("keyword", query), ("page", "1"), ("pagesize", "30"), ("showtype", "0")])
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    let list = json.get("data")
        .and_then(|d| d.get("info"))
        .and_then(|v| v.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = list.iter()
        .filter_map(|item| {
            let hash = item.get("hash")?.as_str()?;
            let title = strip_html_tags(item.get("songname").and_then(|v| v.as_str()).unwrap_or("未知歌曲"));
            let artist = strip_html_tags(item.get("singername").and_then(|v| v.as_str()).unwrap_or("未知歌手"));
            let album = item.get("album_name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let duration = item.get("duration")
                .and_then(|v| v.as_i64())
                .map(|secs| format!("{}:{:02}", secs / 60, secs % 60));
            let size = item.get("filesize")
                .and_then(|v| v.as_f64())
                .map(|b| format!("{:.1}MB", b / 1024.0 / 1024.0));

            Some(MusicItem {
                id: hash.to_string(),
                title,
                artist: if artist.is_empty() { "未知歌手".to_string() } else { artist },
                album,
                cover: None,
                duration,
                size,
                provider: "jianbin-kugou".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

// 煎饼-网易：网易云官方搜索接口
async fn search_jianbin_netease(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let mut params = HashMap::new();
    params.insert("s", query);
    params.insert("type", "1");
    params.insert("limit", "30");
    params.insert("offset", "0");

    let response = client.post("https://music.163.com/api/search/get")
        .form(&params)
        .header("referer", "https://music.163.com/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    let songs = json.get("result")
        .and_then(|r| r.get("songs"))
        .and_then(|v| v.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = songs.iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_i64()?.to_string();
            let title = item.get("name").and_then(|v| v.as_str()).unwrap_or("未知歌曲").to_string();
            let artist = item.get("artists")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join("/"))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "未知歌手".to_string());
            let album = item.get("album")
                .and_then(|a| a.get("name"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let duration = item.get("duration")
                .and_then(|v| v.as_i64())
                .map(|ms| {
                    let secs = ms / 1000;
                    format!("{}:{:02}", secs / 60, secs % 60)
                });

            Some(MusicItem {
                id,
                title,
                artist,
                album,
                cover: None,
                duration,
                size: None,
                provider: "jianbin-netease".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

// 煎饼-酷我：酷我旧版搜索接口
async fn search_jianbin_kuwo(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let response = client.get("http://search.kuwo.cn/r.s")
        .query(&[("all", query), ("ft", "music"), ("pn", "0"), ("rn", "30"),
                 ("encoding", "utf8"), ("rformat", "json"), ("mobi", "1")])
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let text = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    let list = json.get("abslist")
        .and_then(|v| v.as_array())
        .ok_or("Invalid response format")?;

    let items: Vec<MusicItem> = list.iter()
        .filter_map(|item| {
            let rid = item.get("MUSICRID")?.as_str()?;
            let id = rid.trim_start_matches("MUSIC_").to_string();
            let title = item.get("SONGNAME").and_then(|v| v.as_str()).unwrap_or("未知歌曲").to_string();
            let artist = item.get("ARTIST")
                .and_then(|v| v.as_str())
                .map(|s| s.replace('&', "/"))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "未知歌手".to_string());
            let duration = item.get("DURATION")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .map(|secs| format!("{}:{:02}", secs / 60, secs % 60));

            Some(MusicItem {
                id,
                title,
                artist,
                album: None,
                cover: None,
                duration,
                size: None,
                provider: "jianbin-kuwo".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

// 获取gequbao搜索（原站已被Cloudflare拦截，保留实现仅作备用）
#[allow(dead_code)]
async fn search_gequbao(client: &Client, query: &str) -> Result<SearchResponse, String> {
    // 访问搜索页面
    let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://www.gequbao.com/s/{}", encoded);
    eprintln!("[Gequbao] Searching: {}", url);
    
    let response = client.get(&url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("cache-control", "no-cache")
        .header("pragma", "no-cache")
        .header("priority", "u=0, i")
        .header("referer", "https://www.gequbao.com/")
        .header("sec-ch-ua", "\"Google Chrome\";v=\"143\", \"Chromium\";v=\"143\", \"Not A(Brand\";v=\"24\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "document")
        .header("sec-fetch-mode", "navigate")
        .header("sec-fetch-site", "same-origin")
        .header("upgrade-insecure-requests", "1")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;
    
    eprintln!("[Gequbao] Response HTML length: {}", html.len());
    
    // 保存HTML到文件以便分析
    use std::fs;
    let _ = fs::write("gequbao_debug.html", &html);
    eprintln!("[Gequbao] HTML saved to gequbao_debug.html");

    // 解析HTML - 尝试多种选择器
    let document = Html::parse_document(&html);
    
    // 尝试选择器1: a[href^="/music/"]
    let link_selector = Selector::parse("a[href^=\"/music/\"]").map_err(|e| format!("Parse selector failed: {}", e))?;
    let mut items: Vec<MusicItem> = document.select(&link_selector)
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
            
            eprintln!("[Gequbao] Found: id={}, title={}, artist={}", id, clean_title, artist);
            
            Some(MusicItem {
                id: id.to_string(),
                title: clean_title,
                artist,
                album: None,
                cover: None,
                duration: None, // gequbao搜索页面不提供时长信息
                size: None,
                provider: "gequbao".to_string(),
            })
        })
        .collect();

    // 如果没找到，尝试其他选择器
    if items.is_empty() {
        eprintln!("[Gequbao] Trying alternative selectors...");
        
        // 选择器2: div.song-list a
        if let Ok(selector) = Selector::parse("div.song-list a") {
            items = document.select(&selector)
                .filter_map(|el| {
                    let href = el.value().attr("href")?;
                    let re = Regex::new(r"/music/(\d+)").ok()?;
                    let captures = re.captures(href)?;
                    let id = captures.get(1)?.as_str();
                    let title = el.text().collect::<Vec<_>>().join("").trim().to_string();
                    if title.is_empty() { return None; }
                    Some(MusicItem {
                        id: id.to_string(),
                        title,
                        artist: "未知歌手".to_string(),
                        album: None,
                        cover: None,
                        duration: None, // gequbao搜索页面不提供时长信息
                        size: None,
                        provider: "gequbao".to_string(),
                    })
                })
                .collect();
        }
    }

    eprintln!("[Gequbao] Total items found: {}", items.len());
    Ok(SearchResponse { items })
}

async fn search_gequhai(client: &Client, query: &str, provider_name: &str) -> Result<SearchResponse, String> {
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
                duration: None, // gequhai搜索页面不提供时长信息
                size: None,
                provider: provider_name.to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

// 布谷原域名 buguyy.top/.cc 均已失联，改用同框架在线站点 liziyy.top
const BUGU_BASE: &str = "https://liziyy.top";

async fn search_bugu(client: &Client, query: &str) -> Result<SearchResponse, String> {
    let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = format!("{}/search?keyword={}&page=0", BUGU_BASE, encoded);

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

    // 瀑布流卡片结构：div.music-card > a[href="/music/info.html?id=MUSIC_xxx"]
    let document = Html::parse_document(&html);
    let card_selector = Selector::parse("div.music-card a[href]").map_err(|e| format!("Parse selector failed: {}", e))?;
    let name_selector = Selector::parse("div.music-name").map_err(|e| format!("Parse selector failed: {}", e))?;
    let singer_selector = Selector::parse("div.music-singer").map_err(|e| format!("Parse selector failed: {}", e))?;
    let cover_selector = Selector::parse("img.music-cover").map_err(|e| format!("Parse selector failed: {}", e))?;

    let items: Vec<MusicItem> = document.select(&card_selector)
        .filter_map(|el| {
            let href = el.value().attr("href")?;
            // 提取 MUSIC_xxx 中的数字ID
            let id = href.split("id=MUSIC_").nth(1)?.split('&').next()?.to_string();

            let title = el.select(&name_selector).next()?
                .text().collect::<Vec<_>>().join("").trim().to_string();
            let artist = el.select(&singer_selector).next()
                .map(|s| s.text().collect::<Vec<_>>().join("").trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "未知歌手".to_string());
            let cover = el.select(&cover_selector).next()
                .and_then(|s| s.value().attr("src"))
                .filter(|s| s.starts_with("http"))
                .map(|s| s.to_string());

            if id.is_empty() || title.is_empty() {
                return None;
            }

            Some(MusicItem {
                id,
                title,
                artist,
                album: None,
                cover,
                duration: None,
                size: None,
                provider: "bugu".to_string(),
            })
        })
        .collect();

    Ok(SearchResponse { items })
}

async fn search_qq(client: &Client, query: &str) -> Result<SearchResponse, String> {
    // 尝试3次，处理连接关闭问题
    for attempt in 1..=3 {
        let response = client.get(&format!("{}/v2/music/tencent/search/song", VKEYS_BASE))
            .query(&[("word", query)])
            .header("accept", "application/json, text/plain, */*")
            .header("origin", "https://y.qq.com")
            .header("referer", "https://y.qq.com/")
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await;
        
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[QQ] Search attempt {} failed: {}", attempt, e);
                if attempt == 3 {
                    return Err(format!("Request failed: {}", e));
                }
                // 等待一段时间后重试
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
        };

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
                let duration = item.get("duration").and_then(|v| v.as_str()).map(|s| s.to_string());
                let size = item.get("size").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                Some(MusicItem {
                    id: id.to_string(),
                    title: title.to_string(),
                    artist: artist.to_string(),
                    album,
                    cover,
                    duration,
                    size,
                    provider: "qq".to_string(),
                })
            })
            .collect();

        return Ok(SearchResponse { items });
    }
    
    unreachable!()
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
    eprintln!("QQMP3 response: {}", safe_truncate(&text, 200));

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
            let duration = item.get("duration").and_then(|v| v.as_str()).map(|s| s.to_string());
            let size = item.get("size").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            Some(MusicItem {
                id,
                title,
                artist,
                album: None,
                cover,
                duration,
                size,
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
            
            // 提取文件大小信息（从 newRateFormats 或 rateFormats 中获取最大的 size）
            let size = item.get("newRateFormats")
                .or_else(|| item.get("rateFormats"))
                .and_then(|rates| rates.as_array())
                .map(|rates| {
                    // 找到最大的文件大小
                    let mut max_size: Option<String> = None;
                    let mut max_size_mb = 0.0f64;
                    
                    for rate in rates {
                        if let Some(size_val) = rate.get("size").and_then(|s| s.as_str()) {
                            let size_str = size_val.trim();
                            
                            // 判断是否已经有 MB 单位
                            if size_str.to_lowercase().contains("mb") {
                                // 已经有 MB 单位，直接使用
                                max_size = Some(size_str.to_string());
                                // 提取数值用于比较
                                let num_str = size_str.replace("MB", "").replace("mb", "").trim().to_string();
                                if let Ok(num) = num_str.parse::<f64>() {
                                    if num > max_size_mb {
                                        max_size_mb = num;
                                    }
                                }
                            } else {
                                // 没有单位，可能是字节
                                if let Ok(bytes) = size_str.parse::<f64>() {
                                    if bytes > 0.0 {
                                        // 如果数字很大（>1000000），认为是字节
                                        if bytes > 1000000.0 {
                                            let mb = bytes / (1024.0 * 1024.0);
                                            if mb > max_size_mb {
                                                max_size_mb = mb;
                                                max_size = Some(format!("{:.2}MB", mb));
                                            }
                                        } else {
                                            // 否则可能就是 MB 数值
                                            if bytes > max_size_mb {
                                                max_size_mb = bytes;
                                                max_size = Some(format!("{}MB", bytes));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    max_size
                })
                .flatten();
            
            // 提取时长信息
            let duration = item.get("length").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            Some(MusicItem {
                id,
                title,
                artist: if artists.is_empty() { "未知歌手".to_string() } else { artists },
                album: albums,
                cover,
                duration,
                size,
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
                duration: None, // livepoo搜索页面不提供时长信息
                size: None,
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
        // gequbao.com 已被 Cloudflare 拦截，搜索/播放均复用 gequhai 镜像实现
        "gequbao" => get_gequhai_play_url(&client, &id).await?,
        "gequhai" => get_gequhai_play_url(&client, &id).await?,
        "bugu" => get_bugu_play_url(&client, &id).await?,
        "qq" => get_qq_play_url(&client, &id).await?,
        "qqmp3" => get_qqmp3_play_url(&client, &id).await?,
        "migu" => get_migu_play_url(&client, &id).await?,
        "livepoo" => get_livepoo_play_url(&client, &id).await?,
        // 煎饼系列：按子源路由到对应平台播放接口
        "jianbin-qq" => get_qq_play_url(&client, &id).await?,
        "jianbin-netease" => get_netease_play_url(&id)?,
        "jianbin-kugou" => get_kugou_play_url(&client, &id).await?,
        "jianbin-kuwo" => get_kuwo_play_url(&client, &id).await?,
        _ => {
            // 其他源：id就是URL
            let decoded = percent_encoding::percent_decode_str(&id)
                .decode_utf8()
                .map_err(|e| format!("Decode failed: {}", e))?;
            decoded.to_string()
        }
    };

    // 检查URL是否包含DOWNLOAD_ONLY:前缀
    let download_only = url.starts_with("DOWNLOAD_ONLY:");
    
    Ok(UrlResponse { url, download_only: Some(download_only) })
}

#[command]
pub async fn download_music(id: String, _filename: String, provider: String) -> Result<Vec<u8>, String> {
    // 获取音乐 URL
    let url_response = get_music_url(id.clone(), provider.clone()).await?;
    let mut url = url_response.url;
    
    // 移除 DOWNLOAD_ONLY: 前缀（如果存在）
    if url.starts_with("DOWNLOAD_ONLY:") {
        url = url.replace("DOWNLOAD_ONLY:", "");
    }
    
    // 检查是否为网盘链接（仅支持下载）
    if url.contains("pan.quark.cn") || url.contains("pan.baidu.com") {
        // 对于网盘链接，在浏览器中打开，让用户在网盘中下载
        eprintln!("[Download] Cloud drive URL detected, opening in browser: {}", url);
        
        // 使用 Tauri shell 打开链接
        // 注意：这里无法直接返回文件内容，需要前端处理
        return Err(format!("CLOUD_DRIVE:{}", url));
    }
    
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

// 获取gequbao播放URL（原站已被Cloudflare拦截，保留实现仅作备用）
#[allow(dead_code)]
async fn get_gequbao_play_url(client: &Client, id: &str) -> Result<String, String> {
    // 访问音乐页面获取play_id
    let page_url = format!("https://www.gequbao.com/music/{}", id);
    let response = client.get(&page_url)
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

    // 尝试多种格式提取appData
    let patterns = [
        // JSON.parse格式
        r#"window\.appData\s*=\s*JSON\.parse\((['"])([\s\S]*?)\1\)"#,
        // 直接对象格式
        r#"window\.appData\s*=\s*(\{[\s\S]*?\})\s*;"#,
        // play_id的直接匹配
        r#""play_id"\s*:\s*"([^"]+)""#,
    ];
    
    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(&html) {
                let play_id = if captures.len() > 2 {
                    // JSON.parse格式，需要解码字符串
                    if let Some(json_str_match) = captures.get(2) {
                        let json_str = json_str_match.as_str();
                        // 尝试解码JS字符串
                        let decoded = json_str.replace("\\\"", "\"")
                            .replace("\\\\", "\\")
                            .replace("\\n", "\n")
                            .replace("\\t", "\t");
                        
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                            json.get("play_id").and_then(|v| v.as_str()).map(|s| s.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else if captures.len() == 2 {
                    // 直接对象或直接匹配
                    if let Some(matched_capture) = captures.get(1) {
                        let matched = matched_capture.as_str();
                        if matched.starts_with('{') {
                            // JSON对象格式
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(matched) {
                                json.get("play_id").and_then(|v| v.as_str()).map(|s| s.to_string())
                            } else {
                                None
                            }
                        } else {
                            // 直接play_id
                            Some(matched.to_string())
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                if let Some(play_id) = play_id {
                    // 调用API获取播放URL
                    let api_response = client.post("https://www.gequbao.com/api/play-url")
                        .form(&[("id", play_id.as_str())])
                        .header("accept", "application/json, text/javascript, */*; q=0.01")
                        .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
                        .header("origin", "https://www.gequbao.com")
                        .header("referer", &page_url)
                        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                        .send()
                        .await
                        .map_err(|e| format!("API request failed: {}", e))?;

                    let text = api_response.text()
                        .await
                        .map_err(|e| format!("Read API response failed: {}", e))?;
                    
                    eprintln!("Gequbao API response: {}", safe_truncate(&text, 200));

                    let api_json: serde_json::Value = serde_json::from_str(&text)
                        .map_err(|e| format!("Parse API response failed: {}", e))?;

                    if api_json.get("code").and_then(|v| v.as_i64()) == Some(1) {
                        if let Some(url) = api_json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                            return Ok(url.to_string());
                        }
                    } else if let Some(msg) = api_json.get("msg").and_then(|v| v.as_str()) {
                        eprintln!("Gequbao API error: {}", msg);
                    }
                    
                    break;
                }
            }
        }
    }

    Err(format!("Failed to get gequbao play URL - play_id not found in HTML (length: {})", html.len()))
}

// 获取gequhai播放URL
async fn get_gequhai_play_url(client: &Client, id: &str) -> Result<String, String> {
    eprintln!("[Gequhai] Getting play URL for id: {}", id);
    
    let play_url = format!("https://www.gequhai.com/play/{}", id);
    let response = client.get(&play_url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("referer", "https://www.gequhai.com/")
        .header("upgrade-insecure-requests", "1")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;
    
    eprintln!("[Gequhai] Play page HTML length: {}", html.len());

    // 提取appData - 使用前端的相同逻辑
    let mut app_data: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    
    // 尝试提取 window.appData = {...};
    if let Ok(re) = Regex::new(r#"window\.appData\s*=\s*(\{.*?\})\s*;"#) {
        if let Some(captures) = re.captures(&html) {
            if let Some(json_str) = captures.get(1) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                    if let Some(obj) = json.as_object() {
                        app_data.extend(obj.clone());
                    }
                }
            }
        }
    }
    
    // 提取 play_id 或 mp3_id
    let play_id = app_data.get("play_id")
        .or_else(|| app_data.get("mp3_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.to_string());
    
    eprintln!("[Gequhai] Extracted play_id: {}", play_id);
        
    let api_response = client.post("https://www.gequhai.com/api/music")
        .body(format!("id={}&type=0", play_id))
        .header("accept", "application/json, text/javascript, */*; q=0.01")
        .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("origin", "https://www.gequhai.com")
        .header("referer", &play_url)
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("x-requested-with", "XMLHttpRequest")
        .header("x-custom-header", "SecretKey")
        .header("sec-ch-ua", "\"Google Chrome\";v=\"143\", \"Chromium\";v=\"143\", \"Not A(Brand\";v=\"24\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;
    
    let text = api_response.text()
        .await
        .map_err(|e| format!("Read API response failed: {}", e))?;
        
    eprintln!("[Gequhai] API response length: {}", text.len());
    eprintln!("[Gequhai] API response: {}", safe_truncate(&text, 500));
    
    let api_json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| {
            eprintln!("[Gequhai] JSON parse error: {}", e);
            format!("Parse API response failed: {}", e)
        })?;
    
    eprintln!("[Gequhai] Parsed JSON successfully");
    
    // 尝试提取 data.url
    if let Some(url) = api_json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
        if !url.is_empty() {
            eprintln!("[Gequhai] Found URL in data.url: {}", url);
            return Ok(url.to_string());
        }
    }
    
    // Fallback: 提取 mp3_extra_url 并解码
    eprintln!("[Gequhai] No valid URL in data, trying mp3_extra_url...");
    
    // 重新解析HTML提取mp3_extra_url
    if let Ok(re) = Regex::new(r#"window\.mp3_extra_url\s*=\s*['\"]([^'\"]+)['\"]"#) {
        if let Some(captures) = re.captures(&html) {
            if let Some(extra_url) = captures.get(1) {
                let encoded_url = extra_url.as_str();
                eprintln!("[Gequhai] Found mp3_extra_url (encoded): {}", safe_truncate(encoded_url, 200));
                
                // Base64解码 (先将#替换为H，然后Base64解码)
                let b64_str = encoded_url.replace('#', "H");
                eprintln!("[Gequhai] Base64 string: {}", safe_truncate(&b64_str, 200));
                
                use base64::{Engine as _, engine::general_purpose::STANDARD};
                if let Ok(decoded_bytes) = STANDARD.decode(&b64_str) {
                    if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                        eprintln!("[Gequhai] Decoded URL: {}", safe_truncate(&decoded_str, 200));
                        
                        // 检查是否为网盘链接（仅支持下载）
                        if decoded_str.contains("pan.quark.cn") || decoded_str.contains("pan.baidu.com") {
                            eprintln!("[Gequhai] Detected cloud drive URL, adding DOWNLOAD_ONLY marker");
                            // 添加特殊前缀标记，前端检测到后会提示用户
                            let marked_url = format!("DOWNLOAD_ONLY:{}", decoded_str);
                            eprintln!("[Gequhai] Marked URL: {}", safe_truncate(&marked_url, 200));
                            return Ok(marked_url);
                        }
                        
                        return Ok(decoded_str);
                    }
                }
            }
        }
    }
    
    eprintln!("[Gequhai] No url found in API response or extra_url");
    Err(format!("Failed to get gequhai play URL - no url in API response (play_id: {})", play_id))
}

// 获取bugu播放URL（liziyy.top 详情页内嵌 music_mp3Url 直链）
async fn get_bugu_play_url(client: &Client, id: &str) -> Result<String, String> {
    let info_url = format!("{}/music/info.html?id=MUSIC_{}", BUGU_BASE, id);
    let response = client.get(&info_url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("referer", &format!("{}/", BUGU_BASE))
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 页面内JSON存在 \/ 转义："music_mp3Url":"http:\/\/xxx.mp3..."
    if let Some(idx) = html.find("music_mp3Url") {
        let rest = &html[idx..];
        if let Some(http_idx) = rest.find("http") {
            let url_part = &rest[http_idx..];
            if let Some(end) = url_part.find('"') {
                let raw = &url_part[..end];
                let url = raw
                    .replace("\\\\", "\\")
                    .replace("\\/", "/")
                    .trim_end_matches('\\')
                    .to_string();
                if url.starts_with("http") && url.contains("://") {
                    eprintln!("[Bugu] Got mp3 url from info page: {}", safe_truncate(&url, 100));
                    return Ok(url);
                }
            }
        }
    }

    Err("Failed to get play URL".to_string())
}

// 获取qq播放URL
async fn get_qq_play_url(client: &Client, id: &str) -> Result<String, String> {
    // 尝试不同的音质
    let qualities = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    
    for quality in qualities {
        let quality_str = quality.to_string();
        let response = client.get("https://api.vkeys.cn/v2/music/tencent/geturl")
            .query(&[("mid", id), ("quality", quality_str.as_str())])
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

        if json.get("code").and_then(|v| v.as_i64()) == Some(200) {
            if let Some(url) = json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                if url.starts_with("http") {
                    return Ok(url.to_string());
                }
            }
        }
    }
    
    Err("Failed to get play URL".to_string())
}

// 获取酷狗播放直链（m.kugou.com getSongInfo）
async fn get_kugou_play_url(client: &Client, hash: &str) -> Result<String, String> {
    let response = client.get("http://m.kugou.com/app/i/getSongInfo.php")
        .query(&[("cmd", "playInfo"), ("hash", hash)])
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    if let Some(url) = json.get("url").and_then(|v| v.as_str()) {
        if url.starts_with("http") {
            return Ok(url.to_string());
        }
    }

    let err = json.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
    Err(format!("Failed to get kugou play URL: {}", err))
}

// 获取网易云播放直链（外链接口，自动302重定向到mp3）
fn get_netease_play_url(id: &str) -> Result<String, String> {
    Ok(format!("https://music.163.com/song/media/outer/url?id={}.mp3", id))
}

// 获取酷我播放直链（antiserver convert_url）
async fn get_kuwo_play_url(client: &Client, id: &str) -> Result<String, String> {
    let rid = if id.starts_with("MUSIC_") { id.to_string() } else { format!("MUSIC_{}", id) };
    let response = client.get("http://antiserver.kuwo.cn/anti.s")
        .query(&[("type", "convert_url"), ("rid", rid.as_str()), ("format", "mp3"), ("response", "url")])
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
        Err("Failed to get kuwo play URL".to_string())
    }
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
        eprintln!("[Migu] Invalid ID format: {}", id);
        return Err("Invalid migu ID".to_string());
    }
    let content_id = parts[0];
    let copyright_id = parts[1];
    
    eprintln!("[Migu] Getting play URL: content_id={}, copyright_id={}", content_id, copyright_id);

    // 直接尝试常见音质组合（resourceType, toneFlag）
    // 注：旧实现先用 contentId 当关键词搜索拿 rateFormats，结果不稳定，已移除
    let rate_formats: Vec<(&str, &str)> = vec![
        ("2", "PQ"),
        ("2", "HQ"),
        ("3", "LQ"),
        ("E", "SQ"),
    ];

    eprintln!("[Migu] Trying {} quality combinations", rate_formats.len());

    // 尝试每个音质格式
    for (resource_type, format_type) in &rate_formats {
        let format_type = *format_type;
        let resource_type = *resource_type;

        eprintln!("[Migu] Trying: format_type={}, resource_type={}", format_type, resource_type);
        
        let listen_url = format!(
            "https://c.musicapp.migu.cn/MIGUM3.0/strategy/listen-url/v2.4?resourceType={}&netType=01&scene=&toneFlag={}&contentId={}&copyrightId={}&lowerQualityContentId={}",
            resource_type, format_type, content_id, copyright_id, content_id
        );
        
        let response = client.get(&listen_url)
            .header("accept", "application/json, text/plain, */*")
            .header("accept-encoding", "gzip, deflate, br, zstd")
            .header("accept-language", "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("activityid", "v4_zt_2022_music")
            .header("appid", "ce")
            .header("channel", "014X031")
            .header("deviceid", "E60C6B2F-7F11-4362-9FCE-6F1CC86E0F18")
            .header("logid", "h5page[1808]")
            .header("mgm-network-operators", "02")
            .header("mgm-network-standard", "03")
            .header("mgm-network-type", "03")
            .header("origin", "https://y.migu.cn")
            .header("recommendstatus", "1")
            .header("referer", "https://y.migu.cn/app/v4/zt/2022/music/index.html")
            .header("sec-ch-ua", "\"Google Chrome\";v=\"143\", \"Chromium\";v=\"143\", \"Not A(Brand\";v=\"24\"")
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", "\"Windows\"")
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("subchannel", "014X031")
            .header("test", "00")
            .header("ua", "Android_migu")
            .header("version", "6.8.8")
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = response.json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        let code = json.get("code").and_then(|v| v.as_str()).unwrap_or("unknown");
        eprintln!("[Migu] Response code: {}", code);

        if json.get("code").and_then(|v| v.as_i64()) == Some(200) {
            if let Some(url) = json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                if url.starts_with("http") {
                    // 修复URL中的品质路径
                    let fixed_url = url.replace("MP3_128_16_Stero", "MP3_320_16_Stero");
                    eprintln!("[Migu] Success: {}", safe_truncate(&fixed_url, 100));
                    return Ok(fixed_url);
                }
            }
        } else {
            // 尝试fallback URL
            let fallback_url = format!(
                "https://app.pd.nf.migu.cn/MIGUM3.0/v1.0/content/sub/listenSong.do?channel=mx&copyrightId={}&contentId={}&toneFlag={}&resourceType={}&userId=15548614588710179085069&netType=00",
                copyright_id, content_id, format_type, resource_type
            );
            
            let fallback_response = client.get(&fallback_url)
                .header("accept", "application/json, text/plain, */*")
                .header("accept-encoding", "gzip, deflate, br, zstd")
                .header("accept-language", "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7")
                .header("activityid", "v4_zt_2022_music")
                .header("appid", "ce")
                .header("channel", "014X031")
                .header("deviceid", "E60C6B2F-7F11-4362-9FCE-6F1CC86E0F18")
                .header("logid", "h5page[1808]")
                .header("mgm-network-operators", "02")
                .header("mgm-network-standard", "03")
                .header("mgm-network-type", "03")
                .header("origin", "https://y.migu.cn")
                .header("recommendstatus", "1")
                .header("referer", "https://y.migu.cn/app/v4/zt/2022/music/index.html")
                .header("sec-ch-ua", "\"Google Chrome\";v=\"143\", \"Chromium\";v=\"143\", \"Not A(Brand\";v=\"24\"")
                .header("sec-ch-ua-mobile", "?0")
                .header("sec-ch-ua-platform", "\"Windows\"")
                .header("sec-fetch-dest", "empty")
                .header("sec-fetch-mode", "cors")
                .header("sec-fetch-site", "same-site")
                .header("subchannel", "014X031")
                .header("test", "00")
                .header("ua", "Android_migu")
                .header("version", "6.8.8")
                .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
                .send()
                .await;
            
            if let Ok(fallback_resp) = fallback_response {
                if let Ok(fallback_json) = fallback_resp.json::<serde_json::Value>().await {
                    if let Some(url) = fallback_json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                        if url.starts_with("http") {
                            let fixed_url = url.replace("MP3_128_16_Stero", "MP3_320_16_Stero");
                            eprintln!("[Migu] Fallback success: {}", safe_truncate(&fixed_url, 100));
                            return Ok(fixed_url);
                        }
                    }
                }
            }
        }
    }
    
    eprintln!("[Migu] All qualities failed");
    Err("Failed to get play URL".to_string())
}

// 获取livepoo播放URL
async fn get_livepoo_play_url(client: &Client, id: &str) -> Result<String, String> {
    // 新版站点：详情页内嵌 music_mp3Url 直链（旧 /audio/play 接口已返回404）
    let info_url = format!("https://www.livepoo.cn/music/info.html?id=MUSIC_{}", id);
    let response = client.get(&info_url)
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("accept-language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("referer", "https://www.livepoo.cn/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;

    // 页面内JSON存在 \/ 转义："music_mp3Url":"http:\/\/xxx.mp3..."
    // 定位 music_mp3Url 后的第一个 http 链接，截至引号后再反转义
    if let Some(idx) = html.find("music_mp3Url") {
        let rest = &html[idx..];
        if let Some(http_idx) = rest.find("http") {
            let url_part = &rest[http_idx..];
            if let Some(end) = url_part.find('"') {
                let raw = &url_part[..end];
                // 先合并双反斜杠（双层转义），再处理 \/ ，最后去掉尾部残留反斜杠
                let url = raw
                    .replace("\\\\", "\\")
                    .replace("\\/", "/")
                    .trim_end_matches('\\')
                    .to_string();
                if url.starts_with("http") && url.contains("://") {
                    eprintln!("[Livepoo] Got mp3 url from info page: {}", safe_truncate(&url, 100));
                    return Ok(url);
                }
            }
        }
    }

    // Fallback：旧版播放接口（已失效，保留以防站点回滚）
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

#[command]
pub async fn get_lyrics(id: String, provider: String, title: String, artist: String) -> Result<LyricsResponse, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    eprintln!("[Lyrics] Request: {} - {} (provider: {})", title, artist, provider);

    // 根据provider选择不同的歌词获取策略
    let lyrics = match provider.as_str() {
        "jianbin-netease" | "jianbin-qq" | "jianbin-kugou" | "jianbin-kuwo" => {
            // 煎饼搜索源直接使用通用LrcApi搜索
            get_generic_lyrics(&client, &title, &artist).await
        }
        "bugu" => get_bugu_lyrics(&client, &id, &title, &artist).await,
        "qq" => get_qq_lyrics(&client, &id, &title, &artist).await,
        "migu" => get_migu_lyrics(&client, &id, &title, &artist).await,
        // 其他provider使用通用搜索
        _ => get_generic_lyrics(&client, &title, &artist).await,
    };

    match lyrics {
        Ok(lrc) => {
            eprintln!("[Lyrics] Success: {} characters", lrc.len());
            Ok(LyricsResponse {
                lyrics: lrc,
                has_lyrics: true,
            })
        }
        Err(e) => {
            eprintln!("[Lyrics] Failed: {}", e);
            Ok(LyricsResponse {
                lyrics: String::new(),
                has_lyrics: false,
            })
        }
    }
}

// 从布谷获取歌词
async fn get_bugu_lyrics(client: &Client, _id: &str, title: &str, artist: &str) -> Result<String, String> {
    eprintln!("[Bugu Lyrics] Searching: {} - {}", title, artist);
    
    // 布谷API可能不直接提供歌词，使用通用搜索
    get_generic_lyrics(client, title, artist).await
}

// 从QQ音乐获取歌词
async fn get_qq_lyrics(client: &Client, _id: &str, title: &str, artist: &str) -> Result<String, String> {
    eprintln!("[QQ Lyrics] Searching: {} - {}", title, artist);
    
    // 使用vkeys API搜索歌词
    let _response = client.get(&format!("{}/v2/music/tencent/search/song", VKEYS_BASE))
        .query(&[("word", format!("{} {}", title, artist))])
        .header("accept", "application/json, text/plain, */*")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    // QQ音乐API不直接返回歌词，使用通用搜索
    get_generic_lyrics(client, title, artist).await
}

// 从咪咕获取歌词
async fn get_migu_lyrics(client: &Client, _id: &str, title: &str, artist: &str) -> Result<String, String> {
    eprintln!("[Migu Lyrics] Searching: {} - {}", title, artist);
    
    // 咪咕API不直接提供歌词，使用通用搜索
    get_generic_lyrics(client, title, artist).await
}

// 通用歌词搜索（使用LrcApi）
async fn get_generic_lyrics(client: &Client, title: &str, artist: &str) -> Result<String, String> {
    eprintln!("[Generic Lyrics] Searching: {} - {}", title, artist);
    
    // 尝试多个API端点
    let urls = vec![
        format!(
            "https://api.lrc.cx/lyrics?title={}&artist={}&limit=1",
            utf8_percent_encode(title, NON_ALPHANUMERIC),
            utf8_percent_encode(artist, NON_ALPHANUMERIC)
        ),
        format!(
            "https://api.lrc.cx/lyrics?title={}&limit=1",
            utf8_percent_encode(title, NON_ALPHANUMERIC)
        ),
    ];
    
    for (i, url) in urls.iter().enumerate() {
        eprintln!("[Generic Lyrics] Trying URL {}", i + 1);
        
        let response = client.get(url)
            .header("accept", "text/plain, */*")
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                
                if !status.is_success() {
                    eprintln!("[Generic Lyrics] URL {} returned status: {}", i + 1, status);
                    continue;
                }
                
                let lrc_text = resp.text()
                    .await
                    .map_err(|e| format!("Read response failed: {}", e))?;
                
                // 检查是否返回了有效歌词
                if lrc_text.is_empty() || lrc_text.contains("未找到") || lrc_text.contains("not found") {
                    eprintln!("[Generic Lyrics] URL {} returned no valid lyrics", i + 1);
                    continue;
                }
                
                eprintln!("[Generic Lyrics] Success from URL {}, length: {}", i + 1, lrc_text.len());
                return Ok(lrc_text);
            }
            Err(e) => {
                eprintln!("[Generic Lyrics] URL {} request failed: {}", i + 1, e);
                continue;
            }
        }
    }
    
    Err("No lyrics found from any source".to_string())
}

#[command]
pub async fn download_update(
    app: tauri::AppHandle,
    url: String,
    filename: String,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    
    eprintln!("[Update Download] Starting download from: {}", url);
    
    // 显示保存对话框
    let save_path = app.dialog().file().set_file_name(&filename).blocking_save_file();
    
    if save_path.is_none() {
        return Err("User cancelled download".to_string());
    }
    
    let file_path = save_path.unwrap();
    let path_buf = std::path::PathBuf::from(file_path.to_string());
    eprintln!("[Update Download] Saving to: {:?}", path_buf);
    
    // 下载文件
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5分钟超时
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;
    
    let total_size = response
        .content_length()
        .ok_or("Failed to get file size")?;
    
    eprintln!("[Update Download] File size: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1024.0 / 1024.0);
    
    let mut file = tokio::fs::File::create(&path_buf)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;
    
    use tokio::io::AsyncWriteExt;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;
        
        downloaded += chunk.len() as u64;
        
        // 发送进度事件
        let progress = (downloaded as f64 / total_size as f64) * 100.0;
        eprintln!("[Update Download] Progress: {:.1}%", progress);
        
        let progress_data = serde_json::json!({
            "progress": progress,
            "downloaded": downloaded,
            "total": total_size
        });
        
        app.emit("download-progress", progress_data)
            .map_err(|e| format!("Failed to emit progress: {}", e))?;
    }
    
    file.flush().await.map_err(|e| format!("Failed to flush file: {}", e))?;
    
    eprintln!("[Update Download] Download completed successfully");
    
    Ok(path_buf.to_string_lossy().to_string())
}

#[command]
pub async fn open_download_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    
    // 获取下载目录路径
    let download_dir = app.path().download_dir()
        .map_err(|e| format!("Failed to get download directory: {}", e))?;
    
    eprintln!("[Open Folder] Opening download directory: {:?}", download_dir);
    
    // 使用系统默认程序打开文件夹
    let folder_url = format!("file:///{}", download_dir.to_string_lossy().replace("\\", "/"));
    
    // 使用 Tauri shell 打开（虽然已弃用但仍然可用）
    #[allow(deprecated)]
    {
        use tauri_plugin_shell::ShellExt;
        let shell = app.shell();
        shell.open(folder_url, None)
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

#[cfg(test)]
mod provider_tests {
    use super::*;

    fn test_client() -> Client {
        Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap()
    }

    #[tokio::test]
    #[ignore] // 需要网络，手动运行: cargo test -- --ignored
    async fn test_search_qqmp3() {
        let client = test_client();
        let resp = search_qqmp3(&client, "test").await.expect("qqmp3 search failed");
        assert!(!resp.items.is_empty(), "qqmp3 returned no items");
        println!("qqmp3: {} items, first: {} - {}", resp.items.len(), resp.items[0].title, resp.items[0].artist);
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_gequhai() {
        let client = test_client();
        let resp = search_gequhai(&client, "test", "gequhai").await.expect("gequhai search failed");
        assert!(!resp.items.is_empty(), "gequhai returned no items");
        println!("gequhai: {} items, first: {} - {} (id={})", resp.items.len(), resp.items[0].title, resp.items[0].artist, resp.items[0].id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_livepoo() {
        let client = test_client();
        let resp = search_livepoo(&client, "test").await.expect("livepoo search failed");
        assert!(!resp.items.is_empty(), "livepoo returned no items");
        println!("livepoo: {} items, first: {} - {} (id={})", resp.items.len(), resp.items[0].title, resp.items[0].artist, resp.items[0].id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_livepoo_play_url() {
        let client = test_client();
        let resp = search_livepoo(&client, "test").await.expect("livepoo search failed");
        let id = &resp.items[0].id;
        let url = get_livepoo_play_url(&client, id).await.expect("livepoo play url failed");
        assert!(url.contains("://"), "invalid url: {}", url);
        println!("livepoo play url: {}", safe_truncate(&url, 120));
    }

    #[tokio::test]
    #[ignore]
    async fn test_gequhai_play_url() {
        let client = test_client();
        let resp = search_gequhai(&client, "test", "gequhai").await.expect("gequhai search failed");
        let id = &resp.items[0].id;
        match get_gequhai_play_url(&client, id).await {
            Ok(url) => println!("gequhai play url: {}", safe_truncate(&url, 120)),
            Err(e) => println!("gequhai play url failed (may be expected for some songs): {}", e),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_qq() {
        let client = test_client();
        let resp = search_qq(&client, "test").await.expect("qq search failed");
        assert!(!resp.items.is_empty(), "qq returned no items");
        println!("qq: {} items, first: {} - {}", resp.items.len(), resp.items[0].title, resp.items[0].artist);
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_jianbin_kugou() {
        let client = test_client();
        let resp = search_jianbin_kugou(&client, "test").await.expect("kugou search failed");
        assert!(!resp.items.is_empty(), "kugou returned no items");
        println!("kugou: {} items, first: {} - {} (id={})", resp.items.len(), resp.items[0].title, resp.items[0].artist, resp.items[0].id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_jianbin_netease() {
        let client = test_client();
        let resp = search_jianbin_netease(&client, "test").await.expect("netease search failed");
        assert!(!resp.items.is_empty(), "netease returned no items");
        println!("netease: {} items, first: {} - {} (id={})", resp.items.len(), resp.items[0].title, resp.items[0].artist, resp.items[0].id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_jianbin_kuwo() {
        let client = test_client();
        let resp = search_jianbin_kuwo(&client, "test").await.expect("kuwo search failed");
        assert!(!resp.items.is_empty(), "kuwo returned no items");
        println!("kuwo: {} items, first: {} - {} (id={})", resp.items.len(), resp.items[0].title, resp.items[0].artist, resp.items[0].id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_kugou_play_url() {
        let client = test_client();
        let resp = search_jianbin_kugou(&client, "test").await.expect("kugou search failed");
        let mut got_url = false;
        for item in resp.items.iter().take(10) {
            match get_kugou_play_url(&client, &item.id).await {
                Ok(url) => {
                    println!("kugou play url ({}): {}", item.title, safe_truncate(&url, 120));
                    got_url = true;
                    break;
                }
                Err(e) => println!("kugou play url failed for {}: {}", item.title, e),
            }
        }
        assert!(got_url, "no playable kugou song found in first 10 results");
    }

    #[tokio::test]
    #[ignore]
    async fn test_kuwo_play_url() {
        let client = test_client();
        let resp = search_jianbin_kuwo(&client, "test").await.expect("kuwo search failed");
        let mut got_url = false;
        for item in resp.items.iter().take(10) {
            match get_kuwo_play_url(&client, &item.id).await {
                Ok(url) => {
                    println!("kuwo play url ({}): {}", item.title, safe_truncate(&url, 120));
                    got_url = true;
                    break;
                }
                Err(e) => println!("kuwo play url failed for {}: {}", item.title, e),
            }
        }
        assert!(got_url, "no playable kuwo song found in first 10 results");
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_bugu() {
        let client = test_client();
        let resp = search_bugu(&client, "test").await.expect("bugu search failed");
        assert!(!resp.items.is_empty(), "bugu returned no items");
        println!("bugu: {} items, first: {} - {} (id={})", resp.items.len(), resp.items[0].title, resp.items[0].artist, resp.items[0].id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_bugu_play_url() {
        let client = test_client();
        let resp = search_bugu(&client, "test").await.expect("bugu search failed");
        let id = &resp.items[0].id;
        let url = get_bugu_play_url(&client, id).await.expect("bugu play url failed");
        assert!(url.contains("://"), "invalid url: {}", url);
        println!("bugu play url: {}", safe_truncate(&url, 120));
    }
}
