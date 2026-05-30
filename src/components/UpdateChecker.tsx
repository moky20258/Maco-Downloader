"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Download, X, AlertCircle, CheckCircle, Loader2, Copy, ExternalLink } from "lucide-react";
import { isTauri } from "@/lib/tauri-api";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";

interface GitHubRelease {
  tag_name: string;
  name: string;
  published_at: string;
  html_url: string;
  assets: Array<{
    name: string;
    browser_download_url: string;
    size: number;
  }>;
  body: string;
}

interface DownloadProgress {
  progress: number;
  downloaded: number;
  total: number;
}

export function UpdateChecker() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [checking, setChecking] = useState(false);
  const [showModal, setShowModal] = useState(false);
  const [latestRelease, setLatestRelease] = useState<GitHubRelease | null>(null);
  const [currentVersion, setCurrentVersion] = useState("0.0.0");
  const [downloadUrl, setDownloadUrl] = useState("");
  const [copied, setCopied] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadComplete, setDownloadComplete] = useState(false);

  // 获取应用当前版本号
  useEffect(() => {
    const loadVersion = async () => {
      try {
        if (isTauri()) {
          const { getVersion } = await import('@tauri-apps/api/app');
          const version = await getVersion();
          setCurrentVersion(version);
        }
      } catch (error) {
        console.error("Failed to get app version:", error);
      }
    };
    loadVersion();
  }, []);

  // 监听下载进度事件
  useEffect(() => {
    if (!isTauri()) return;

    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const unlisten = await listen('download-progress', (event: any) => {
          const data = event.payload as DownloadProgress;
          setDownloadProgress(data);
          
          if (data.progress >= 100) {
            setDownloadComplete(true);
            setDownloading(false);
          }
        });
        
        return unlisten;
      } catch (error) {
        console.error("Failed to setup download progress listener:", error);
      }
    };

    const cleanup = setupListener();
    return () => {
      cleanup.then(fn => fn?.());
    };
  }, []);

  const checkForUpdates = async () => {
    if (checking) return;
    
    setChecking(true);
    try {
      // 确保已获取当前版本号
      if (currentVersion === "0.0.0") {
        console.log("Version not loaded yet, skipping update check");
        return;
      }

      const response = await fetch(
        "https://api.github.com/repos/moky20258/Maco-Downloader/releases/latest"
      );
      
      if (!response.ok) {
        console.error("Failed to fetch releases");
        return;
      }

      const release: GitHubRelease = await response.json();
      const latestVersion = release.tag_name.replace(/^v/, "");

      console.log(`Version check - Current: ${currentVersion}, Latest: ${latestVersion}`);

      // 比较版本号（相同版本不提示更新）
      if (isNewerVersion(latestVersion, currentVersion)) {
        console.log("New version available!");
        setLatestRelease(release);
        setUpdateAvailable(true);
        setShowModal(true);
      } else {
        console.log("Already up to date");
      }
    } catch (error) {
      console.error("Error checking for updates:", error);
    } finally {
      setChecking(false);
    }
  };

  const isNewerVersion = (latest: string, current: string): boolean => {
    const latestParts = latest.split(".").map(Number);
    const currentParts = current.split(".").map(Number);

    for (let i = 0; i < 3; i++) {
      const latestNum = latestParts[i] || 0;
      const currentNum = currentParts[i] || 0;
      if (latestNum > currentNum) return true;
      if (latestNum < currentNum) return false;
    }
    return false;
  };

  const handleDownload = async () => {
    const url = getDownloadUrl();
    if (!url) {
      console.error("No download URL available");
      return;
    }

    console.log("Download URL:", url);

    // 在Tauri环境中，使用应用内下载
    if (isTauri()) {
      try {
        setDownloading(true);
        setDownloadProgress(null);
        setDownloadComplete(false);
        
        // 获取文件名
        const filename = latestRelease?.assets.find(a => a.browser_download_url === url)?.name || "update.exe";
        
        // 调用后端下载命令
        const { invoke } = await import('@tauri-apps/api/core');
        const result = await invoke<string>('download_update', {
          url,
          filename
        });
        
        console.log("Download completed:", result);
        setDownloadComplete(true);
        setDownloading(false);
      } catch (error) {
        console.error("Download failed:", error);
        setDownloading(false);
        setDownloadProgress(null);
        // 如果下载失败，回退到浏览器下载
        try {
          const { open } = await import('@tauri-apps/plugin-shell');
          await open(url);
        } catch (err) {
          console.error("Failed to open browser:", err);
          setDownloadUrl(url);
        }
      }
    } else {
      // 非Tauri环境（开发环境），使用window.open
      window.open(url, '_blank');
    }
  };

  const handleCopyUrl = async () => {
    const url = downloadUrl || getDownloadUrl();
    if (!url) return;

    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy:", error);
      // 降级方案：创建临时textarea
      const textarea = document.createElement('textarea');
      textarea.value = url;
      textarea.style.position = 'fixed';
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand('copy');
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (err) {
        console.error("Copy failed:", err);
      }
      document.body.removeChild(textarea);
    }
  };

  const getDownloadUrl = (): string => {
    if (!latestRelease) return "";

    // 根据操作系统选择合适的安装包
    const isWindows = navigator.platform.includes("Win");
    const isMac = navigator.platform.includes("Mac");
    const isLinux = navigator.platform.includes("Linux");

    if (isWindows) {
      // 优先NSIS安装包，其次MSI
      const nsisAsset = latestRelease.assets.find(a => 
        a.name.includes("x64-setup.exe") || a.name.includes("x64-setup.nsis.exe")
      );
      if (nsisAsset) return nsisAsset.browser_download_url;

      const msiAsset = latestRelease.assets.find(a => a.name.endsWith(".msi"));
      if (msiAsset) return msiAsset.browser_download_url;
    } else if (isMac) {
      const dmgAsset = latestRelease.assets.find(a => a.name.endsWith(".dmg"));
      if (dmgAsset) return dmgAsset.browser_download_url;
    } else if (isLinux) {
      const debAsset = latestRelease.assets.find(a => a.name.endsWith(".deb"));
      if (debAsset) return debAsset.browser_download_url;
    }

    // 默认返回GitHub releases页面
    return latestRelease.html_url;
  };

  useEffect(() => {
    // 仅在Tauri环境中自动检查更新
    if (isTauri()) {
      // 启动后延迟检查，确保版本号已加载
      const timer = setTimeout(() => {
        checkForUpdates();
      }, 5000); // 增加到5秒，确保版本号已加载

      return () => clearTimeout(timer);
    }
  }, [currentVersion]); // 依赖currentVersion，确保版本号加载后再检查

  if (!updateAvailable && !checking) return null;

  return (
    <AnimatePresence>
      {showModal && latestRelease && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setShowModal(false)}
            className="fixed inset-0 bg-black/50 backdrop-blur-sm z-[100]"
          />

          {/* Update Modal */}
          <motion.div
            initial={{ scale: 0.9, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.9, opacity: 0 }}
            transition={{ type: "spring", damping: 25, stiffness: 300 }}
            className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-md bg-white dark:bg-slate-900 rounded-2xl shadow-2xl z-[110] overflow-hidden"
          >
            {/* Header */}
            <div className="bg-gradient-to-r from-sky-500 to-blue-600 p-6 text-white">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="w-12 h-12 bg-white/20 rounded-full flex items-center justify-center">
                    <Download className="w-6 h-6" />
                  </div>
                  <div>
                    <h2 className="text-xl font-bold">发现新版本</h2>
                    <p className="text-sm text-white/80">
                      当前版本: v{currentVersion} → 最新版本: {latestRelease.tag_name}
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => setShowModal(false)}
                  className="p-2 hover:bg-white/20 rounded-full transition-colors"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>
            </div>

            {/* Content */}
            <div className="p-6 space-y-4">
              {/* Update Info */}
              <div className="flex items-start gap-3 p-4 bg-green-50 dark:bg-green-900/20 rounded-xl border border-green-200 dark:border-green-800">
                <CheckCircle className="w-5 h-5 text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
                <div className="flex-1">
                  <p className="text-sm font-medium text-green-800 dark:text-green-300">
                    新版本已发布
                  </p>
                  <p className="text-xs text-green-600 dark:text-green-400 mt-1">
                    发布于 {new Date(latestRelease.published_at).toLocaleDateString("zh-CN")}
                  </p>
                </div>
              </div>

              {/* Release Notes */}
              {latestRelease.body && (
                <div className="space-y-2">
                  <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
                    更新内容：
                  </h3>
                  <div className="max-h-40 overflow-y-auto p-3 bg-slate-50 dark:bg-slate-800 rounded-lg text-xs text-slate-600 dark:text-slate-400 whitespace-pre-wrap">
                    {latestRelease.body}
                  </div>
                </div>
              )}

              {/* Action Buttons */}
              <div className="space-y-3 pt-2">
                {/* 下载进度条 */}
                {downloading && downloadProgress && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between text-xs text-slate-600 dark:text-slate-400">
                      <span>下载中...</span>
                      <span>{downloadProgress.progress.toFixed(1)}%</span>
                    </div>
                    <div className="w-full h-3 bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                      <motion.div
                        initial={{ width: 0 }}
                        animate={{ width: `${downloadProgress.progress}%` }}
                        transition={{ duration: 0.3 }}
                        className="h-full bg-gradient-to-r from-sky-500 to-blue-600 rounded-full"
                      />
                    </div>
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      {(downloadProgress.downloaded / 1024 / 1024).toFixed(2)} MB / {(downloadProgress.total / 1024 / 1024).toFixed(2)} MB
                    </p>
                  </div>
                )}

                {/* 下载完成提示 */}
                {downloadComplete && (
                  <div className="flex items-center gap-3 p-4 bg-green-50 dark:bg-green-900/20 rounded-xl border border-green-200 dark:border-green-800">
                    <CheckCircle className="w-5 h-5 text-green-600 dark:text-green-400 flex-shrink-0" />
                    <div className="flex-1">
                      <p className="text-sm font-medium text-green-800 dark:text-green-300">
                        下载完成！
                      </p>
                      <p className="text-xs text-green-600 dark:text-green-400 mt-1">
                        文件已保存，请运行安装程序进行更新
                      </p>
                    </div>
                  </div>
                )}

                {/* 主下载按钮 */}
                <button
                  onClick={handleDownload}
                  disabled={downloading}
                  className={`w-full py-3 px-4 rounded-xl font-medium text-sm transition-colors flex items-center justify-center gap-2 ${
                    downloading
                      ? "bg-slate-300 dark:bg-slate-700 cursor-not-allowed"
                      : downloadComplete
                      ? "bg-green-500 hover:bg-green-600 text-white"
                      : "bg-sky-500 hover:bg-sky-600 text-white"
                  } cursor-pointer`}
                >
                  {downloading ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      下载中...
                    </>
                  ) : downloadComplete ? (
                    <>
                      <CheckCircle className="w-4 h-4" />
                      下载完成
                    </>
                  ) : (
                    <>
                      <Download className="w-4 h-4" />
                      立即下载
                    </>
                  )}
                </button>

                {/* 下载链接显示和复制 - 始终显示以便用户复制 */}
                {!downloading && (
                  <div className="space-y-2">
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      或手动复制下载链接：
                    </p>
                    <div className="flex gap-2">
                      <div className="flex-1 p-2 bg-slate-50 dark:bg-slate-800 rounded-lg border border-slate-200 dark:border-slate-700 overflow-hidden">
                        <p className="text-xs text-slate-600 dark:text-slate-400 font-mono break-all select-text" style={{ wordBreak: 'break-all' }}>
                          {downloadUrl || getDownloadUrl()}
                        </p>
                      </div>
                      <button
                        onClick={handleCopyUrl}
                        className="px-3 py-2 bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-lg transition-colors flex items-center gap-1 flex-shrink-0"
                      >
                        {copied ? (
                          <>
                            <CheckCircle className="w-4 h-4 text-green-500" />
                            <span className="text-xs">已复制</span>
                          </>
                        ) : (
                          <>
                            <Copy className="w-4 h-4" />
                            <span className="text-xs">复制</span>
                          </>
                        )}
                      </button>
                    </div>
                  </div>
                )}

                {/* 稍后再说按钮 */}
                <button
                  onClick={() => setShowModal(false)}
                  className="w-full py-3 px-4 bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-xl font-medium text-sm transition-colors"
                >
                  稍后再说
                </button>
              </div>
            </div>
          </motion.div>
        </>
      )}

      {/* Checking Indicator (Toast) */}
      {checking && (
        <motion.div
          initial={{ y: -100, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -100, opacity: 0 }}
          className="fixed top-4 left-1/2 -translate-x-1/2 z-[120] bg-white dark:bg-slate-900 shadow-lg rounded-full px-4 py-2 flex items-center gap-2 border border-slate-200 dark:border-slate-700"
        >
          <Loader2 className="w-4 h-4 animate-spin text-sky-500" />
          <span className="text-sm text-slate-700 dark:text-slate-300">检查更新中...</span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
