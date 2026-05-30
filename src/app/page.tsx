"use client";

import React, { useState, useRef, useEffect } from "react";
import Image from "next/image";
import { Search, Loader2, Play, Pause, Download, Check, Music, Trash2, Flame, Zap, ShieldCheck, Headphones, ExternalLink, ListPlus, X, Shuffle } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";
import { MusicItem } from "@/types/music";
import { PlayerBar } from "@/components/PlayerBar";
import { DownloadDrawer } from "@/components/DownloadDrawer";
import { PlaylistDrawer } from "@/components/PlaylistDrawer";
import { LyricsPanel } from "@/components/LyricsPanel";
import { UpdateChecker } from "@/components/UpdateChecker";
import { DownloadTask } from "@/types/download";
import axios from "axios";
import { searchMusic, getMusicUrl, downloadMusic } from "@/lib/tauri-api";

const SourceLinkButton = ({ item }: { item: MusicItem }) => {
  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    
    // 根据 provider 打开对应的源网站
    const providerUrls: Record<string, string> = {
      'gequbao': 'https://www.gequbao.com',
      'gequhai': 'https://www.gequhai.com',
      'bugu': 'https://a.buguyy.top',
      'qq': 'https://y.qq.com',
      'qqmp3': 'https://www.qqmp3.vip',
      'migu': 'https://music.migu.cn',
      'livepoo': 'https://www.livepoo.cn',
      'jianbin-netease': 'https://music.163.com',
      'jianbin-qq': 'https://y.qq.com',
      'jianbin-kugou': 'https://www.kugou.com',
      'jianbin-kuwo': 'https://www.kuwo.cn',
    };
    
    const baseUrl = providerUrls[item.provider] || 'https://www.gequbao.com';
    // 尝试构造搜索 URL（不同网站的搜索 URL 格式不同）
    const searchUrl = `${baseUrl}/search?q=${encodeURIComponent(item.title)}`;
    
    window.open(searchUrl, '_blank');
  };

  return (
    <button
      onClick={handleClick}
      className="p-2 text-slate-400 dark:text-slate-500 hover:text-sky-500 dark:hover:text-sky-400 hover:bg-sky-50 dark:hover:bg-slate-800 rounded-full transition-colors cursor-pointer flex items-center justify-center"
      title="打开源网站"
    >
      <ExternalLink className="w-5 h-5" />
    </button>
  );
};

type PlayMode = "order" | "shuffle" | "single";

