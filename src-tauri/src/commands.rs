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
    eprintln!("[Jianbin] Searching: query='{}', provider='{}'", query, provider);
    
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

    eprintln!("[Jianbin] Requesting: {} with source={}", JBSOU_BASE, source);
    
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

    eprintln!("[Jianbin] Response length: {}", body.len());
    eprintln!("[Jianbin] Response preview: {}", &body[..body.len().min(200)]);

    // 解析 JSON
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    // 提取 data 数组
    let data = json.get("data")
        .and_then(|d| d.as_array())
        .ok_or("Invalid response format")?;

    eprintln!("[Jianbin] Found {} items in data array", data.len());

    let items: Vec<MusicItem> = data.iter()
        .filter_map(|item| {
            let url = item.get("url")?.as_str()?;
            let absolute_url = to_absolute_url(url);
            
            // 尝试提取时长信息
            let duration = item.get("duration")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    // 如果API返回的是数字（秒数），转换为 MM:SS 格式
                    item.get("duration")
                        .and_then(|v| v.as_i64())
                        .map(|secs| {
                            let minutes = secs / 60;
                            let seconds = secs % 60;
                            format!("{}:{:02}", minutes, seconds)
                        })
                });
            
            Some(MusicItem {
                id: percent_encoding::utf8_percent_encode(&absolute_url, percent_encoding::NON_ALPHANUMERIC).to_string(),
                title: item.get("name").and_then(|v| v.as_str()).unwrap_or("未知歌曲").to_string(),
                artist: item.get("artist").and_then(|v| v.as_str()).unwrap_or("未知歌手").to_string(),
                album: item.get("album").and_then(|v| v.as_str()).map(|s| s.to_string()),
                cover: item.get("cover").and_then(|v| v.as_str()).map(to_absolute_url),
                duration,
                size: None,
                provider: provider.to_string(),
            })
        })
        .filter(|item| !item.id.is_empty())
        .collect();

    eprintln!("[Jianbin] Returning {} items", items.len());

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
                duration: None, // gequhai搜索页面不提供时长信息
                size: None,
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
            let duration = item.get("duration").and_then(|v| v.as_str()).map(|s| s.to_string());
            let size = item.get("size").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            Some(MusicItem {
                id,
                title: title.to_string(),
                artist: artist.to_string(),
                album,
                cover,
                duration,
                size,
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

    Ok(UrlResponse { url, download_only: None })
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

// 获取gequbao播放URL
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
                    
                    eprintln!("Gequbao API response: {}", &text[..text.len().min(200)]);

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
    eprintln!("[Gequhai] API response: {}", &text[..text.len().min(500)]);
    
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
                eprintln!("[Gequhai] Found mp3_extra_url (encoded): {}", &encoded_url[..encoded_url.len().min(200)]);
                
                // Base64解码 (先将#替换为H，然后Base64解码)
                let b64_str = encoded_url.replace('#', "H");
                eprintln!("[Gequhai] Base64 string: {}", &b64_str[..b64_str.len().min(200)]);
                
                use base64::{Engine as _, engine::general_purpose::STANDARD};
                if let Ok(decoded_bytes) = STANDARD.decode(&b64_str) {
                    if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                        eprintln!("[Gequhai] Decoded URL: {}", &decoded_str[..decoded_str.len().min(200)]);
                        
                        // 检查是否为网盘链接（仅支持下载）
                        if decoded_str.contains("pan.quark.cn") || decoded_str.contains("pan.baidu.com") {
                            eprintln!("[Gequhai] Detected cloud drive URL, adding DOWNLOAD_ONLY marker");
                            // 添加特殊前缀标记，前端检测到后会提示用户
                            let marked_url = format!("DOWNLOAD_ONLY:{}", decoded_str);
                            eprintln!("[Gequhai] Marked URL: {}", &marked_url[..marked_url.len().min(200)]);
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

// 获取bugu播放URL
async fn get_bugu_play_url(client: &Client, id: &str) -> Result<String, String> {
    let response = client.get("https://a.buguyy.top/newapi/geturl2.php")
        .query(&[("id", id)])
        .header("accept", "application/json, text/plain, */*")
        .header("origin", "https://buguyy.top")
        .header("referer", "https://buguyy.top/")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    // 先获取文本，以便调试
    let text = response.text()
        .await
        .map_err(|e| format!("Read response failed: {}", e))?;
    
    eprintln!("[Bugu] API response: {}", &text[..text.len().min(200)]);

    // 尝试解析JSON
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Parse JSON failed: {}", e))?;

    if let Some(url) = json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
        Ok(url.to_string())
    } else {
        Err("Failed to get play URL".to_string())
    }
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

    // 第一步：搜索歌曲详情，获取rateFormats
    let search_url = format!(
        "https://c.musicapp.migu.cn/v1.0/content/search_all.do?text={}&pageNo=1&pageSize=1&isCopyright=1&sort=1&searchSwitch={{'song':1,'album':0,'singer':0,'tagSong':1,'mvSong':0,'bestShow':1}}",
        content_id
    );
    
    eprintln!("[Migu] Searching for song details");
    
    let search_response = client.get(&search_url)
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
        .map_err(|e| format!("Search request failed: {}", e))?;

    let search_json: serde_json::Value = search_response.json()
        .await
        .map_err(|e| format!("Parse search JSON failed: {}", e))?;

    // 获取歌曲列表
    let songs = search_json.get("songResultData")
        .and_then(|d| d.get("result"))
        .and_then(|r| r.as_array())
        .ok_or("No songs found")?;
    
    if songs.is_empty() {
        return Err("Song not found".to_string());
    }
    
    let song = &songs[0];
    
    // 获取rateFormats和newRateFormats
    let mut rate_formats = Vec::new();
    if let Some(formats) = song.get("rateFormats").and_then(|f| f.as_array()) {
        rate_formats.extend(formats.iter().cloned());
    }
    if let Some(formats) = song.get("newRateFormats").and_then(|f| f.as_array()) {
        rate_formats.extend(formats.iter().cloned());
    }
    
    if rate_formats.is_empty() {
        return Err("No rate formats found".to_string());
    }
    
    eprintln!("[Migu] Found {} rate formats", rate_formats.len());
    
    // 尝试每个音质格式
    for rate in &rate_formats {
        let format_type = rate.get("formatType").and_then(|v| v.as_str()).unwrap_or("");
        let resource_type = rate.get("resourceType").and_then(|v| v.as_str()).unwrap_or("");
        
        if format_type.is_empty() || resource_type.is_empty() {
            continue;
        }
        
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
                    eprintln!("[Migu] Success: {}", &fixed_url[..fixed_url.len().min(100)]);
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
                            eprintln!("[Migu] Fallback success: {}", &fixed_url[..fixed_url.len().min(100)]);
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
