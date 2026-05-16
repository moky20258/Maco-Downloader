# GitHub Actions 自动构建指南

## 📦 自动构建流程

本项目已配置 GitHub Actions，可以自动为 Windows、macOS 和 Linux 构建安装包。

### 触发条件

自动构建会在以下情况下触发：

1. **推送到 main/master 分支**
   ```bash
   git push origin main
   ```

2. **创建版本标签**（推荐，会创建 GitHub Release）
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

3. **手动触发**
   - 进入 GitHub 仓库页面
   - 点击 "Actions" 标签
   - 选择 "Tauri Build" workflow
   - 点击 "Run workflow" 按钮

### 构建产物

#### Windows (windows-latest)
- **NSIS 安装程序**: `.exe` 文件
- **MSI 安装包**: `.msi` 文件

#### macOS (macos-latest)
- **DMG 镜像**: `.dmg` 文件
- **应用程序包**: `.app` 文件

#### Linux (ubuntu-22.04)
- **DEB 包**: `.deb` 文件（Debian/Ubuntu）
- **RPM 包**: `.rpm` 文件（Fedora/RHEL）
- **AppImage**: `.AppImage` 文件（通用 Linux）

### 使用方式

#### 方式一：从 Artifacts 下载（适用于所有推送）

1. 进入仓库的 "Actions" 页面
2. 点击最新的构建任务
3. 等待构建完成
4. 在页面底部的 "Artifacts" 区域下载对应平台的安装包

**注意**: Artifacts 保留 90 天

#### 方式二：从 Releases 下载（仅版本标签）

1. 创建并推送版本标签：
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. 进入仓库的 "Releases" 页面
3. 会自动创建一个新的 Release
4. 下载附带的安装包文件

### 版本号管理

更新版本号的步骤：

1. 更新 `package.json` 中的版本号：
   ```json
   {
     "version": "1.0.0"
   }
   ```

2. 更新 `src-tauri/tauri.conf.json` 中的版本号：
   ```json
   {
     "version": "1.0.0"
   }
   ```

3. 创建 Git 标签：
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

### 构建状态

查看构建状态：
- ✅ 绿色：构建成功
- ❌ 红色：构建失败
- 🟡 黄色：构建中

### 故障排查

如果构建失败，检查以下内容：

1. **构建日志**
   - 点击失败的 job
   - 查看详细错误信息

2. **常见问题**
   - Node.js 依赖安装失败：检查 `package-lock.json` 是否存在
   - Rust 编译错误：检查 `src-tauri/Cargo.toml` 配置
   - 前端构建失败：运行 `npm run build` 本地测试

3. **重新触发构建**
   - 修复问题后推送新的 commit
   - 或在 Actions 页面手动重新运行

## 🔧 Workflow 配置

配置文件位置：`.github/workflows/tauri-build.yml`

### 主要功能

- ✅ 多平台并行构建（Windows、macOS、Linux）
- ✅ 自动上传构建产物
- ✅ 版本标签自动创建 Release
- ✅ 自动生成 Release Notes
- ✅ 缓存依赖加速构建

### 环境变量

无需配置额外的 secrets，使用默认的 `GITHUB_TOKEN` 即可。

## 📝 示例：发布新版本

```bash
# 1. 更新版本号
# 编辑 package.json 和 src-tauri/tauri.conf.json

# 2. 提交更改
git add .
git commit -m "release: v1.0.0"

# 3. 创建标签
git tag v1.0.0

# 4. 推送
git push origin main
git push origin v1.0.0
```

GitHub Actions 会自动：
1. 为三个平台构建安装包
2. 上传构建产物
3. 创建 GitHub Release
4. 附加所有安装包文件

---

**构建时长**: 通常 10-20 分钟（取决于平台）
