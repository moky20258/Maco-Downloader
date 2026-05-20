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
    const response = await invoke<{ items: MusicItem[] }>('search_music', {
      query,
      provider,
    });
    return response.items;
  } else {
    // 使用 Next.js API（开发环境）
    const res = await fetch(`/api/search?q=${encodeURIComponent(query)}&provider=${provider}`);
    const data = await res.json();
    return data.items || [];
  }
}

// 获取音乐播放地址
export async function getMusicUrl(id: string, provider: string): Promise<string> {
  if (isTauri()) {
    const response = await invoke<{ url: string }>('get_music_url', {
      id,
      provider,
    });
    return response.url;
  } else {
    const res = await fetch(`/api/url?id=${encodeURIComponent(id)}&provider=${provider}`);
    const data = await res.json();
    return data.url;
  }
}

// 下载音乐
export async function downloadMusic(id: string, filename: string, provider: string): Promise<ArrayBuffer> {
  if (isTauri()) {
    const bytes = await invoke<number[]>('download_music', {
      id,
      filename,
      provider,
    });
    // 将数字数组转换为 ArrayBuffer
    return new Uint8Array(bytes).buffer;
  } else {
    // 返回 URL（开发环境）
    const url = `/api/download?id=${encodeURIComponent(id)}&filename=${encodeURIComponent(filename)}&provider=${provider}`;
    const response = await fetch(url);
    return await response.arrayBuffer();
  }
}
