# 歌词API集成指南

## 当前状态
歌词功能已完成UI和交互实现,目前使用模拟数据。需要接入真实歌词API才能显示实际歌词。

## 推荐歌词API

### 1. LrcApi (推荐)
- **URL**: `https://api.lrc.cx/lyrics`
- **参数**:
  - `title`: 歌曲名称
  - `artist`: 歌手名称
  - `album`: 专辑名称 (可选)
- **返回**: LRC格式歌词
- **示例**:
  ```typescript
  const response = await fetch(
    `https://api.lrc.cx/lyrics?title=${encodeURIComponent(title)}&artist=${encodeURIComponent(artist)}`
  );
  const lrcText = await response.text();
  const lyrics = parseLrcLyrics(lrcText);
  ```

### 2. 网易云音乐API
- **需要**: 自建服务器或使用第三方封装
- **参考项目**: 
  - https://github.com/Binaryify/NeteaseCloudMusicApi
  - https://neteasecloudmusicapi.vercel.app/
- **端点**: `/lyric?id={songId}`
- **返回**: JSON格式,包含 lrc, tlyric 等

### 3. QQ音乐API
- **非官方**,可能需要逆向工程
- **建议**: 使用第三方封装库

### 4. 其他开源方案
- **LyricsFinder**: https://github.com/JiangWeixian/lyrics-finder
- **genius-lyrics-api**: https://github.com/mikeal/lyrics (英文为主)

## 集成步骤

### 1. 修改 LyricsPanel.tsx

找到 `loadLyrics` 函数,替换为真实API调用:

```typescript
const loadLyrics = async () => {
  if (!currentMusic) return;
  
  setLoading(true);
  setError(null);
  
  try {
    // 使用 LrcApi 示例
    const apiUrl = `https://api.lrc.cx/lyrics?title=${encodeURIComponent(currentMusic.title)}&artist=${encodeURIComponent(currentMusic.artist)}`;
    
    const response = await fetch(apiUrl);
    
    if (!response.ok) {
      throw new Error('Failed to fetch lyrics');
    }
    
    const lrcText = await response.text();
    const parsedLyrics = parseLrcLyrics(lrcText);
    
    if (parsedLyrics.length === 0) {
      // 无歌词
      setLyrics([]);
    } else {
      setLyrics(parsedLyrics);
    }
  } catch (err) {
    console.error('Failed to load lyrics:', err);
    setError('歌词加载失败');
  } finally {
    setLoading(false);
  }
};
```

### 2. 处理多语言歌词

如果需要显示翻译歌词,可以修改 `LyricLine` 接口:

```typescript
interface LyricLine {
  time: number;
  text: string;
  translation?: string; // 翻译
}
```

然后在解析时同时加载原文和翻译。

### 3. 歌词缓存优化

为了避免重复请求,可以添加缓存:

```typescript
const lyricsCache = new Map<string, LyricLine[]>();

const loadLyrics = async () => {
  const cacheKey = `${currentMusic.title}-${currentMusic.artist}`;
  
  if (lyricsCache.has(cacheKey)) {
    setLyrics(lyricsCache.get(cacheKey)!);
    return;
  }
  
  // ... 加载逻辑
  
  lyricsCache.set(cacheKey, parsedLyrics);
};
```

### 4. 错误处理

建议添加重试机制:

```typescript
const loadLyrics = async (retryCount = 3) => {
  for (let i = 0; i < retryCount; i++) {
    try {
      // ... 加载逻辑
      return; // 成功则退出
    } catch (err) {
      if (i === retryCount - 1) {
        setError('歌词加载失败');
      }
      await new Promise(resolve => setTimeout(resolve, 1000 * (i + 1)));
    }
  }
};
```

## LRC格式说明

标准LRC格式:
```
[00:12.34]这是歌词内容
[00:15.67]第二行歌词
[mm:ss.xx]文本
```

已内置的 `parseLrcLyrics` 函数可以解析此格式。

## 测试建议

1. **单元测试**: 测试 `parseLrcLyrics` 函数
2. **集成测试**: 模拟API响应
3. **边界情况**:
   - 无歌词
   - 歌词格式错误
   - 网络超时
   - 特殊字符

## 注意事项

1. **版权问题**: 歌词可能有版权限制,请确保合法使用
2. **API限制**: 注意API的调用频率限制
3. **隐私保护**: 不要在日志中记录用户搜索的歌词信息
4. **性能优化**: 考虑使用CDN或缓存提高加载速度

## 后续优化方向

- [ ] 支持歌词字体大小调节
- [ ] 支持歌词颜色主题
- [ ] 支持卡拉OK模式(逐字高亮)
- [ ] 支持歌词分享
- [ ] 支持歌词编辑和纠错
- [ ] 支持多语言歌词切换
- [ ] 支持歌词滚动速度调节
