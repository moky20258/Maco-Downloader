"use client";

import { X, Music } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";
import { MusicItem } from "@/types/music";
import { useEffect, useRef, useState } from "react";
import { getLyrics } from "@/lib/tauri-api";
import { useTheme } from "next-themes";

interface LyricLine {
  time: number; // 秒
  text: string;
}

interface LyricsPanelProps {
  isOpen: boolean;
  onClose: () => void;
  currentMusic: MusicItem | null;
  currentTime: number;
  isPlaying: boolean;
  onSeek?: (time: number) => void; // 点击歌词跳转
}

// LRC格式歌词解析器
export const parseLrcLyrics = (lrcText: string): LyricLine[] => {
  console.log('[Parse LRC] Input length:', lrcText.length);
  console.log('[Parse LRC] First 200 chars:', lrcText.substring(0, 200));
  
  const lines: LyricLine[] = [];
  const lineRegex = /\[([\d:.]+)\](.*)/g;
  let match;

  while ((match = lineRegex.exec(lrcText)) !== null) {
    const timeStr = match[1];
    const text = match[2].trim();
    
    if (!text) continue;

    // 解析时间 [mm:ss.xx] 或 [mm:ss]
    const timeParts = timeStr.split(':');
    const minutes = parseInt(timeParts[0]);
    const seconds = parseFloat(timeParts[1]);
    const time = minutes * 60 + seconds;

    lines.push({ time, text });
  }

  console.log('[Parse LRC] Parsed lines:', lines.length);
  
  // 按时间排序
  lines.sort((a, b) => a.time - b.time);
  return lines;
};

// 模拟歌词数据 - 实际应该从API获取
const generateMockLyrics = (title: string, duration: number): LyricLine[] => {
  const lyrics: LyricLine[] = [];
  const lines = [
    { text: "♪ 音乐加载中...", time: 0 },
    { text: "♪ ♪ ♪", time: 5 },
    { text: "《" + title + "》", time: 10 },
    { text: "正在播放", time: 15 },
    { text: "♪", time: 20 },
    { text: "（暂无歌词）", time: 25 },
    { text: "歌词功能开发中...", time: 30 },
    { text: "敬请期待", time: 35 },
    { text: "♪ ♪ ♪", time: 40 },
    { text: "音乐是最好的语言", time: 45 },
    { text: "♪", time: 50 },
    { text: "享受此刻的旋律", time: 55 },
    { text: "♪ ♪", time: 60 },
  ];

  // 根据实际时长扩展歌词
  const durationSeconds = Math.floor(duration);
  let currentTime = 0;
  
  for (let i = 0; currentTime < durationSeconds; i++) {
    const line = lines[i % lines.length];
    lyrics.push({
      time: currentTime,
      text: line.text,
    });
    currentTime += Math.floor(durationSeconds / lines.length) + 2;
  }

  return lyrics;
};

