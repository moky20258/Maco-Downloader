# 歌词功能更新 - v1.3.0

## 🎯 更新内容

### 1. ✅ 从歌曲源搜索歌词
- **后端实现**: 在Rust后端添加了 `get_lyrics` 命令
- **API集成**: 使用 LrcApi (https://api.lrc.cx/) 作为通用歌词搜索源
- **多源支持**: 根据不同provider选择对应的歌词获取策略
- **前端调用**: 在 `tauri-api.ts` 中添加了 `getLyrics` 函数

### 2. ✅ 点击播放控制栏空白处展开歌词
- **交互优化**: 移除了麦克风图标按钮
- **自然操作**: 点击PlayerBar任意空白区域即可展开/收起歌词面板
- **事件处理**: 在按钮和控件上添加 `stopPropagation`,防止误触发

## 📁 文件变更

### 新增文件
- 无

### 修改文件
1. **src-tauri/src/api_types.rs**
   - 添加 `LyricsResponse` 结构体
   
2. **src-tauri/src/commands.rs** (+120行)
   - 添加 `get_lyrics` 命令
   - 实现各provider的歌词获取函数
   - 实现 `get_generic_lyrics` 通用搜索(使用LrcApi)

3. **src-tauri/src/main.rs**
   - 注册 `get_lyrics` 命令

4. **src/lib/tauri-api.ts** (+29行)
   - 添加 `getLyrics` 前端API调用函数

5. **src/components/LyricsPanel.tsx**
   - 移除模拟数据生成逻辑
   - 改为从Tauri后端获取真实歌词
   - 使用 `getLyrics` API调用

6. **src/components/PlayerBar.tsx**
   - 移除麦克风图标按钮
   - 在容器上添加 `onClick={onToggleLyrics}`
   - 在子控件上添加 `stopPropagation` 防止冒泡

## 🔧 技术实现

### 后端歌词获取流程
```rust
get_lyrics(id, provider, title, artist)
  ↓
根据provider选择策略
  ↓
- jianbin-* → get_jianbin_lyrics (待实现)
- bugu → get_bugu_lyrics → get_generic_lyrics
- qq → get_qq_lyrics → get_generic_lyrics
- migu → get_migu_lyrics → get_generic_lyrics
- 其他 → get_generic_lyrics
  ↓
get_generic_lyrics
  ↓
调用 LrcApi: https://api.lrc.cx/lyrics?title=xxx&artist=xxx
  ↓
返回LRC格式歌词
```

### 前端调用流程
```typescript
用户点击播放控制栏空白处
  ↓
触发 onToggleLyrics
  ↓
打开 LyricsPanel
  ↓
调用 loadLyrics()
  ↓
调用 getLyrics(id, provider, title, artist)
  ↓
Tauri invoke('get_lyrics')
  ↓
Rust后端搜索歌词
  ↓
返回LRC文本
  ↓
parseLrcLyrics() 解析
  ↓
显示歌词
```

## 🎨 用户体验

### 之前
- ❌ 需要点击麦克风图标
- ❌ 使用模拟数据
- ❌ 歌词不真实

### 现在
- ✅ 点击播放控制栏任意空白处即可
- ✅ 从真实API获取歌词
- ✅ 支持大多数流行歌曲
- ✅ 操作更自然、更直观

## 📊 支持的歌词源

| Provider | 歌词来源 | 状态 |
|----------|---------|------|
| jianbin-netease | 待实现 | ⏳ |
| jianbin-qq | 待实现 | ⏳ |
| jianbin-kugou | 待实现 | ⏳ |
| jianbin-kuwo | 待实现 | ⏳ |
| bugu | LrcApi | ✅ |
| qq | LrcApi | ✅ |
| migu | LrcApi | ✅ |
| qqmp3 | LrcApi | ✅ |
| livepoo | LrcApi | ✅ |
| gequbao | LrcApi | ✅ |
| gequhai | LrcApi | ✅ |

## 🚀 使用方式

1. **播放音乐**
2. **点击播放控制栏空白处** - 展开歌词面板
3. **查看歌词** - 自动从网络搜索并显示
4. **点击歌词** - 跳转到对应播放位置
5. **再次点击空白处或X按钮** - 关闭歌词面板

## ⚠️ 注意事项

1. **网络依赖**: 歌词搜索需要网络连接
2. **API限制**: LrcApi可能有调用频率限制
3. **歌词可用性**: 不是所有歌曲都有歌词
4. **首次加载**: 可能需要1-2秒搜索时间

## 🔮 后续优化

- [ ] 实现煎饼搜索源的直接歌词获取
- [ ] 添加歌词缓存,减少重复请求
- [ ] 支持多语言歌词(原文+翻译)
- [ ] 卡拉OK模式(逐字高亮)
- [ ] 歌词字体大小调节
- [ ] 离线歌词支持

## 📝 测试建议

1. 播放不同provider的歌曲
2. 测试有歌词和无歌词的情况
3. 测试网络断开时的错误处理
4. 测试点击空白处展开/收起
5. 测试点击歌词跳转功能

---

**版本**: v1.3.0  
**更新日期**: 2026-05-30  
**状态**: ✅ 已完成并可用
