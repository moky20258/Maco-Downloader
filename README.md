# Maco Music (Maco音乐下载站)

![Next.js](https://img.shields.io/badge/Next.js-16.1-black)
![TypeScript](https://img.shields.io/badge/TypeScript-5.0-blue)
![Tailwind CSS](https://img.shields.io/badge/Tailwind_CSS-4.0-38B2AC)
![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131)
![License](https://img.shields.io/badge/License-MIT-green)
![Version](https://img.shields.io/badge/Version-1.3.5-blue)

## 📖 简介

**Maco Music** 是一个基于 Next.js 16 和 Tauri 2.0 构建的现代化跨平台音乐搜索与下载桌面应用。界面设计简约纯净，支持多渠道音乐搜索、在线试听、歌词显示、批量下载，并配备了丝滑的暗黑模式（涟漪过渡动画）。

本项目致力于提供无广告、极速、纯净的音乐获取体验，同时支持 Web 版本和桌面客户端。

## ✨ 主要特性

- 🎵 **多源聚合搜索**：支持 10+ 音乐源渠道（歌曲宝、歌曲海、布谷、QQ音乐、QQMP3、咪咕、力音、煎饼系列等），一键切换。
- 🎧 **在线试听**：内置精美悬浮播放器，支持播放/暂停、进度拖拽、音量调节、上下曲切换、**播放模式切换（顺序/随机/单曲）**。
- 📝 **歌词显示**：支持实时滚动歌词面板，自动匹配歌词并高亮当前行。
- 🖱️ **便捷交互**：支持列表**双击播放**，鼠标悬停/选中效果优化，操作流畅。
- ⬇️ **批量下载**：支持多选歌曲，一键批量打包下载选中的音乐，带下载进度管理。
- 📂 **下载文件夹管理**：下载任务列表支持一键打开下载文件夹，快速查看已下载音频。
- 🎼 **播放列表**：支持添加歌曲到播放列表，持久化存储，方便连续播放。
- 🌓 **极致主题体验**：
    - 完美适配**深色/浅色模式**。
    - 独家定制的**涟漪扩散**切换动画（基于 View Transitions API），视觉效果惊艳。
- 🖥️ **跨平台支持**：基于 Tauri 2.0 构建，支持 Windows、macOS、Linux 桌面平台。
- ⚡ **现代化技术栈**：基于 React 19、Next.js 16 App Router、Tailwind CSS v4、Tauri 2.0 构建。

## 🎹 支持音源与音质说明

本项目聚合了多个第三方音乐搜索引擎，支持以下音源：

- **歌曲宝** (gequbao)
- **歌曲海** (gequhai)
- **布谷** (bugu)
- **QQ音乐** (qq)
- **QQMP3** (qqmp3)
- **咪咕** (migu)
- **力音** (livepoo)
- **煎饼系列** (jianbin-netease/jianbin-qq/jianbin-kugou/jianbin-kuwo)

> **⚠️ 关于音质的重要说明：**
> 1. **不支持自定义音质选择**：本程序自动解析目标源提供的默认最高可用音质。
> 2. **无损音质支持**：部分音源（如咪咕、QQMP3等）在资源允许的情况下会自动解析出 **FLAC** 无损格式，请自行探索尝试。
> 3. **解析策略**：程序会自动尝试获取最佳播放链接，若某个源无法播放，建议切换其他源重试。
> 4. **网盘链接处理**：部分音源（如歌曲海）可能返回网盘链接（夸克/百度网盘），此类资源仅支持下载，无法在线试听。

## 🛠 技术栈

### 前端
- **核心框架**: [Next.js 16.1.2](https://nextjs.org/) (App Router)
- **UI 框架**: [React 19](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/)
- **样式方案**: [Tailwind CSS v4](https://tailwindcss.com/)
- **组件库**: [Ant Design 6](https://ant.design/) + [LobeHub UI](https://github.com/lobehub/lobe-ui)
- **动画库**: [Framer Motion](https://www.framer.com/motion/)
- **图标库**: [Lucide React](https://lucide.dev/)
- **主题管理**: [next-themes](https://github.com/pacocoursey/next-themes) + View Transitions API

### 后端 (Tauri)
- **桌面框架**: [Tauri 2.0](https://tauri.app/)
- **后端语言**: Rust
- **HTTP 客户端**: reqwest
- **HTML 解析**: scraper + regex
- **异步运行时**: tokio

### Web API
- **后端处理**: Next.js API Routes + Axios + Cheerio

## 🚀 快速开始

### 环境要求

- Node.js >= 18.17.0
- npm / pnpm / yarn
- Rust 1.70+ (仅桌面客户端开发需要)

### 1. 克隆项目

```bash
git clone https://github.com/markcxx/coco-downloader.git
cd coco-downloader
```

### 2. 安装依赖

```bash
npm install
# 或者
yarn install
# 或者
pnpm install
```

### 3. 运行开发服务器

```bash
npm run dev
```

打开浏览器访问 [http://localhost:3000](http://localhost:3000) 即可开始使用。

### 4. 构建 Tauri 桌面应用

```bash
# 开发模式
npm run tauri:dev

# 构建生产版本
npm run tauri:build
```

构建完成后，可执行文件位于 `src-tauri/target/release/bundle/` 目录下。

### 5. 构建 Web 生产版本

```bash
npm run build
npm start
```

## 📂 项目结构

```
coco-downloader/
├── src/                          # Next.js 前端代码
│   ├── app/                      # Next.js App Router 核心目录
│   │   ├── api/                  # Web API 路由 (search, url, download)
│   │   ├── globals.css           # 全局样式 (含 Tailwind v4 配置)
│   │   ├── layout.tsx            # 根布局 (集成 ThemeProvider)
│   │   └── page.tsx              # 首页主要逻辑 (搜索、列表、交互)
│   ├── components/               # UI 组件
│   │   ├── Navbar.tsx            # 顶部导航栏 (含涟漪主题切换逻辑)
│   │   ├── PlayerBar.tsx         # 底部悬浮播放器
│   │   ├── LyricsPanel.tsx       # 歌词显示面板
│   │   ├── PlaylistDrawer.tsx    # 播放列表抽屉
│   │   ├── DownloadDrawer.tsx    # 下载管理抽屉
│   │   ├── UpdateChecker.tsx     # 版本更新检查器
│   │   └── ThemeProvider.tsx     # 主题上下文提供者
│   ├── lib/                      # 工具库
│   │   ├── providers/            # 音乐源策略模式实现 (前端 Web API)
│   │   │   └── impl/             # 各音源实现 (bugu, gequbao, gequhai, jianbin, livepoo, migu, qq, qqmp3)
│   │   ├── tauri-api.ts          # Tauri 桌面 API 封装
│   │   └── utils.ts              # 工具函数
│   ├── types/                    # TypeScript 类型定义
│   └── const/                    # 常量配置 (品牌信息等)
├── src-tauri/                    # Tauri 桌面应用后端
│   ├── src/
│   │   ├── main.rs               # Tauri 入口
│   │   ├── commands.rs           # Tauri 命令实现 (搜索、播放、下载、歌词)
│   │   └── api_types.rs          # API 类型定义
│   ├── tauri.conf.json           # Tauri 配置文件
│   └── icons/                    # 应用图标
├── public/                       # 静态资源文件
└── ...
```

## 🎨 特色功能实现解析

### 涟漪主题切换
在 `src/components/Navbar.tsx` 中，我们利用了浏览器原生的 `document.startViewTransition` API 配合 CSS `clip-path` 属性。
当用户点击主题切换按钮时，计算点击坐标，以该坐标为圆心，计算覆盖全屏所需的最大半径，然后执行圆形扩散遮罩动画。这比传统的 CSS `transition` 全局淡入淡出更具动感和现代感。

### 音乐源扩展
项目采用策略模式设计，支持前端 Web API 和 Tauri 后端双实现：
- **前端**: 在 `src/lib/providers/impl` 下定义了统一的接口，每个音源独立实现
- **Tauri 后端**: 在 `src-tauri/src/commands.rs` 中实现了 Rust 版本的音源解析

若需添加新的音乐网站源，只需在对应位置新建实现并注册即可，无需大幅修改前端逻辑。

### 播放模式支持
播放器支持三种播放模式：
- **顺序播放** (order): 按列表顺序依次播放
- **随机播放** (shuffle): 随机打乱列表后播放
- **单曲循环** (single): 重复播放当前歌曲

### 歌词同步
系统通过 Tauri 后端调用 LrcApi 获取歌词，前端 `LyricsPanel` 组件实现实时滚动和高亮当前行，支持主题适配。

## 🚀 部署方案

### 方案一：Web 本地部署

```bash
npm run build
npm start
```

### 方案二：Docker 部署

本项目支持 Docker 快速部署，且支持自定义端口。

1. **拉取镜像**
   ```bash
   docker pull markcxx/coco-downloader:latest
   ```

2. **运行容器**
   ```bash
   # 默认运行在 3000 端口
   docker run -d -p 3000:3000 --name coco-downloader markcxx/coco-downloader:latest

   # 自定义端口 (例如 8080)
   docker run -d -p 8080:3000 -e PORT=3000 --name coco-downloader markcxx/coco-downloader:latest
   ```

### 方案三：Tauri 桌面应用

构建为原生桌面应用，支持 Windows、macOS、Linux：

```bash
# 构建生产版本
npm run tauri:build
```

产物位置：`src-tauri/target/release/bundle/`

## ⚠️ 免责声明

1. 本项目仅供**个人学习与技术交流**使用，严禁用于任何商业用途。
2. 本项目所有音乐资源均来源于互联网第三方网站，本项目仅提供数据聚合与检索服务，不存储任何音乐文件。
3. 部分音源可能返回网盘链接（如夸克网盘、百度网盘），此类资源需要用户在网盘客户端下载。
4. 若您发现本项目侵犯了您的权益，请联系我们进行删除。
5. 使用本项目产生的任何法律后果由使用者自行承担。

## 🤝 贡献与反馈

如果您发现任何问题或有新功能建议，欢迎提交 Issue 或 Pull Request。

- **仓库地址**: [https://github.com/markcxx/coco-downloader](https://github.com/markcxx/coco-downloader)
- **开发者博客**: [https://www.markqq.com](https://www.markqq.com)
- **联系邮箱**: 2811016860@qq.com

## 📄 许可证

MIT License
