import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Download, CheckCircle2, AlertCircle, Trash2, Music, Loader2, FolderOpen } from 'lucide-react';
import { DownloadTask } from '@/types/download';
import { cn } from '@/lib/utils';
import Image from 'next/image';

interface DownloadDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  tasks: DownloadTask[];
  onRemoveTask: (taskId: string) => void;
  onClearCompleted: () => void;
  onOpenFolder?: () => void;
}

export function DownloadDrawer({
  isOpen,
  onClose,
  tasks,
  onRemoveTask,
  onClearCompleted,
  onOpenFolder
}: DownloadDrawerProps) {
  // Sort tasks: downloading first, then recent
  const sortedTasks = [...tasks].sort((a, b) => {
    if (a.status === 'downloading' && b.status !== 'downloading') return -1;
    if (a.status !== 'downloading' && b.status === 'downloading') return 1;
    return b.startTime - a.startTime;
  });

  const downloadingCount = tasks.filter(t => t.status === 'downloading').length;

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
            className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[60]"
          />

          {/* Drawer Panel */}
          <motion.div
            initial={{ x: '100%' }}
            animate={{ x: 0 }}
            exit={{ x: '100%' }}
            transition={{ type: 'spring', damping: 25, stiffness: 200 }}
            className="fixed right-0 top-0 bottom-0 w-full max-w-md bg-white dark:bg-slate-900 shadow-2xl z-[70] border-l border-slate-100 dark:border-slate-800 flex flex-col"
          >
            {/* Header */}
            <div className="p-4 border-b border-slate-100 dark:border-slate-800 flex items-center justify-between bg-slate-50/50 dark:bg-slate-900/50">
              <div className="flex items-center gap-3">
                <div className="relative">
                  <Download className="w-5 h-5 text-sky-500" />
                  {downloadingCount > 0 && (
                    <span className="absolute -top-1 -right-1 w-2.5 h-2.5 bg-red-500 rounded-full animate-pulse" />
                  )}
                </div>
                <div>
                  <h2 className="font-bold text-slate-800 dark:text-slate-100">下载任务</h2>
                  <p className="text-xs text-slate-400 dark:text-slate-500">
                    {downloadingCount > 0 ? `${downloadingCount} 个任务进行中` : '暂无进行中的任务'}
                  </p>
                </div>
              </div>
              <button
                onClick={onClose}
                className="p-2 hover:bg-slate-100 dark:hover:bg-slate-800 rounded-full text-slate-400 transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Task List */}
            <div className="flex-1 overflow-y-auto p-4 space-y-3">
              {tasks.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-slate-400 dark:text-slate-500 space-y-4">
                  <div className="w-16 h-16 bg-slate-50 dark:bg-slate-800 rounded-full flex items-center justify-center">
                    <Download className="w-8 h-8 text-slate-300 dark:text-slate-600" />
                  </div>
                  <p>还没有下载任务哦</p>
                </div>
              ) : (
                sortedTasks.map((task) => (
                  <div
                    key={task.id}
                    className="bg-slate-50 dark:bg-slate-800/50 rounded-xl p-3 flex gap-3 items-center group relative border border-transparent hover:border-sky-100 dark:hover:border-slate-700 transition-colors"
                  >
                    {/* Cover */}
                    <div className="w-12 h-12 rounded-lg bg-slate-200 dark:bg-slate-700 overflow-hidden flex-shrink-0 relative">
                      {task.musicItem.cover ? (
                        <Image
                          src={task.musicItem.cover}
                          alt={task.musicItem.title}
                          fill
                          className="object-cover"
                          unoptimized
                        />
                      ) : (
                        <div className="w-full h-full flex items-center justify-center">
                          <Music className="w-6 h-6 text-slate-400" />
                        </div>
                      )}
                      {/* Status Icon Overlay */}
                      <div className="absolute inset-0 bg-black/30 flex items-center justify-center">
                        {task.status === 'completed' && <CheckCircle2 className="w-5 h-5 text-green-400" />}
                        {task.status === 'error' && <AlertCircle className="w-5 h-5 text-red-400" />}
                        {task.status === 'downloading' && <Loader2 className="w-5 h-5 text-white animate-spin" />}
                      </div>
                    </div>

                    {/* Info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between mb-1">
                        <h3 className="font-medium text-sm text-slate-700 dark:text-slate-200 truncate pr-2">
                          {task.musicItem.title}
                        </h3>
                        <div className="flex items-center gap-1.5 flex-shrink-0">
                          {task.musicItem.size && task.status === 'completed' && (
                            <span className="text-[10px] text-slate-400 dark:text-slate-500 font-medium">
                              {task.musicItem.size}
                            </span>
                          )}
                          <span className={cn(
                            "text-xs font-medium",
                            task.status === 'completed' && "text-green-500",
                            task.status === 'error' && "text-red-500",
                            task.status === 'downloading' && "text-sky-500",
                            task.status === 'pending' && "text-slate-400"
                          )}>
                            {task.status === 'completed' && '已完成'}
                            {task.status === 'error' && '失败'}
                            {task.status === 'downloading' && `${Math.round(task.progress)}%`}
                            {task.status === 'pending' && '等待中'}
                          </span>
                        </div>
                      </div>
                      
                      {/* Progress Bar */}
                      <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                        <motion.div
                          initial={{ width: 0 }}
                          animate={{ width: `${task.progress}%` }}
                          transition={{ duration: 0.2 }}
                          className={cn(
                            "h-full rounded-full",
                            task.status === 'completed' ? "bg-green-500" :
                            task.status === 'error' ? "bg-red-500" :
                            "bg-sky-500"
                          )}
                        />
                      </div>
                      {task.error && (
                        <p className="text-[10px] text-red-400 mt-1 truncate">{task.error}</p>
                      )}
                    </div>

                    {/* Actions */}
                    <button
                      onClick={() => onRemoveTask(task.id)}
                      className="p-1.5 text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
                      title="删除记录"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                ))
              )}
            </div>

            {/* Footer */}
            {tasks.some(t => t.status === 'completed' || t.status === 'error') && (
              <div className="p-4 border-t border-slate-100 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 space-y-2">
                {onOpenFolder && (
                  <button
                    onClick={onOpenFolder}
                    className="w-full py-2 px-4 bg-sky-500 hover:bg-sky-600 text-white rounded-xl text-sm font-medium transition-colors flex items-center justify-center gap-2"
                  >
                    <FolderOpen className="w-4 h-4" />
                    打开下载文件夹
                  </button>
                )}
                <button
                  onClick={onClearCompleted}
                  className="w-full py-2 px-4 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors flex items-center justify-center gap-2"
                >
                  <Trash2 className="w-4 h-4" />
                  清空已完成/失败任务
                </button>
              </div>
            )}
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
