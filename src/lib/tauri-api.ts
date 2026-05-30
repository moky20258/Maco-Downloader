import { invoke } from '@tauri-apps/api/core';
import { MusicItem } from '@/types/music';

// 检测是否在 Tauri 环境中
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// 搜索音乐
export async function searchMusic(query: string, provider: string): Promise<MusicItem[]> {
  console.log('[searchMusic] Called with query:', query, 'provider:', provider);
  console.log('[searchMusic] isTauri():', isTauri());
  
  if (isTauri()) {
    // 使用 Tauri 命令
    try {
      console.log('[searchMusic] Invoking Tauri command...');
      const response = await invoke<{ items: MusicItem[] }>('search_music', {
        query,
        provider,
      });
      console.log('[searchMusic] Got response:', response.items.length, 'items');
      return response.items;
    } catch (error) {
      console.error('[searchMusic] Tauri search error:', error);
      return [];
    }
  } else {
    // 非 Tauri 环境（开发环境）
    console.warn('[searchMusic] Not in Tauri environment, search may not work without API server');
    return [];
  }
}

// 获取音乐播放地址
export async function getMusicUrl(id: string, provider: string): Promise<{ url: string; downloadOnly: boolean }> {
  if (isTauri()) {
    try {
      const response = await invoke<{ url: string }>('get_music_url', {
        id,
        provider,
      });
      
      // 检查是否为仅下载链接
      const downloadOnly = response.url.startsWith('DOWNLOAD_ONLY:');
      const url = downloadOnly ? response.url.replace('DOWNLOAD_ONLY:', '') : response.url;
      
      return { url, downloadOnly };
    } catch (error) {
      console.error('Tauri get URL error:', error);
      return { url: '', downloadOnly: false };
    }
  } else {
    console.warn('Not in Tauri environment');
    return { url: '', downloadOnly: false };
  }
}

// 下载音乐
export async function downloadMusic(id: string, filename: string, provider: string): Promise<ArrayBuffer> {
  if (isTauri()) {
    try {
      const bytes = await invoke<number[]>('download_music', {
        id,
        filename,
        provider,
      });
      // 将数字数组转换为 ArrayBuffer
      return new Uint8Array(bytes).buffer;
    } catch (error) {
      console.error('Tauri download error:', error);
      throw error;
    }
  } else {
    console.warn('Not in Tauri environment');
    throw new Error('Download not available');
  }
}

// 获取歌词
export async function getLyrics(
  id: string, 
  provider: string, 
  title: string, 
  artist: string
): Promise<{ lyrics: string; hasLyrics: boolean }> {
  if (isTauri()) {
    try {
      const response = await invoke<{ lyrics: string; has_lyrics: boolean }>('get_lyrics', {
        id,
        provider,
        title,
        artist,
      });
      return {
        lyrics: response.lyrics,
        hasLyrics: response.has_lyrics,
      };
    } catch (error) {
      console.error('Tauri get lyrics error:', error);
      return { lyrics: '', hasLyrics: false };
    }
  } else {
    console.warn('Not in Tauri environment');
    return { lyrics: '', hasLyrics: false };
  }
}
