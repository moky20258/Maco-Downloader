import axios from 'axios';
import { MusicItem, MusicProvider, PlayInfo } from '@/types/music';

const SEARCH_HEADERS = {
  'accept': 'application/json, text/plain, */*',
  'accept-language': 'zh-CN,zh;q=0.9,en;q=0.8',
  'origin': 'https://y.qq.com',
  'referer': 'https://y.qq.com/',
  'sec-ch-ua': '"Google Chrome";v="143", "Chromium";v="143", "Not A(Brand";v="24"',
  'sec-ch-ua-mobile': '?0',
  'sec-ch-ua-platform': '"Windows"',
  'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36',
};

type VKeysSearchItem = {
  mid?: string;
  song?: string;
  subtitle?: string;
  album?: string;
  singer?: string;
  cover?: string;
  duration?: string | number; // 时长信息
};

type VKeysSearchResponse = {
  code?: number;
  data?: VKeysSearchItem[];
};

type VKeysGetUrlResponse = {
  code?: number;
  data?: {
    url?: string;
    quality?: string;
    kbps?: string;
    cover?: string;
  };
};

const QUALITY_PRIORITY = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

function extractExt(url: string) {
  const clean = url.split('?')[0];
  const parts = clean.split('.');
  return parts.length > 1 ? parts[parts.length - 1] : 'mp3';
}

export class QQProvider implements MusicProvider {
  name = 'qq';

  async search(query: string): Promise<MusicItem[]> {
    try {
      const { data } = await axios.get<VKeysSearchResponse>('https://api.vkeys.cn/v2/music/tencent/search/song', {
        headers: SEARCH_HEADERS,
        params: { word: query },
        timeout: 15000,
      });
      const list = data?.data || [];
      return list
        .map((item) => {
          // 提取时长信息
          let duration: string | undefined;
          const durationValue = item?.duration;
          if (durationValue !== null && durationValue !== undefined) {
            const durationStr = String(durationValue).trim();
            // 如果已经是字符串格式（如 "03:45"），直接使用
            if (durationStr.includes(':')) {
              duration = durationStr;
            } else {
              // 如果是秒数，转换为 MM:SS 格式
              const seconds = Number(durationStr);
              if (Number.isFinite(seconds) && seconds > 0) {
                const minutes = Math.floor(seconds / 60);
                const remainingSeconds = Math.floor(seconds % 60);
                duration = `${minutes.toString().padStart(2, '0')}:${remainingSeconds.toString().padStart(2, '0')}`;
              }
            }
          }
          
          return {
            id: item?.mid || '',
            title: item?.song || '未知歌曲',
            artist: item?.singer || '未知歌手',
            album: item?.album || undefined,
            cover: item?.cover || undefined,
            duration,
            provider: this.name,
          };
        })
        .filter((item) => item.id);
    } catch (error) {
      console.error('QQ search error:', error);
      return [];
    }
  }

  async getPlayInfo(id: string): Promise<PlayInfo> {
    try {
      for (const quality of QUALITY_PRIORITY) {
        const { data } = await axios.get<VKeysGetUrlResponse>('https://api.vkeys.cn/v2/music/tencent/geturl', {
          headers: SEARCH_HEADERS,
          params: { mid: id, quality },
          timeout: 15000,
        });
        if (data?.code !== 200) {
          continue;
        }
        const url = data?.data?.url;
        if (typeof url === 'string' && url.startsWith('http')) {
          return {
            url,
            type: extractExt(url),
            bitrate: data?.data?.kbps || data?.data?.quality,
            cover: data?.data?.cover || undefined,
          };
        }
      }
      throw new Error('Failed to get play url');
    } catch (error) {
      console.error('QQ getPlayInfo error:', error);
      throw error;
    }
  }
}