export default function Home() {
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState("jianbin-kugou");
  const [results, setResults] = useState<MusicItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [randomLoading, setRandomLoading] = useState(false);
  const [isRandomListen, setIsRandomListen] = useState(false);
  
  // Playback State
  const [activeMusic, setActiveMusic] = useState<MusicItem | null>(null);
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [playMode, setPlayMode] = useState<PlayMode>("order");
  const [shuffleOrder, setShuffleOrder] = useState<string[]>([]);
  const [shuffleIndex, setShuffleIndex] = useState(-1);

  const [searched, setSearched] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [downloadingCount, setDownloadingCount] = useState(0);
  
  // 用于取消时长获取的 AbortController
  const durationFetchControllerRef = useRef<AbortController | null>(null);
  
  // 播放锁定：防止并发播放请求
  const isPlaybackLockedRef = useRef<boolean>(false);
  
  // Download Manager State
  const [downloadTasks, setDownloadTasks] = useState<DownloadTask[]>([]);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [downloadEnabled, setDownloadEnabled] = useState(true);
  
  // Playlist State - with persistence
  const [playlist, setPlaylist] = useState<MusicItem[]>(() => {
    // 从 localStorage 恢复播放列表
    if (typeof window !== 'undefined') {
      try {
        const saved = localStorage.getItem('maco-playlist');
        if (saved) {
          return JSON.parse(saved);
        }
      } catch (error) {
        console.error('Failed to load playlist from localStorage:', error);
      }
    }
    return [];
  });
  const [isPlaylistOpen, setIsPlaylistOpen] = useState(false);
  const [isLyricsOpen, setIsLyricsOpen] = useState(false);
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [toastPosition, setToastPosition] = useState<{ top: number; left: number } | null>(null);

  // 显示提示消息
  const showToast = (msg: string, element?: HTMLElement) => {
    setToastMessage(msg);
    
    // 如果提供了元素，定位到该元素附近
    if (element) {
      const rect = element.getBoundingClientRect();
      setToastPosition({
        top: rect.top + window.scrollY,
        left: rect.left + window.scrollX
      });
    } else {
      setToastPosition(null);
    }
    
    setTimeout(() => {
      setToastMessage(null);
      setToastPosition(null);
    }, 3000);
  };

  const openSourceUrl = async (item: MusicItem) => {
    const { url } = await getMusicUrl(item.id, item.provider || "gequbao");
    if (url) {
      window.open(url, "_blank");
      return;
    }
    throw new Error("Failed to get source url");
  };

  const buildShuffleOrder = (ids: string[]) => {
    const next = [...ids];
    for (let i = next.length - 1; i > 0; i -= 1) {
      const j = Math.floor(Math.random() * (i + 1));
      [next[i], next[j]] = [next[j], next[i]];
    }
    return next;
  };

  // 搜索超时控制
  const searchAbortControllerRef = useRef<AbortController | null>(null);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;
    
    setLoading(true);
    setSearched(true);
    setIsRandomListen(false);
    setResults([]);
    setSelectedIds(new Set());
    
    // 创建 AbortController 用于取消请求
    if (searchAbortControllerRef.current) {
      searchAbortControllerRef.current.abort();
    }
    searchAbortControllerRef.current = new AbortController();
    const { signal } = searchAbortControllerRef.current;
    
    // 超时控制：15秒后自动停止
    const timeoutId = setTimeout(() => {
      if (!signal.aborted) {
        console.warn('[Search] Timeout after 15s, aborting...');
        searchAbortControllerRef.current?.abort();
      }
    }, 15000);
    
    try {
      const items = await searchMusic(query, provider);
      
      // 检查是否被取消
      if (signal.aborted) {
        console.log('[Search] Request was aborted, stopping...');
        if (items.length === 0) {
          alert('搜索超时或网络异常，请稍后重试或切换搜索源');
          setSearched(false);
          setLoading(false);
          return;
        }
        // 有部分结果，继续处理
        console.log('[Search] Using partial results:', items.length);
      }
      
      // 清除超时定时器
      clearTimeout(timeoutId);
      
      setResults(items);
      
      // 搜索完成后，在后台异步获取每首歌的时长
      fetchDurationsForResults(items);
    } catch (err) {
      console.error('[Search] Search failed:', err);
      // 清除超时定时器
      clearTimeout(timeoutId);
      
      // 检查是否是取消操作
      if (searchAbortControllerRef.current?.signal.aborted) {
        console.log('[Search] Request was cancelled');
      } else {
        // 真正的错误
        alert('搜索失败，请检查网络连接或切换搜索源');
        setSearched(false);
      }
    } finally {
      // 清除超时定时器
      clearTimeout(timeoutId);
      setLoading(false);
    }
  };

  // 随便听听：随机搜索热门关键词，获取20首歌曲
  const randomAbortControllerRef = useRef<AbortController | null>(null);

  const handleRandomListen = async () => {
    if (randomLoading) return;
      
    setRandomLoading(true);
    setSearched(true);
    setIsRandomListen(true);
    setResults([]);
    setSelectedIds(new Set());
    
    // 创建 AbortController 用于取消请求
    if (randomAbortControllerRef.current) {
      randomAbortControllerRef.current.abort();
    }
    randomAbortControllerRef.current = new AbortController();
    const { signal } = randomAbortControllerRef.current;
    
    // 超时控制：15秒后自动停止
    const timeoutId = setTimeout(() => {
      if (!signal.aborted) {
        console.warn('[Random Listen] Timeout after 15s, aborting...');
        randomAbortControllerRef.current?.abort();
      }
    }, 15000);
      
    // 按类型分类的热门搜索词库（50+个关键词）
    const hotKeywordsByCategory = {
      // 华语流行
      huayu: [
        "周杰伦", "林俊杰", "陈奕迅", "薛之谦", "邓紫棋",
        "王力宏", "蔡依林", "五月天", "李荣浩", "毛不易"
      ],
      // 欧美热歌
      oumei: [
        "Taylor Swift", "Adele", "Ed Sheeran", "Justin Bieber",
        "The Weeknd", "Billie Eilish", "Bruno Mars", "Dua Lipa"
      ],
      // 抖音热歌
      douyin: [
        "抖音热歌", "网络热歌", "快手热歌", "热门BGM",
        "抖音神曲", "网红歌曲", "短视频热歌"
      ],
      // 经典老歌
      laoge: [
        "经典老歌", "80年代歌曲", "90年代歌曲", "港台老歌",
        "怀旧歌曲", "流金岁月", "岁月经典"
      ],
      // 古风歌曲
      gufeng: [
        "古风歌曲", "古风", "国风音乐", "仙侠歌曲",
        "古风音乐", "中国风歌曲"
      ],
      // 民谣
      minyao: [
        "民谣", "校园民谣", "城市民谣", "独立民谣",
        "赵雷", "宋冬野", "陈粒"
      ],
      // 摇滚
      yaogun: [
        "摇滚", "中国摇滚", "Beyond", "五月天摇滚",
        "汪峰", "许巍", "朴树"  ],
      // 说唱
      shuochang: [
        "说唱", "中文说唱", "Rap", "HIPHOP",
        "中国有嘻哈", "rapper"
      ],
      // 电子音乐
      dianzi: [
        "电子音乐", "EDM", "DJ", "电音",
        "舞曲", "Club Music"
      ],
      // 伤感歌曲
      shanggan: [
        "伤感歌曲", "催泪歌曲", "失恋歌曲",
        "悲伤歌曲", "情感歌曲"
      ]
    };
      
    try {
      // 从所有分类中随机选择3个不同的分类
      const categories = Object.keys(hotKeywordsByCategory);
      const shuffledCategories = categories.sort(() => Math.random() - 0.5);
      const selectedCategories = shuffledCategories.slice(0, 3);
        
      console.log('[Random Listen] Selected categories:', selectedCategories);
        
      // 从每个选中的分类中随机选择1个关键词
      const selectedKeywords: string[] = [];
      for (const category of selectedCategories) {
        const keywords = hotKeywordsByCategory[category as keyof typeof hotKeywordsByCategory];
        const randomKeyword = keywords[Math.floor(Math.random() * keywords.length)];
        selectedKeywords.push(randomKeyword);
      }
        
      console.log('[Random Listen] Searching for keywords:', selectedKeywords);
      console.log('[Random Listen] Current provider:', provider);
            
      const allResults: MusicItem[] = [];
      let successCount = 0;
      let errorCount = 0;
            
      // 并发搜索多个关键词
      const searchPromises = selectedKeywords.map(async (keyword) => {
        try {
          // 检查是否已被取消
          if (signal.aborted) {
            console.log('[Random Listen] Search cancelled for:', keyword);
            return [];
          }
          
          console.log('[Random Listen] Searching for:', keyword);
          const items = await searchMusic(keyword, provider);
          console.log('[Random Listen] Got', items.length, 'items for:', keyword);
          
          if (items.length > 0) {
            successCount++;
          } else {
            errorCount++;
          }
          
          return items;
        } catch (err) {
          console.error('[Random Listen] Failed to search "' + keyword + '":', err);
          errorCount++;
          return [];
        }
      });
            
      console.log('[Random Listen] Waiting for all searches to complete...');
      const resultsArray = await Promise.all(searchPromises);
      console.log('[Random Listen] All searches completed');
      
      // 检查是否被取消
      if (signal.aborted) {
        console.log('[Random Listen] Request was aborted, stopping...');
        // 检查是否有部分结果
        for (const items of resultsArray) {
          allResults.push(...items);
        }
        
        if (allResults.length === 0) {
          // 超时或取消且无结果
          alert('搜索超时或网络异常，请稍后重试或切换搜索源');
          setSearched(false);
          setIsRandomListen(false);
          setRandomLoading(false);
          return;
        }
        
        // 有部分结果，继续处理
        console.log('[Random Listen] Using partial results:', allResults.length);
      } else {
        // 正常完成，合并所有结果
        for (const items of resultsArray) {
          allResults.push(...items);
        }
      }
      
      // 清除超时定时器
      clearTimeout(timeoutId);
      
      console.log('[Random Listen] Total results before dedup:', allResults.length);
            
      // 去重（根据 id + provider）
      const uniqueMap = new Map<string, MusicItem>();
      for (const item of allResults) {
        const key = `${item.provider}-${item.id}`;
        if (!uniqueMap.has(key)) {
          uniqueMap.set(key, item);
        }
      }
            
      // 随机打乱并20首
      const uniqueSongs = Array.from(uniqueMap.values());
      console.log('[Random Listen] Unique songs after dedup:', uniqueSongs.length);
      
      const shuffledSongs = uniqueSongs.sort(() => Math.random() - 0.5);
      const finalResults = shuffledSongs.slice(0, 20);
      
      console.log('[Random Listen] Final results to display:', finalResults.length);
      
      setResults(finalResults);
            
      // 在后台异步获取每首歌的时长
      fetchDurationsForResults(finalResults);
    } catch (err) {
      console.error('Random listen failed:', err);
      // 清除超时定时器
      clearTimeout(timeoutId);
      
      // 检查是否是取消操作
      if (randomAbortControllerRef.current?.signal.aborted) {
        console.log('[Random Listen] Request was cancelled');
        // 已经被处理，不需要额外操作
      } else {
        // 真正的错误
        alert('搜索失败，请检查网络连接或切换搜索源');
        setSearched(false);
        setIsRandomListen(false);
      }
    } finally {
      // 清除超时定时器
      clearTimeout(timeoutId);
      setRandomLoading(false);
      setIsRandomListen(false);
    }
  };

  const syncShuffleIndex = (id: string) => {
    const index = shuffleOrder.indexOf(id);
    if (index >= 0) {
      setShuffleIndex(index);
      return;
    }
    if (results.length > 0) {
      const ids = results.map(r => r.id);
      const nextOrder = buildShuffleOrder(ids);
      setShuffleOrder(nextOrder);
      setShuffleIndex(nextOrder.indexOf(id));
    } else {
      setShuffleIndex(-1);
    }
  };

  const getNextIndexById = (id: string) => {
    if (playMode === "shuffle") {
      const order = shuffleOrder.length > 0 ? shuffleOrder : results.map(r => r.id);
      const orderIndex = order.indexOf(id);
      if (orderIndex >= 0 && orderIndex < order.length - 1) {
        const nextId = order[orderIndex + 1];
        return results.findIndex(r => r.id === nextId);
      }
      return -1;
    }
    const index = results.findIndex(r => r.id === id);
    if (index >= 0 && index < results.length - 1) {
      return index + 1;
    }
    return -1;
  };

  const handlePlay = async (item: MusicItem, event?: React.MouseEvent) => {
    // 如果点击的是当前正在播放的歌曲，切换播放/暂停
    if (activeMusic?.id === item.id && activeMusic?.provider === item.provider) {
      if (playing) {
        audioRef.current?.pause();
        setPlaying(false);
      } else {
        audioRef.current?.play().catch(() => setPlaying(false));
        setPlaying(true);
      }
      return;
    }

    // 防止并发播放请求
    if (isPlaybackLockedRef.current) {
      console.log('Playback locked, ignoring request');
      return;
    }

    try {
      // 锁定播放，防止并发请求
      isPlaybackLockedRef.current = true;
      
      // 停止当前播放
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current.src = ''; // 清空src，停止加载
      }
      
      // 设置新的播放状态
      setActiveMusic(item);
      syncShuffleIndex(item.id);
      setPlaying(false);
      setCurrentTime(0);
      setDuration(0);

      // 获取音乐URL
      const { url, downloadOnly } = await getMusicUrl(item.id, item.provider || 'gequbao');
      
      // 检查是否已被取消（用户可能又点击了其他歌曲）
      if (!isPlaybackLockedRef.current) {
        return;
      }
      
      // 如果歌曲仅支持下载，弹出提示
      if (downloadOnly) {
        const targetElement = event?.currentTarget as HTMLElement | undefined;
        showToast(`《${item.title}》仅支持下载，不支持在线播放`, targetElement);
        setActiveMusic(null); // 清除当前播放状态
        isPlaybackLockedRef.current = false; // 解锁
        return;
      }
      
      if (!url || !audioRef.current) {
        // 获取URL失败，提示用户并跳到下一首
        console.warn(`无法获取《${item.title}》的播放链接`);
        showToast(`无法获取《${item.title}》的播放链接，跳到下一首`);
        setActiveMusic(null);
        isPlaybackLockedRef.current = false; // 先解锁
        // 延迟调用，确保状态已更新
        setTimeout(() => {
          const nextIndex = getNextIndex();
          if (nextIndex >= 0) {
            if (isPlayingFromPlaylist) {
              handlePlay(playlist[nextIndex]);
            } else {
              handlePlay(results[nextIndex]);
            }
          }
        }, 100);
        return;
      }
      
      // 设置音频源并播放
      audioRef.current.src = url;
      audioRef.current.load();
      
      // 添加超时机制，防止永久卡住
      const playTimeout = setTimeout(() => {
        if (isPlaybackLockedRef.current && !playing) {
          console.warn(`歌曲《${item.title}》加载超时，自动跳到下一首`);
          showToast(`《${item.title}》加载超时，跳到下一首`);
          
          // 先解锁
          isPlaybackLockedRef.current = false;
          
          // 清空当前播放状态
          setActiveMusic(null);
          setPlaying(false);
          
          // 跳到下一首
          setTimeout(() => {
            // 再次确保锁已释放
            isPlaybackLockedRef.current = false;
            
            const nextIndex = getNextIndex();
            if (nextIndex >= 0) {
              if (isPlayingFromPlaylist) {
                handlePlay(playlist[nextIndex]);
              } else {
                handlePlay(results[nextIndex]);
              }
            }
          }, 300); // 增加延迟到300ms
        }
      }, 10000); // 10秒超时
      
      try {
        await audioRef.current.play();
        clearTimeout(playTimeout);
        setPlaying(true);
        // 播放成功，解锁
        isPlaybackLockedRef.current = false;
      } catch (playError) {
        clearTimeout(playTimeout);
        console.error("Play failed", playError);
        showToast(`《${item.title}》播放失败，跳到下一首`);
        
        // 先解锁，确保可以播放下一首
        isPlaybackLockedRef.current = false;
        
        // 清空当前播放状态
        setActiveMusic(null);
        setPlaying(false);
        
        // 延迟调用，确保状态已更新
        setTimeout(() => {
          // 再次确保锁已释放
          isPlaybackLockedRef.current = false;
          
          const nextIndex = getNextIndex();
          if (nextIndex >= 0) {
            if (isPlayingFromPlaylist) {
              handlePlay(playlist[nextIndex]);
            } else {
              handlePlay(results[nextIndex]);
            }
          }
        }, 300); // 增加延迟到300ms
        return;
      }
      
      // 异步获取文件大小
      if (!item.size && url) {
        fetchFileSize(item.id, url);
      }
    } catch (err) {
      console.error('handlePlay error:', err);
      showToast(`播放《${item.title}》时发生错误，跳到下一首`);
      
      // 先解锁
      isPlaybackLockedRef.current = false;
      
      // 清空当前播放状态
      setActiveMusic(null);
      setPlaying(false);
      
      // 延迟调用，确保状态已更新
      setTimeout(() => {
        // 再次确保锁已释放
        isPlaybackLockedRef.current = false;
        
        const nextIndex = getNextIndex();
        if (nextIndex >= 0) {
          if (isPlayingFromPlaylist) {
            handlePlay(playlist[nextIndex]);
          } else {
            handlePlay(results[nextIndex]);
          }
        }
      }, 300); // 增加延迟到300ms
    }
  };

  // 获取文件大小
  const fetchFileSize = async (itemId: string, url: string) => {
    try {
      console.log('Fetching file size for:', itemId, url);
      
      // 方法1: 尝试 HEAD 请求
      let contentLength: string | null = null;
      
      try {
        const headResponse = await fetch(url, { 
          method: 'HEAD',
          mode: 'cors',
        });
        contentLength = headResponse.headers.get('content-length');
        console.log('HEAD request content-length:', contentLength);
      } catch (headError) {
        console.log('HEAD request failed, trying GET with range:', headError);
        // 方法2: 如果 HEAD 失败，尝试 GET 请求（只取少量数据）
        try {
          const getResponse = await fetch(url, {
            method: 'GET',
            headers: {
              'Range': 'bytes=0-0',
            },
          });
          if (getResponse.status === 206 || getResponse.status === 200) {
            contentLength = getResponse.headers.get('content-range')?.split('/')[1] 
              || getResponse.headers.get('content-length');
            console.log('GET request content-length:', contentLength);
          }
        } catch (getError) {
          console.log('GET request also failed:', getError);
        }
      }
      
      if (contentLength) {
        const bytes = parseInt(contentLength, 10);
        console.log('File size in bytes:', bytes);
        
        if (bytes > 0) {
          let sizeStr: string;
          // 判断是否需要转换（大于 1MB 的数值认为是字节）
          if (bytes > 1048576) {
            const mb = (bytes / (1024 * 1024)).toFixed(2);
            sizeStr = `${mb}MB`;
          } else if (bytes > 1024) {
            const kb = (bytes / 1024).toFixed(2);
            sizeStr = `${kb}KB`;
          } else {
            sizeStr = `${bytes}B`;
          }
          
          console.log('Formatted size:', sizeStr);
          
          // 更新 results 中对应 item 的 size
          setResults(prev => prev.map(item => 
            item.id === itemId ? { ...item, size: sizeStr } : item
          ));
          
          // 同时更新播放列表中的 size
          setPlaylist(prev => prev.map(item => 
            item.id === itemId ? { ...item, size: sizeStr } : item
          ));
          
          // 如果当前正在播放这首歌，也更新 activeMusic
          setActiveMusic(prev => {
            if (prev && prev.id === itemId) {
              return { ...prev, size: sizeStr };
            }
            return prev;
          });
        }
      } else {
        console.log('No content-length header found');
      }
    } catch (error) {
      // 静默失败，不影响播放
      console.error('Failed to fetch file size:', error);
    }
  };

  // 批量获取搜索结果中歌曲的时长
  const fetchDurationsForResults = async (items: MusicItem[]) => {
    // 取消之前的时长获取任务
    if (durationFetchControllerRef.current) {
      durationFetchControllerRef.current.abort();
    }
    
    // 创建新的 AbortController
    const controller = new AbortController();
    durationFetchControllerRef.current = controller;
    
    // 限制并发数量和总数，避免过多请求
    const batchSize = 3;
    const maxItems = 15; // 最多获取前15首的时长
    
    try {
      for (let i = 0; i < Math.min(items.length, maxItems); i += batchSize) {
        // 检查是否已被取消
        if (controller.signal.aborted) {
          console.log('Duration fetch cancelled');
          return;
        }
        
        const batch = items.slice(i, i + batchSize);
        
        const promises = batch.map(async (item) => {
          // 检查是否已被取消
          if (controller.signal.aborted) return;
          
          // 如果已经有时长信息，跳过
          if (item.duration) return;
          
          try {
            const { url } = await getMusicUrl(item.id, item.provider || 'gequbao');
            if (!url) return;
            
            // 检查是否已被取消
            if (controller.signal.aborted) return;
            
            // 创建一个临时的 Audio 对象来获取时长
            const tempAudio = new Audio();
            
            // 等待元数据加载
            await new Promise<void>((resolve, reject) => {
              const timeout = setTimeout(() => {
                tempAudio.removeEventListener('loadedmetadata', onLoaded);
                tempAudio.removeEventListener('error', onError);
                resolve();
              }, 5000); // 5秒超时
              
              const onLoaded = () => {
                clearTimeout(timeout);
                tempAudio.removeEventListener('loadedmetadata', onLoaded);
                tempAudio.removeEventListener('error', onError);
                resolve();
              };
              
              const onError = () => {
                clearTimeout(timeout);
                tempAudio.removeEventListener('loadedmetadata', onLoaded);
                tempAudio.removeEventListener('error', onError);
                reject(new Error('Failed to load audio'));
              };
              
              tempAudio.addEventListener('loadedmetadata', onLoaded);
              tempAudio.addEventListener('error', onError);
              tempAudio.src = url;
              tempAudio.load();
            });
            
            // 检查是否已被取消
            if (controller.signal.aborted) {
              tempAudio.src = '';
              return;
            }
            
            // 获取时长
            if (Number.isFinite(tempAudio.duration) && tempAudio.duration > 0) {
              const minutes = Math.floor(tempAudio.duration / 60);
              const seconds = Math.floor(tempAudio.duration % 60);
              const durationStr = `${minutes}:${seconds.toString().padStart(2, '0')}`;
              
              // 更新 results 中的 duration
              setResults(prev => prev.map(prevItem => 
                (prevItem.id === item.id && prevItem.provider === item.provider)
                  ? { ...prevItem, duration: durationStr }
                  : prevItem
              ));
            }
            
            // 清理
            tempAudio.src = '';
          } catch (error) {
            // 如果是取消操作，不打印错误
            if (error instanceof DOMException && error.name === 'AbortError') {
              return;
            }
            // 静默失败，不影响用户体验
            console.error(`Failed to fetch duration for ${item.title}:`, error);
          }
        });
        
        // 等待当前批次完成
        await Promise.all(promises);
        
        // 检查是否已被取消
        if (controller.signal.aborted) {
          console.log('Duration fetch cancelled between batches');
          return;
        }
        
        // 批次间稍微延迟，避免请求过快
        if (i + batchSize < Math.min(items.length, maxItems)) {
          await new Promise(resolve => setTimeout(resolve, 300));
        }
      }
    } catch (error) {
      // 如果是取消操作，不打印错误
      if (error instanceof DOMException && error.name === 'AbortError') {
        console.log('Duration fetch aborted');
        return;
      }
      console.error('Error in fetchDurationsForResults:', error);
    }
  };

  const handleSeek = (time: number) => {
    if (audioRef.current) {
      audioRef.current.currentTime = time;
      setCurrentTime(time);
    }
  };

  useEffect(() => {
    const ids = results.map(r => r.id);
    if (ids.length === 0) {
      setShuffleOrder([]);
      setShuffleIndex(-1);
      return;
    }
    setShuffleOrder(buildShuffleOrder(ids));
  }, [results]);

  useEffect(() => {
    if (!activeMusic) {
      setShuffleIndex(-1);
      return;
    }
    const index = shuffleOrder.indexOf(activeMusic.id);
    setShuffleIndex(index);
  }, [activeMusic, shuffleOrder]);

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.volume = volume;
    }
  }, [volume]);

  useEffect(() => {
    const env = (window as Window & { __COCO_ENV?: { ENABLE_DOWNLOAD?: string } }).__COCO_ENV;
    if (env?.ENABLE_DOWNLOAD === "0") {
      setDownloadEnabled(false);
      return;
    }
    if (env?.ENABLE_DOWNLOAD === "1") {
      setDownloadEnabled(true);
    }
  }, []);

  const listGridTemplate = downloadEnabled
    ? "grid-cols-[40px_1fr_40px] md:grid-cols-[50px_2fr_1.5fr_80px_120px]"
    : "grid-cols-[1fr_40px] md:grid-cols-[2fr_1.5fr_80px_80px]";

  const executeDownload = async (task: DownloadTask) => {
    try {
      // Update status to downloading
      setDownloadTasks(prev => prev.map(t => 
        t.id === task.id ? { ...t, status: 'downloading' } : t
      ));

      if (!downloadEnabled) {
        await openSourceUrl(task.musicItem);
        setDownloadTasks(prev =>
          prev.map(t => (t.id === task.id ? { ...t, status: "completed", progress: 100 } : t))
        );
        return;
      }

      // 获取下载 URL
      let arrayBuffer;
      try {
        arrayBuffer = await downloadMusic(
          task.musicItem.id,
          task.fileName,
          task.musicItem.provider || 'gequbao'
        );
      } catch (err: unknown) {
        // 检查是否为网盘链接
        const errorMessage = err instanceof Error ? err.message : '';
        if (errorMessage.startsWith('CLOUD_DRIVE:')) {
          const cloudUrl = errorMessage.replace('CLOUD_DRIVE:', '');
          console.log('[Download] Opening cloud drive URL:', cloudUrl);
          
          // 在浏览器中打开网盘链接
          window.open(cloudUrl, '_blank');
          
          // 标记为完成
          setDownloadTasks(prev =>
            prev.map(t => (t.id === task.id ? { ...t, status: "completed", progress: 100 } : t))
          );
          return;
        }
        throw err; // 重新抛出其他错误
      }

      // 创建 Blob 并下载
      if (arrayBuffer) {
        const blob = new Blob([arrayBuffer]);
        const url = window.URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = task.fileName;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        window.URL.revokeObjectURL(url);
      }

      setDownloadTasks(prev => prev.map(t => 
        t.id === task.id ? { ...t, status: 'completed', progress: 100 } : t
      ));

    } catch (err: unknown) {
      console.error(err);
      const errorMessage = err instanceof Error ? err.message : 'Download failed';
      setDownloadTasks(prev => prev.map(t => 
        t.id === task.id ? { ...t, status: 'error', error: errorMessage } : t
      ));
    }
  };

  const downloadOne = async (item: MusicItem, event?: React.MouseEvent) => {
    // 显示文件大小提示（如果有）
    if (item.size) {
      showToast(`《${item.title}》文件大小：${item.size}`, event?.currentTarget as HTMLElement | undefined);
    }
    
    const taskId = `${item.id}-${Date.now()}`;
    const cleanTitle = item.title.replace(/\s+/g, ' ').trim();
    const filename = `${cleanTitle}.mp3`;

    // Add initial task
    const newTask: DownloadTask = {
      id: taskId,
      musicItem: item,
      status: 'pending',
      progress: 0,
      fileName: filename,
      startTime: Date.now()
    };

    setDownloadTasks(prev => [newTask, ...prev]);
    setIsDrawerOpen(true);
    
    // Execute immediately for single download
    await executeDownload(newTask);
  };

  const toggleSelection = (id: string) => {
    const next = new Set(selectedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setSelectedIds(next);
  };

  const toggleAll = () => {
    if (selectedIds.size === results.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(results.map(r => r.id)));
    }
  };

  const handleBatchDownload = async () => {
    const items = results.filter(r => selectedIds.has(r.id));
    if (items.length === 0) return;
    
    if (items.length > 5) {
      if (!confirm(`即将下载 ${items.length} 首歌曲，可能需要一些时间，是否继续？`)) return;
    }

    // 1. Create all tasks immediately
    const newTasks: DownloadTask[] = items.map(item => {
      const cleanTitle = item.title.replace(/\s+/g, ' ').trim();
      return {
        id: `${item.id}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        musicItem: item,
        status: 'pending',
        progress: 0,
        fileName: `${cleanTitle}.mp3`,
        startTime: Date.now()
      };
    });

    // 2. Add to state
    setDownloadTasks(prev => [...newTasks, ...prev]);
    setIsDrawerOpen(true);
    setDownloadingCount(items.length);

    // 3. Process with concurrency limit
    const CONCURRENCY_LIMIT = 3;
    const queue = [...newTasks];
    const activePromises: Promise<void>[] = [];

    const processQueue = async () => {
      while (queue.length > 0) {
        if (activePromises.length >= CONCURRENCY_LIMIT) {
          await Promise.race(activePromises);
        }
        
        const task = queue.shift();
        if (task) {
          const promise = executeDownload(task).then(() => {
            setDownloadingCount(prev => Math.max(0, prev - 1));
            // Remove self from active promises
            const index = activePromises.indexOf(promise);
            if (index > -1) activePromises.splice(index, 1);
          });
          activePromises.push(promise);
        }
      }
      // Wait for remaining
      await Promise.all(activePromises);
    };

    await processQueue();
    setDownloadingCount(0);
  };

  const currentIndex = activeMusic ? results.findIndex(r => r.id === activeMusic.id) : -1;
  const currentPlaylistIndex = activeMusic ? playlist.findIndex(p => p.id === activeMusic.id && p.provider === activeMusic.provider) : -1;
  
  // 判断当前是否在播放播放列表
  const isPlayingFromPlaylist = currentPlaylistIndex >= 0;
  
  const getNextIndex = () => {
    if (!activeMusic) return -1;
    
    // 如果正在播放播放列表，使用播放列表
    if (isPlayingFromPlaylist) {
      if (playMode === "shuffle") {
        // 随机播放：从播放列表中随机选择
        if (playlist.length <= 1) return -1;
        let randomIndex;
        do {
          randomIndex = Math.floor(Math.random() * playlist.length);
        } while (randomIndex === currentPlaylistIndex);
        return randomIndex;
      }
      // 顺序播放
      if (currentPlaylistIndex >= 0 && currentPlaylistIndex < playlist.length - 1) {
        return currentPlaylistIndex + 1;
      }
      return -1;
    }
    
    // 否则使用搜索结果列表
    if (playMode === "shuffle") {
      if (shuffleIndex >= 0 && shuffleIndex < shuffleOrder.length - 1) {
        const nextId = shuffleOrder[shuffleIndex + 1];
        return results.findIndex(r => r.id === nextId);
      }
      return -1;
    }
    if (currentIndex >= 0 && currentIndex < results.length - 1) {
      return currentIndex + 1;
    }
    return -1;
  };

  const getPrevIndex = () => {
    if (!activeMusic) return -1;
    
    // 如果正在播放播放列表，使用播放列表
    if (isPlayingFromPlaylist) {
      if (playMode === "shuffle") {
        // 随机播放：从播放列表中随机选择
        if (playlist.length <= 1) return -1;
        let randomIndex;
        do {
          randomIndex = Math.floor(Math.random() * playlist.length);
        } while (randomIndex === currentPlaylistIndex);
        return randomIndex;
      }
      // 顺序播放
      if (currentPlaylistIndex > 0) {
        return currentPlaylistIndex - 1;
      }
      return -1;
    }
    
    // 否则使用搜索结果列表
    if (playMode === "shuffle") {
      if (shuffleIndex > 0) {
        const prevId = shuffleOrder[shuffleIndex - 1];
        return results.findIndex(r => r.id === prevId);
      }
      return -1;
    }
    if (currentIndex > 0) {
      return currentIndex - 1;
    }
    return -1;
  };

  const canNext = getNextIndex() >= 0;
  const canPrev = getPrevIndex() >= 0;

  const handleNext = () => {
    // 防止并发播放请求
    if (isPlaybackLockedRef.current) {
      console.log('Playback locked, ignoring next request');
      return;
    }
    
    const nextIndex = getNextIndex();
    if (nextIndex >= 0) {
      // 根据播放来源选择正确的列表
      if (isPlayingFromPlaylist) {
        handlePlay(playlist[nextIndex]);
      } else {
        handlePlay(results[nextIndex]);
      }
    }
  };
  
  const handlePrev = () => {
    // 防止并发播放请求
    if (isPlaybackLockedRef.current) {
      console.log('Playback locked, ignoring prev request');
      return;
    }
    
    const prevIndex = getPrevIndex();
    if (prevIndex >= 0) {
      // 根据播放来源选择正确的列表
      if (isPlayingFromPlaylist) {
        handlePlay(playlist[prevIndex]);
      } else {
        handlePlay(results[prevIndex]);
      }
    }
  };

  const togglePlayMode = () => {
    setPlayMode(prev => {
      if (prev === "order") return "shuffle";
      if (prev === "shuffle") return "single";
      return "order";
    });
  };

  // Playlist Functions
  const addToPlaylist = async (item: MusicItem, event?: React.MouseEvent) => {
    // 检查歌曲是否支持播放
    try {
      const { downloadOnly } = await getMusicUrl(item.id, item.provider || 'gequbao');
      
      if (downloadOnly) {
        const targetElement = event?.currentTarget as HTMLElement | undefined;
        showToast(`《${item.title}》仅支持下载，不支持播放`, targetElement);
        return; // 拒绝加入播放列表
      }
    } catch (error) {
      console.error('Failed to check music URL:', error);
    }
    
    // 支持播放，加入播放列表
    setPlaylist(prev => {
      const exists = prev.find(p => p.id === item.id && p.provider === item.provider);
      if (exists) {
        return prev;
      }
      return [...prev, item];
    });
  };

  const removeFromPlaylist = (id: string, provider: string) => {
    setPlaylist(prev => prev.filter(p => !(p.id === id && p.provider === provider)));
  };

  const clearPlaylist = () => {
    setPlaylist([]);
  };

  // 播放列表持久化：每当 playlist 变化时保存到 localStorage
  useEffect(() => {
    try {
      localStorage.setItem('maco-playlist', JSON.stringify(playlist));
    } catch (error) {
      console.error('Failed to save playlist to localStorage:', error);
    }
  }, [playlist]);

  const playFromPlaylist = (item: MusicItem) => {
    handlePlay(item);
  };

  const movePlaylistItemUp = (index: number) => {
    if (index === 0) return;
    setPlaylist(prev => {
      const newPlaylist = [...prev];
      [newPlaylist[index - 1], newPlaylist[index]] = [newPlaylist[index], newPlaylist[index - 1]];
      return newPlaylist;
    });
  };

  const movePlaylistItemDown = (index: number) => {
    if (index === playlist.length - 1) return;
    setPlaylist(prev => {
      const newPlaylist = [...prev];
      [newPlaylist[index], newPlaylist[index + 1]] = [newPlaylist[index + 1], newPlaylist[index]];
      return newPlaylist;
    });
  };

  useEffect(() => {
    if (!audioRef.current) {
      audioRef.current = new Audio();
    }
    const audio = audioRef.current;

    const handleTimeUpdate = () => setCurrentTime(audio.currentTime);
    const handleLoadedMetadata = () => {
      const audioDuration = audio.duration;
      setDuration(audioDuration);
      
      // 将获取到的时长更新到搜索结果中
      if (activeMusic && !activeMusic.duration && Number.isFinite(audioDuration)) {
        const minutes = Math.floor(audioDuration / 60);
        const seconds = Math.floor(audioDuration % 60);
        const durationStr = `${minutes}:${seconds.toString().padStart(2, '0')}`;
        
        // 更新 results 中的 duration
        setResults(prev => prev.map(item => 
          (item.id === activeMusic.id && item.provider === activeMusic.provider)
            ? { ...item, duration: durationStr }
            : item
        ));
        
        // 更新 playlist 中的 duration
        setPlaylist(prev => prev.map(item => 
          (item.id === activeMusic.id && item.provider === activeMusic.provider)
            ? { ...item, duration: durationStr }
            : item
        ));
      }
      
      if (playing) audio.play().catch(() => setPlaying(false));
    };
    const handleEnded = () => {
      if (playMode === "single") {
        if (audioRef.current) {
          audioRef.current.currentTime = 0;
          audioRef.current.play()
            .then(() => setPlaying(true))
            .catch(() => setPlaying(false));
        }
        return;
      }
      
      // 防止并发播放请求
      if (isPlaybackLockedRef.current) {
        console.log('Playback locked, ignoring ended event');
        return;
      }
      
      const nextIndex = getNextIndex();
      if (nextIndex >= 0) {
        // 根据播放来源选择正确的列表
        if (isPlayingFromPlaylist) {
          handlePlay(playlist[nextIndex]);
        } else {
          handlePlay(results[nextIndex]);
        }
      } else {
        setPlaying(false);
      }
    };

    audio.addEventListener('timeupdate', handleTimeUpdate);
    audio.addEventListener('loadedmetadata', handleLoadedMetadata);
    audio.addEventListener('ended', handleEnded);

    return () => {
      audio.removeEventListener('timeupdate', handleTimeUpdate);
      audio.removeEventListener('loadedmetadata', handleLoadedMetadata);
      audio.removeEventListener('ended', handleEnded);
    };
  }, [playing, playMode, results, playlist, activeMusic, shuffleIndex, shuffleOrder, isPlayingFromPlaylist, handlePlay]);

  return (
    <main className="min-h-[calc(100vh-64px)] bg-slate-50 dark:bg-slate-950 text-slate-800 dark:text-slate-100 font-sans selection:bg-sky-100 dark:selection:bg-sky-900 pb-32 pt-20 transition-colors duration-300">
      <div className="container mx-auto px-4 py-12 flex flex-col items-center">
        
        {/* Header Area */}
        <motion.div 
          layout
          className={cn(
            "flex flex-col items-center justify-center transition-all duration-500 w-full",
            searched ? "mt-0 mb-8" : "mt-[10vh] mb-12"
          )}
        >
          <div className="flex items-center gap-3 mb-4">
             <span className="px-3 py-1 rounded-full bg-sky-100 dark:bg-sky-900 text-sky-600 dark:text-sky-300 text-xs font-bold tracking-wider uppercase">
               V1.0 BETA
             </span>
          </div>
          <h1 className="text-4xl md:text-6xl font-bold text-slate-800 dark:text-slate-100 tracking-tight mb-4 text-center">
            Maco 在线音乐
          </h1>
          <p className="text-slate-500 dark:text-slate-400 text-lg mb-8 max-w-lg text-center leading-relaxed hidden md:block">
            您的专属高品质音乐获取助手，支持多平台搜索，
            <br />
            极速解析，批量下载，纯净无广。
          </p>
          
          {/* Provider Selector */}
          <div className="flex items-center gap-2 mb-3 text-sm font-medium text-slate-500 dark:text-slate-400">
             <Music className="w-4 h-4" />
             <span>选择搜索来源:</span>
          </div>
          <div className="flex justify-center mb-6 gap-3 flex-wrap">
            {[
              // 暂时只显示已实现的provider
              { id: 'bugu', name: '布谷' },
              { id: 'qq', name: 'QQ音乐' },
              { id: 'jianbin-netease', name: '煎饼-网易' },
              { id: 'jianbin-qq', name: '煎饼-qq' },
              { id: 'jianbin-kugou', name: '煎饼-酷狗' },
              { id: 'jianbin-kuwo', name: '煎饼-酷我' },
              // { id: 'gequbao', name: '歌曲宝' }, // 暂时隐藏
              // { id: 'gequhai', name: '歌曲海' }, // 暂时隐藏
              { id: 'qqmp3', name: 'QQMP3' },
              // { id: 'migu', name: '咪咕' }, // 暂时隐藏（版权限制201007）
              { id: 'livepoo', name: '力音' },
            ].map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => setProvider(p.id)}
                className={cn(
                  "px-4 py-2 rounded-full text-sm font-medium transition-all duration-300 cursor-pointer",
                  provider === p.id 
                    ? "bg-sky-500 text-white shadow-lg shadow-sky-200 dark:shadow-none ring-2 ring-sky-200 dark:ring-sky-800 ring-offset-2 dark:ring-offset-slate-900" 
                    : "bg-white dark:bg-slate-900 text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 border border-slate-100 dark:border-slate-800 hover:border-sky-200 dark:hover:border-sky-700"
                )}
              >
                {p.name}
              </button>
            ))}
          </div>
          
          {/* Search Bar */}
          <div className="flex items-center justify-center gap-3 mb-6">
            <form onSubmit={handleSearch} className="relative w-full max-w-xl group">
              <div className="absolute inset-0 bg-sky-200 dark:bg-sky-900 rounded-full blur-xl opacity-30 group-hover:opacity-50 transition-opacity duration-300"></div>
              <div className="relative bg-white dark:bg-slate-900 shadow-xl shadow-slate-200/50 dark:shadow-none rounded-full flex items-center p-2 pr-2 border border-slate-100 dark:border-slate-800 transition-transform duration-300 hover:scale-[1.01]">
                <Search className="w-6 h-6 text-slate-400 dark:text-slate-500 ml-4" />
                <input
                  type="text"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="搜索歌曲、歌手..."
                  className="flex-1 bg-transparent border-none outline-none px-4 text-lg text-slate-700 dark:text-slate-200 placeholder:text-slate-300 dark:placeholder:text-slate-600 h-12"
                />
                <button
                  type="submit"
                  disabled={loading}
                  className="bg-sky-500 hover:bg-sky-600 text-white rounded-full px-8 h-12 font-medium transition-all active:scale-95 disabled:opacity-70 flex items-center gap-2 cursor-pointer"
                >
                  {loading ? <Loader2 className="w-5 h-5 animate-spin" /> : "搜索"}
                </button>
              </div>
            </form>
            {/* 随便听听按钮 - 放在搜索框右侧 */}
            <button
              type="button"
              onClick={handleRandomListen}
              disabled={randomLoading}
              className="flex-shrink-0 bg-gradient-to-r from-purple-500 to-pink-500 hover:from-purple-600 hover:to-pink-600 text-white rounded-full px-5 h-12 font-medium transition-all active:scale-95 disabled:opacity-70 flex items-center gap-2 cursor-pointer shadow-lg shadow-purple-200/50 dark:shadow-none"
              title="随机发现好听的音乐"
            >
              {randomLoading ? (
                <Loader2 className="w-5 h-5 animate-spin" />
              ) : (
                <>
                  <Shuffle className="w-5 h-5" />
                  <span className="hidden sm:inline">随便听听</span>
                </>
              )}
            </button>
          </div>

          {/* Hot Tags */}
          <AnimatePresence>
            {!searched && (
              <motion.div 
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, height: 0 }}
                className="flex flex-wrap justify-center gap-3 text-sm text-slate-500 dark:text-slate-400"
              >
                <div className="flex items-center gap-1 text-slate-400 dark:text-slate-500">
                  <Flame className="w-4 h-4 text-orange-500" />
                  <span>热门搜索:</span>
                </div>
                {["周杰伦", "林俊杰", "抖音热歌", "陈奕迅", "古典音乐"].map((tag) => (
                  <span 
                    key={tag}
                    onClick={() => setQuery(tag)}
                    className="px-3 py-1 bg-white dark:bg-slate-900 border border-slate-100 dark:border-slate-800 rounded-full cursor-pointer hover:bg-sky-50 dark:hover:bg-slate-800 hover:text-sky-600 dark:hover:text-sky-400 hover:border-sky-100 dark:hover:border-sky-900 transition-colors shadow-sm dark:shadow-none"
                  >
                    {tag}
                  </span>
                ))}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>

        {/* Features Grid - Only show when not searched */}
        <AnimatePresence>
            {!searched && results.length === 0 && (
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20 }}
                transition={{ delay: 0.1 }}
                className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-4xl w-full mt-8"
              >
                 {[
                   { icon: Headphones, title: "全网聚合", desc: "支持主流音乐平台搜索，海量曲库一网打尽" },
                   { icon: Zap, title: "极速解析", desc: "毫秒级解析响应，多线程并发下载，拒绝等待" },
                   { icon: ShieldCheck, title: "纯净无广", desc: "无任何广告干扰，还原最纯粹的音乐体验" }
                 ].map((feature, i) => (
                   <div key={i} className="bg-white/50 dark:bg-slate-900/50 backdrop-blur-sm border border-slate-100 dark:border-slate-800 p-6 rounded-2xl flex flex-col items-center text-center hover:bg-white dark:hover:bg-slate-900 hover:shadow-lg hover:shadow-slate-100/50 dark:hover:shadow-none transition-all duration-300 group cursor-default">
                     <div className="w-12 h-12 bg-sky-50 dark:bg-slate-800 rounded-xl flex items-center justify-center text-sky-500 dark:text-sky-400 mb-4 group-hover:scale-110 transition-transform duration-300">
                       <feature.icon className="w-6 h-6" />
                     </div>
                     <h3 className="text-lg font-bold text-slate-800 dark:text-slate-100 mb-2">{feature.title}</h3>
                     <p className="text-slate-500 dark:text-slate-400 text-sm leading-relaxed">{feature.desc}</p>
                   </div>
                 ))}
              </motion.div>
            )}
        </AnimatePresence>

        {/* Footer Info - Only show when not searched */}
        <AnimatePresence>
          {!searched && results.length === 0 && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ delay: 0.2 }}
              className="mt-16 text-center text-slate-400 dark:text-slate-500 text-sm"
            >
              <p>© 2026 Maco Music. Powered by Next.js & React.</p>
              <p className="mt-2 text-xs text-slate-300 dark:text-slate-600">仅供个人学习交流使用，请勿用于商业用途</p>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Results List */}
        <div className="w-full max-w-4xl mx-auto flex-1">
          <AnimatePresence mode="wait">
            {loading ? (
              <motion.div 
                key="loading"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="flex flex-col items-center justify-center py-20 text-slate-400 dark:text-slate-500"
              >
                <Loader2 className="w-10 h-10 animate-spin mb-4 text-sky-400" />
                <p>正在寻找动听旋律...</p>
              </motion.div>
            ) : results.length > 0 ? (
              <motion.div 
                key="results"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0 }}
                className="bg-white dark:bg-slate-900 rounded-2xl shadow-sm border border-slate-100 dark:border-slate-800 overflow-hidden mb-24"
              >
                {/* List Header */}
                <div
                  className={cn(
                    "grid gap-4 p-4 border-b border-slate-50 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-800/50 text-sm font-medium text-slate-500 dark:text-slate-400",
                    listGridTemplate
                  )}
                >
                  {downloadEnabled ? (
                    <div className="flex justify-center items-center">
                      <button 
                        onClick={toggleAll}
                        className={cn(
                          "w-5 h-5 rounded border flex items-center justify-center transition-colors cursor-pointer",
                          selectedIds.size === results.length && results.length > 0
                            ? "bg-sky-500 border-sky-500 text-white" 
                            : "border-slate-300 dark:border-slate-600 hover:border-sky-400 dark:hover:border-sky-500"
                        )}
                      >
                        {selectedIds.size === results.length && results.length > 0 && <Check className="w-3.5 h-3.5" />}
                      </button>
                    </div>
                  ) : null}
                  <div>歌曲</div>
                  <div className="hidden md:block">歌手</div>
                  <div className="hidden md:block text-center">时长</div>
                  <div className="text-right pr-4 md:pr-4">操作</div>
                </div>

                {/* Toast 提示 */}
                <AnimatePresence>
                  {toastMessage && (
                    <motion.div
                      initial={{ opacity: 0, y: -10 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: -10 }}
                      className="fixed z-50 p-3 bg-amber-50 dark:bg-amber-900/30 border border-amber-200 dark:border-amber-700 rounded-lg text-amber-800 dark:text-amber-200 text-sm shadow-lg pointer-events-none"
                      style={toastPosition ? {
                        top: `${toastPosition.top - 50}px`,
                        left: `${toastPosition.left}px`,
                        transform: 'translateX(-50%)'
                      } : {
                        top: '50%',
                        left: '50%',
                        transform: 'translate(-50%, -50%)'
                      }}
                    >
                      {toastMessage}
                    </motion.div>
                  )}
                </AnimatePresence>

                {/* List Items */}
                <div className="divide-y divide-slate-50 dark:divide-slate-800">
                  {results.map((item) => {
                    const isActive = activeMusic?.id === item.id;
                    const isSelected = selectedIds.has(item.id);
                    
                    return (
                      <motion.div 
                        key={item.id}
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        onDoubleClick={(e) => handlePlay(item, e)}
                        className={cn(
                          "grid gap-4 p-4 items-center hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-all duration-200 group cursor-pointer select-none active:scale-[0.99] rounded-xl",
                          listGridTemplate,
                          isActive && "bg-sky-50/50 dark:bg-sky-900/20"
                        )}
                      >
                        {downloadEnabled ? (
                          <div className="flex justify-center items-center">
                            <button 
                              onClick={(e) => { e.stopPropagation(); toggleSelection(item.id); }}
                              className={cn(
                                "w-5 h-5 rounded border flex items-center justify-center transition-colors cursor-pointer",
                                isSelected 
                                  ? "bg-sky-500 border-sky-500 text-white" 
                                  : "border-slate-300 dark:border-slate-600 hover:border-sky-400 dark:hover:border-sky-500"
                              )}
                            >
                              {isSelected && <Check className="w-3.5 h-3.5" />}
                            </button>
                          </div>
                        ) : null}

                        <div className="flex items-center gap-3 overflow-hidden">
                          <div 
                            onClick={(e) => { e.stopPropagation(); handlePlay(item, e); }}
                            className="w-10 h-10 rounded-lg bg-slate-100 dark:bg-slate-800 overflow-hidden flex-shrink-0 cursor-pointer relative group/cover"
                          >
                            {item.cover ? (
                              <Image
                                src={item.cover}
                                alt={item.title}
                                fill
                                sizes="40px"
                                className="object-cover"
                                unoptimized
                              />
                            ) : (
                              <div className="w-full h-full flex items-center justify-center text-slate-400 dark:text-slate-500">
                                <Music className="w-5 h-5" />
                              </div>
                            )}
                            <div className={cn(
                              "absolute inset-0 bg-black/20 flex items-center justify-center transition-opacity",
                              isActive ? "opacity-100" : "opacity-0 group-hover/cover:opacity-100"
                            )}>
                              {isActive && playing ? (
                                <Pause className="w-4 h-4 text-white fill-current" />
                              ) : (
                                <Play className="w-4 h-4 text-white fill-current" />
                              )}
                            </div>
                          </div>
                          <div className="flex flex-col min-w-0 overflow-hidden">
                            <span className={cn(
                              "font-medium truncate",
                              isActive ? "text-sky-600 dark:text-sky-400" : "text-slate-700 dark:text-slate-200"
                            )}>
                              {item.title}
                            </span>
                            <span className="text-xs text-slate-400 dark:text-slate-500 truncate md:hidden block mt-0.5">
                              {item.artist}
                            </span>
                          </div>
                        </div>

                        <div className="text-slate-500 dark:text-slate-400 truncate text-sm hidden md:block">
                          {item.artist}
                        </div>

                        <div className="text-slate-500 dark:text-slate-400 text-sm text-center hidden md:block">
                          {item.duration || '-'}
                        </div>

                        <div className="flex justify-end pr-2 md:pr-2 gap-2">
                          <button
                            onClick={(e) => { e.stopPropagation(); handlePlay(item, e); }}
                            className="p-2 text-slate-400 dark:text-slate-500 hover:text-sky-500 dark:hover:text-sky-400 hover:bg-sky-50 dark:hover:bg-slate-800 rounded-full transition-colors cursor-pointer"
                            title="播放"
                          >
                            <Play className="w-5 h-5" />
                          </button>
                          <button
                            onClick={(e) => { e.stopPropagation(); addToPlaylist(item, e); }}
                            className="p-2 text-slate-400 dark:text-slate-500 hover:text-green-500 dark:hover:text-green-400 hover:bg-green-50 dark:hover:bg-slate-800 rounded-full transition-colors cursor-pointer"
                            title="添加到播放列表"
                          >
                            <ListPlus className="w-5 h-5" />
                          </button>
                          {/* <SourceLinkButton item={item} /> */}
                          {downloadEnabled ? (
                            <button
                              onClick={(e) => { e.stopPropagation(); downloadOne(item, e); }}
                              className="p-2 text-slate-400 dark:text-slate-500 hover:text-sky-500 dark:hover:text-sky-400 hover:bg-sky-50 dark:hover:bg-slate-800 rounded-full transition-colors cursor-pointer"
                              title="下载"
                            >
                              <Download className="w-5 h-5" />
                            </button>
                          ) : null}
                        </div>
                      </motion.div>
                    );
                  })}
                </div>
              </motion.div>
            ) : searched && !isRandomListen ? (
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="text-center py-20 text-slate-400 dark:text-slate-500"
              >
                <p>未找到相关歌曲，换个关键词试试？</p>
              </motion.div>
            ) : null}
          </AnimatePresence>
        </div>
      </div>

      {downloadEnabled ? (
        <DownloadDrawer
          isOpen={isDrawerOpen}
          onClose={() => setIsDrawerOpen(false)}
          tasks={downloadTasks}
          onRemoveTask={(taskId) => setDownloadTasks(prev => prev.filter(t => t.id !== taskId))}
          onClearCompleted={() => setDownloadTasks(prev => prev.filter(t => t.status === 'downloading' || t.status === 'pending'))}
        />
      ) : null}

      {/* Floating Download Toggle Button (Bottom Right) */}
      <AnimatePresence>
        {downloadEnabled && !isDrawerOpen && downloadTasks.length > 0 && (
          <motion.button
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            exit={{ scale: 0 }}
            onClick={() => setIsDrawerOpen(true)}
            className="fixed bottom-24 right-6 z-[60] w-14 h-14 bg-sky-500 hover:bg-sky-600 text-white rounded-full shadow-lg shadow-sky-500/30 flex items-center justify-center transition-all active:scale-95 group"
          >
            <div className="relative">
               <Download className="w-6 h-6" />
               {downloadTasks.some(t => t.status === 'downloading') && (
                 <span className="absolute -top-1 -right-1 flex h-3 w-3">
                   <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"></span>
                   <span className="relative inline-flex rounded-full h-3 w-3 bg-red-500"></span>
                 </span>
               )}
            </div>
            {/* Tooltip */}
            <span className="absolute right-full mr-4 px-2 py-1 bg-slate-800 text-white text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
              查看下载任务
            </span>
          </motion.button>
        )}
      </AnimatePresence>

      {/* Floating Batch Action Bar */}
      <AnimatePresence>
        {downloadEnabled && selectedIds.size > 0 && (
          <motion.div
            initial={{ y: 100, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 100, opacity: 0 }}
            className="fixed bottom-40 left-0 right-0 flex justify-center z-[60] pointer-events-none"
          >
            <div className="bg-white dark:bg-slate-900 shadow-xl shadow-slate-200/50 dark:shadow-none border border-slate-100 dark:border-slate-800 rounded-full px-6 py-3 flex items-center gap-6 pointer-events-auto">
              <span className="text-sm font-medium text-slate-600 dark:text-slate-400">
                已选择 <span className="text-sky-600 dark:text-sky-400 font-bold">{selectedIds.size}</span> 首歌曲
              </span>
              
              <div className="h-4 w-px bg-slate-200 dark:bg-slate-700"></div>

              <button 
                onClick={handleBatchDownload}
                disabled={downloadingCount > 0}
                className="flex items-center gap-2 text-sky-600 dark:text-sky-400 hover:text-sky-700 dark:hover:text-sky-300 font-medium text-sm transition-colors disabled:opacity-50 cursor-pointer"
              >
                {downloadingCount > 0 ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    剩余 {downloadingCount} 首...
                  </>
                ) : (
                  <>
                    <Download className="w-4 h-4" />
                    批量下载
                  </>
                )}
              </button>

              <button 
                onClick={() => setSelectedIds(new Set())}
                className="text-slate-400 dark:text-slate-500 hover:text-red-500 dark:hover:text-red-400 transition-colors cursor-pointer"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Player Bar */}
      <AnimatePresence>
        {activeMusic && (
          <PlayerBar 
            currentMusic={activeMusic}
            isPlaying={playing}
            onPlayPause={() => {
              if (playing) {
                audioRef.current?.pause();
                setPlaying(false);
              } else {
                audioRef.current?.play();
                setPlaying(true);
              }
            }}
            onNext={canNext ? handleNext : undefined}
            onPrev={canPrev ? handlePrev : undefined}
            playMode={playMode}
            onTogglePlayMode={togglePlayMode}
            currentTime={currentTime}
            duration={duration}
            onSeek={handleSeek}
            volume={volume}
            onVolumeChange={setVolume}
            onOpenPlaylist={() => setIsPlaylistOpen(true)}
            playlistCount={playlist.length}
            onToggleLyrics={() => setIsLyricsOpen(!isLyricsOpen)}
          />
        )}
      </AnimatePresence>

      {/* Playlist Drawer */}
      <PlaylistDrawer
        isOpen={isPlaylistOpen}
        onClose={() => setIsPlaylistOpen(false)}
        playlist={playlist}
        currentMusic={activeMusic}
        isPlaying={playing}
        onPlay={playFromPlaylist}
        onRemove={removeFromPlaylist}
        onClear={clearPlaylist}
        onMoveUp={movePlaylistItemUp}
        onMoveDown={movePlaylistItemDown}
      />

      {/* Lyrics Panel */}
      <LyricsPanel
        isOpen={isLyricsOpen}
        onClose={() => setIsLyricsOpen(false)}
        currentMusic={activeMusic}
        currentTime={currentTime}
        isPlaying={playing}
        onSeek={handleSeek}
      />

      {/* Update Checker */}
      <UpdateChecker />
    </main>
  );
}
