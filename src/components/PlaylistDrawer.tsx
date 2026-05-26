"use client";

import Image from "next/image";
import { Play, Pause, X, Trash2, ListMusic, ChevronUp, ChevronDown } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";
import { MusicItem } from "@/types/music";

// 获取音乐源中文名称
const getProviderName = (provider: string): string => {
  const providerMap: Record<string, string> = {
    'bugu': '布谷',
    'qq': 'QQ',
    'jianbin-netease': '网易',
    'jianbin-qq': 'QQ',
    'jianbin-kugou': '酷狗',
    'jianbin-kuwo': '酷我',
    'gequbao': '歌曲宝',
    'gequhai': '歌曲海',
    'qqmp3': 'QQMP3',
    'migu': '咪咕',
    'livepoo': '力音',
  };
  return providerMap[provider] || provider;
};

interface PlaylistDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  playlist: MusicItem[];
  currentMusic: MusicItem | null;
  isPlaying: boolean;
  onPlay: (item: MusicItem) => void;
  onRemove: (id: string, provider: string) => void;
  onClear: () => void;
  onMoveUp: (index: number) => void;
  onMoveDown: (index: number) => void;
}

export function PlaylistDrawer({
  isOpen,
  onClose,
  playlist,
  currentMusic,
  isPlaying,
  onPlay,
  onRemove,
  onClear,
  onMoveUp,
  onMoveDown,
}: PlaylistDrawerProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="fixed inset-0 bg-black/30 dark:bg-black/50 backdrop-blur-sm z-50"
          />
          
          {/* Drawer */}
          <motion.div
            initial={{ x: "100%" }}
            animate={{ x: 0 }}
            exit={{ x: "100%" }}
            transition={{ type: "spring", damping: 25, stiffness: 200 }}
            className="fixed right-0 top-0 bottom-0 w-full max-w-md bg-white dark:bg-slate-900 shadow-2xl z-50 flex flex-col"
          >
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b border-slate-200 dark:border-slate-800">
              <div className="flex items-center gap-3">
                <ListMusic className="w-6 h-6 text-sky-500" />
                <div>
                  <h2 className="text-lg font-bold text-slate-800 dark:text-slate-100">播放列表</h2>
                  <p className="text-xs text-slate-500 dark:text-slate-400">
                    {playlist.length} 首歌曲
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {playlist.length > 0 && (
                  <button
                    onClick={onClear}
                    className="p-2 text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-slate-800 rounded-full transition-colors"
                    title="清空播放列表"
                  >
                    <Trash2 className="w-5 h-5" />
                  </button>
                )}
                <button
                  onClick={onClose}
                  className="p-2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 rounded-full transition-colors"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>
            </div>

            {/* Playlist Content */}
            <div className="flex-1 overflow-y-auto">
              {playlist.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full text-slate-400 dark:text-slate-500 py-20">
                  <ListMusic className="w-16 h-16 mb-4 opacity-30" />
                  <p className="text-sm">播放列表为空</p>
                  <p className="text-xs mt-2">搜索歌曲并添加到播放列表</p>
                </div>
              ) : (
                <div className="divide-y divide-slate-100 dark:divide-slate-800">
                  {playlist.map((item, index) => {
                    const isActive = currentMusic?.id === item.id && currentMusic?.provider === item.provider;
                    
                    return (
                      <motion.div
                        key={`${item.id}-${item.provider}`}
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ delay: index * 0.03 }}
                        className={cn(
                          "flex items-center gap-3 p-3 hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors cursor-pointer group",
                          isActive && "bg-sky-50 dark:bg-sky-900/20"
                        )}
                        onClick={() => onPlay(item)}
                      >
                        {/* Index / Playing Indicator */}
                        <div className="w-8 h-8 flex-shrink-0 flex items-center justify-center">
                          {isActive && isPlaying ? (
                            <Pause className="w-4 h-4 text-sky-500" />
                          ) : (
                            <span className={cn(
                              "text-sm font-medium",
                              isActive ? "text-sky-600 dark:text-sky-400" : "text-slate-400 dark:text-slate-500"
                            )}>
                              {index + 1}
                            </span>
                          )}
                        </div>

                        {/* Cover */}
                        <div className="w-10 h-10 rounded-lg bg-slate-100 dark:bg-slate-800 overflow-hidden flex-shrink-0 relative">
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
                              <ListMusic className="w-5 h-5" />
                            </div>
                          )}
                        </div>

                        {/* Info */}
                        <div className="flex-1 min-w-0">
                          <p className={cn(
                            "text-sm font-medium truncate",
                            isActive ? "text-sky-600 dark:text-sky-400" : "text-slate-700 dark:text-slate-200"
                          )}>
                            {item.title}
                          </p>
                          <div className="flex items-center gap-2">
                            <p className="text-xs text-slate-400 dark:text-slate-500 truncate">
                              {item.artist}
                            </p>
                            {/* 来源标签 */}
                            <span className="flex-shrink-0 px-1.5 py-0.5 text-[10px] font-medium rounded bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400 border border-slate-200 dark:border-slate-700">
                              {getProviderName(item.provider)}
                            </span>
                          </div>
                        </div>

                        {/* Sort Buttons */}
                        <div className="flex flex-col gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onMoveUp(index);
                            }}
                            disabled={index === 0}
                            className="p-1 text-slate-400 hover:text-sky-500 hover:bg-sky-50 dark:hover:bg-slate-800 rounded transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                            title="上移"
                          >
                            <ChevronUp className="w-4 h-4" />
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onMoveDown(index);
                            }}
                            disabled={index === playlist.length - 1}
                            className="p-1 text-slate-400 hover:text-sky-500 hover:bg-sky-50 dark:hover:bg-slate-800 rounded transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                            title="下移"
                          >
                            <ChevronDown className="w-4 h-4" />
                          </button>
                        </div>

                        {/* Actions */}
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            onRemove(item.id, item.provider);
                          }}
                          className="p-2 text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-slate-800 rounded-full transition-colors opacity-0 group-hover:opacity-100"
                          title="从播放列表移除"
                        >
                          <X className="w-4 h-4" />
                        </button>
                      </motion.div>
                    );
                  })}
                </div>
              )}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
