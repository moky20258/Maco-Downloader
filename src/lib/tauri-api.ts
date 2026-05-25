import { invoke } from '@tauri-apps/api/core';
import { MusicItem } from '@/types/music';

// 检测是否在 Tauri 环境中
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// 搜索音乐
export async function searchMusic(query: string, provider: string): Promise<MusicItem[]> {
  if (isTauri()) {
    // 使用 Tauri 命令
    try {
      const response = await invoke<{ items: MusicItem[] }>('search_music', {
        query,
        provider,
      });
      return response.items;
    } catch (error) {
      console.error('Tauri search error:', error);
      return [];
    }
  } else {
    // 非 Tauri 环境（开发环境）
    console.warn('Not in Tauri environment, search may not work without API server');
    return [];
  }
}

// 获取音乐播放地址
export async function getMusicUrl(id: string, provider: string): Promise<string> {
  if (isTauri()) {
    try {
      const response = await invoke<{ url: string }>('get_music_url', {
        id,
        provider,
      });
      return response.url;
    } catch (error) {
      console.error('Tauri get URL error:', error);
      return '';
    }
  } else {
    console.warn('Not in Tauri environment');
    return '';
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