export function LyricsPanel({
  isOpen,
  onClose,
  currentMusic,
  currentTime,
  isPlaying,
  onSeek,
}: LyricsPanelProps) {
  const [lyrics, setLyrics] = useState<LyricLine[]>([]);
  const [activeLineIndex, setActiveLineIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const lyricsContainerRef = useRef<HTMLDivElement>(null);
  const isUserScrollingRef = useRef(false);
  const scrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  const { theme } = useTheme();
  const isDark = theme !== 'light'; // 默认深色主题

  // 当歌曲切换时加载歌词
  useEffect(() => {
    if (currentMusic) {
      loadLyrics();
    }
  }, [currentMusic]);

  // 加载歌词(从歌曲源搜索)
  const loadLyrics = async () => {
    if (!currentMusic) return;
    
    setLoading(true);
    setError(null);
    
    try {
      // 从Tauri后端获取歌词
      console.log('[Lyrics] Loading lyrics for:', currentMusic.title, '-', currentMusic.artist);
      const { lyrics, hasLyrics } = await getLyrics(
        currentMusic.id,
        currentMusic.provider || 'gequbao',
        currentMusic.title,
        currentMusic.artist
      );
      
      console.log('[Lyrics] API response:', { hasLyrics, lyricsLength: lyrics?.length });
      
      if (hasLyrics && lyrics) {
        // 解析LRC格式歌词
        const parsedLyrics = parseLrcLyrics(lyrics);
        console.log('[Lyrics] Parsed lyrics count:', parsedLyrics.length);
        
        if (parsedLyrics.length > 0) {
          setLyrics(parsedLyrics);
          setLoading(false);
          return;
        }
      }
      
      // 无歌词
      console.log('[Lyrics] No lyrics found');
      setLyrics([]);
    } catch (err) {
      console.error('[Lyrics] Failed to load lyrics:', err);
      setError('歌词加载失败');
    } finally {
      setLoading(false);
    }
  };

  // 更新当前活跃的歌词行
  useEffect(() => {
    if (lyrics.length === 0 || isUserScrollingRef.current) return;

    let activeIndex = 0;
    for (let i = lyrics.length - 1; i >= 0; i--) {
      if (currentTime >= lyrics[i].time) {
        activeIndex = i;
        break;
      }
    }
    
    console.log('[Lyrics] Current time:', currentTime, 'Active index:', activeIndex, 'Total lines:', lyrics.length);
    setActiveLineIndex(activeIndex);
  }, [currentTime, lyrics]);

  // 滚动到当前歌词行
  useEffect(() => {
    console.log('[Lyrics Scroll Effect] Triggered! activeLineIndex:', activeLineIndex);
    console.log('[Lyrics Scroll Effect] Container exists:', !!lyricsContainerRef.current);
    console.log('[Lyrics Scroll Effect] Is user scrolling:', isUserScrollingRef.current);
    
    if (lyricsContainerRef.current && !isUserScrollingRef.current && activeLineIndex >= 0) {
      const container = lyricsContainerRef.current;
      console.log('[Lyrics Scroll Effect] Container children count:', container.children.length);
      
      // 歌词被包裹在一个div中,需要找到这个div
      const lyricsWrapper = container.firstElementChild;
      if (!lyricsWrapper) {
        console.log('[Lyrics Scroll] No lyrics wrapper found');
        return;
      }
      
      console.log('[Lyrics Scroll Effect] Wrapper children count:', lyricsWrapper.children.length);
      
      // 在wrapper中查找歌词元素
      const activeElement = lyricsWrapper.children[activeLineIndex] as HTMLElement;
      
      if (activeElement) {
        const containerHeight = container.clientHeight;
        // offsetTop是相对于wrapper的位置,wrapper有py-[20vh]的padding
        const elementTop = activeElement.offsetTop;
        const elementHeight = activeElement.clientHeight;
        
        // 计算滚动位置，使当前行居中
        // 需要考虑wrapper的padding-top (20vh)
        const wrapperPadding = containerHeight * 0.2; // 20vh大约是容器高度的20%
        // 额外向下调整一行的位置(约40px)
        const scrollPosition = elementTop - containerHeight / 2 + elementHeight / 2 - wrapperPadding - 40;
        
        // 检查是否需要滚动（差异大于50px才滚动）
        const currentScroll = container.scrollTop;
        console.log('[Lyrics Scroll] Current:', currentScroll, 'Target:', scrollPosition, 'Diff:', Math.abs(currentScroll - scrollPosition));
        
        if (Math.abs(currentScroll - scrollPosition) > 50) {
          console.log('[Lyrics Scroll] Scrolling to:', scrollPosition);
          container.scrollTo({
            top: scrollPosition,
            behavior: "smooth",
          });
        }
      } else {
        console.log('[Lyrics Scroll] Active element not found at index:', activeLineIndex);
      }
    } else {
      console.log('[Lyrics Scroll] Conditions not met');
    }
  }, [activeLineIndex]);

  // 处理用户滚动
  const handleScroll = () => {
    isUserScrollingRef.current = true;
    
    // 清除之前的定时器
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
    }
    
    // 3秒后恢复自动滚动
    scrollTimeoutRef.current = setTimeout(() => {
      isUserScrollingRef.current = false;
    }, 3000);
  };

  // 点击歌词跳转
  const handleLyricClick = (time: number) => {
    if (onSeek) {
      onSeek(time);
    }
  };

  // 组件卸载时清理定时器
  useEffect(() => {
    return () => {
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
    };
  }, []);

  if (!isOpen || !currentMusic) return null;

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ opacity: 0, y: "100%" }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: "100%" }}
          transition={{ type: "spring", damping: 25, stiffness: 200 }}
          className={cn(
            "fixed inset-x-0 top-16 bottom-20 z-[40] backdrop-blur-3xl",
            isDark 
              ? "bg-gradient-to-b from-slate-900 via-slate-900 to-black"
              : "bg-gradient-to-b from-slate-50 via-white to-slate-100"
          )}
        >
          {/* 背景模糊效果 */}
          <div className={cn(
            "absolute inset-0 backdrop-blur-3xl",
            isDark ? "bg-black/40" : "bg-white/40"
          )} />

          {/* 关闭按钮 */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onClose();
            }}
            className={cn(
              "absolute top-6 right-6 z-50 p-2 rounded-full transition-all active:scale-95",
              isDark 
                ? "bg-white/10 hover:bg-white/20 text-white"
                : "bg-black/10 hover:bg-black/20 text-slate-800"
            )}
          >
            <X className="w-6 h-6" />
          </button>

          {/* 内容区域 */}
          <div className="relative z-10 h-full flex flex-col items-center justify-center px-8">
            {/* 歌曲信息 */}
            <motion.div
              initial={{ opacity: 0, y: -20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="text-center mb-12"
            >
              <div className="flex items-center justify-center gap-3 mb-4">
                <Music className={cn(
                  "w-6 h-6",
                  isDark ? "text-sky-400" : "text-sky-600"
                )} />
                <h2 className={cn(
                  "text-3xl md:text-4xl font-bold",
                  isDark ? "text-white" : "text-slate-900"
                )}>
                  {currentMusic.title}
                </h2>
              </div>
              <p className={cn(
                "text-lg",
                isDark ? "text-slate-400" : "text-slate-600"
              )}>{currentMusic.artist}</p>
            </motion.div>

            {/* 歌词显示区域 */}
            <div
              ref={lyricsContainerRef}
              onScroll={handleScroll}
              className="w-full max-w-2xl h-[50vh] overflow-y-auto scrollbar-hide scroll-smooth"
              style={{
                scrollbarWidth: 'none',
                msOverflowStyle: 'none',
              }}
            >
              {loading ? (
                <div className={cn(
                  "flex flex-col items-center justify-center h-full",
                  isDark ? "text-slate-400" : "text-slate-600"
                )}>
                  <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-400 mb-4"></div>
                  <p>正在加载歌词...</p>
                </div>
              ) : error ? (
                <div className={cn(
                  "flex flex-col items-center justify-center h-full",
                  isDark ? "text-slate-400" : "text-slate-600"
                )}>
                  <p className="text-lg mb-2">{error}</p>
                  <button 
                    onClick={loadLyrics}
                    className="px-4 py-2 bg-sky-500 text-white rounded-lg hover:bg-sky-600 transition-colors"
                  >
                    重试
                  </button>
                </div>
              ) : lyrics.length === 0 ? (
                <div className={cn(
                  "flex flex-col items-center justify-center h-full",
                  isDark ? "text-slate-400" : "text-slate-600"
                )}>
                  <Music className="w-16 h-16 mb-4 opacity-50" />
                  <p className="text-lg mb-2">暂无歌词</p>
                  <p className={cn(
                    "text-sm",
                    isDark ? "text-slate-500" : "text-slate-500"
                  )}>纯音乐亦是一种享受</p>
                </div>
              ) : (
                <div className="flex flex-col items-center gap-6 py-[20vh]">
                  {lyrics.map((line, index) => {
                    const isActive = index === activeLineIndex;
                    const isNear = Math.abs(index - activeLineIndex) <= 2;

                    return (
                      <motion.p
                        key={index}
                        initial={{ opacity: 0 }}
                        animate={{ 
                          opacity: isNear ? 1 : 0.3,
                          scale: isActive ? 1.05 : 1,
                        }}
                        transition={{ duration: 0.3 }}
                        className={cn(
                          "text-center transition-all duration-300 cursor-pointer",
                          isActive
                            ? cn(
                                "text-2xl md:text-3xl font-bold",
                                isDark ? "text-sky-400" : "text-sky-600"
                              )
                            : cn(
                                "text-lg md:text-xl",
                                isDark ? "text-slate-300 hover:text-sky-400" : "text-slate-700 hover:text-sky-600"
                              )
                        )}
                        onClick={() => handleLyricClick(line.time)}
                        title={`点击跳转到 ${Math.floor(line.time / 60)}:${Math.floor(line.time % 60).toString().padStart(2, '0')}`}
                      >
                        {line.text}
                      </motion.p>
                    );
                  })}
                </div>
              )}
            </div>

            {/* 提示文字 */}
            {!loading && !error && lyrics.length > 0 && (
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: 0.5 }}
                className="mt-8 text-center text-sm text-slate-500"
              >
                <p>点击歌词可跳转到对应位置</p>
                <p className="mt-1">滚动查看歌词，3秒后自动恢复同步</p>
              </motion.div>
            )}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
