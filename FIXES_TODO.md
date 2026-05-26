# 待修复问题清单

## 问题1: 非煎饼源搜索不到结果

### 原因分析
- Tauri后端(`commands.rs`)只实现了`jianbin`系列provider的搜索
- 其他provider(gequbao, bugu, gequhai等)直接返回空数组
- 原版本使用Next.js API Routes + TypeScript Provider系统
- 迁移到Tauri后，Rust后端没有完整实现所有provider

### 解决方案（3选1）

#### 方案A: 在Rust中重新实现所有provider（推荐，工作量大）
需要为每个provider实现对应的Rust搜索逻辑：
- gequbao: 爬取 https://www.gequbao.com/s/{query}
- gequhai: 爬取 https://www.gequhai.com
- bugu: 调用 https://a.buguyy.top API
- 等等...

#### 方案B: 使用外部API代理（中等工作量）
- 创建一个统一的音乐搜索API服务
- Tauri后端调用该API
- 优点：逻辑集中，易维护
- 缺点：需要额外服务器

#### 方案C: 临时方案 - 只保留煎饼源（快速）
- 移除UI中不可用的provider选项
- 只保留jianbin-*系列
- 等有时间再完整实现

### 建议
暂时采用**方案C**，快速修复用户体验。在后续版本中实现方案A。

---

## 问题2: 播放失败时退出播放状态

### 原因分析
- `handlePlay`函数在获取URL失败或播放失败时调用`setActiveMusic(null)`
- 这会清空播放状态，导致控制栏消失
- 对于播放列表，应该自动跳过而不是退出

### 修复方案
已修改`handlePlay`函数：
1. 获取URL失败时：直接调用`handleNext()`自动跳过
2. 播放失败时：弹窗提示用户选择
   - 点击"是"：立即跳到下一首
   - 点击"否"或3秒无操作：自动跳到下一首
3. 不再调用`setActiveMusic(null)`

### 代码位置
`src/app/page.tsx` - `handlePlay`函数

---

## 问题3: 版本号不一致

### 原因分析
- `tauri.conf.json`中的`version`字段硬编码为"0.1.0"
- 每次构建都使用这个固定版本号
- GitHub Release的tag（如v0.2.0）与应用内部版本号不同步

### 解决方案

#### 方案A: CI自动更新版本号（推荐）
在GitHub Actions中添加步骤：
```yaml
- name: Update version from tag
  if: startsWith(github.ref, 'refs/tags/')
  run: |
    VERSION=${GITHUB_REF#refs/tags/v}
    # 更新 tauri.conf.json
    jq ".version = \"$VERSION\"" src-tauri/tauri.conf.json > tmp.json && mv tmp.json src-tauri/tauri.conf.json
    # 更新 package.json  
    jq ".version = \"$VERSION\"" package.json > tmp.json && mv tmp.json package.json
```

#### 方案B: 使用Git tag作为版本号
修改Tauri配置，从环境变量读取版本：
```json
{
  "version": "{{VERSION}}"  // 构建时替换
}
```

#### 方案C: 手动更新（不推荐）
每次发版前手动修改版本号

### 建议
采用**方案A**，在CI/CD中自动化版本号管理。

---

## 实施计划

### 第一阶段（立即执行）
1. ✅ 问题2: 修复播放失败逻辑 - 已完成
2. 问题1: 临时隐藏不可用的provider选项
3. 问题3: 添加CI自动版本号更新

### 第二阶段（后续优化）
1. 在Rust中实现gequbao provider
2. 逐步实现其他provider
3. 完善播放失败的错误提示UI

---

## 需要用户确认

1. **问题1**: 是否先采用方案C（只保留煎饼源），后续再完整实现？
2. **问题3**: 是否同意在GitHub Actions中添加自动版本号更新？
