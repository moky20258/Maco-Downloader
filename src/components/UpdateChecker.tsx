"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Download, X, AlertCircle, CheckCircle, Loader2 } from "lucide-react";
import { isTauri } from "@/lib/tauri-api";
import { getVersion } from "@tauri-apps/api/app";

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

export function UpdateChecker() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [checking, setChecking] = useState(false);
  const [showModal, setShowModal] = useState(false);
  const [latestRelease, setLatestRelease] = useState<GitHubRelease | null>(null);
  const [currentVersion, setCurrentVersion] = useState("0.0.0");

  // 获取应用当前版本号
  useEffect(() => {
    const loadVersion = async () => {
      try {
        if (isTauri()) {
          const version = await getVersion();
          setCurrentVersion(version);
        }
      } catch (error) {
        console.error("Failed to get app version:", error);
      }
    };
    loadVersion();
  }, []);

  const checkForUpdates = async () => {
    if (checking) return;
    
    setChecking(true);
    try {
      const response = await fetch(
        "https://api.github.com/repos/moky20258/Maco-Downloader/releases/latest"
      );
      
      if (!response.ok) {
        console.error("Failed to fetch releases");
        return;
      }

      const release: GitHubRelease = await response.json();
      const latestVersion = release.tag_name.replace(/^v/, "");

      // 比较版本号
      if (isNewerVersion(latestVersion, currentVersion)) {
        setLatestRelease(release);
        setUpdateAvailable(true);
        setShowModal(true);
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
      // 启动后延迟检查，避免影响加载体验
      const timer = setTimeout(() => {
        checkForUpdates();
      }, 3000);

      return () => clearTimeout(timer);
    }
  }, []);

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
              <div className="flex gap-3 pt-2">
                <a
                  href={getDownloadUrl()}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex-1 py-3 px-4 bg-sky-500 hover:bg-sky-600 text-white rounded-xl font-medium text-sm transition-colors flex items-center justify-center gap-2 cursor-pointer"
                  onClick={() => setShowModal(false)}
                >
                  <Download className="w-4 h-4" />
                  立即下载
                </a>
                <button
                  onClick={() => setShowModal(false)}
                  className="py-3 px-4 bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-xl font-medium text-sm transition-colors"
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
