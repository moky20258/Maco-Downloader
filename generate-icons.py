#!/usr/bin/env python3
"""
生成 Maco 品牌图标
设计理念：
- 字母 "M" 为核心
- 融入音乐元素（波形/音符）
- 现代、简洁风格
"""

from PIL import Image, ImageDraw, ImageFont
import os
import math

# 颜色配置
PRIMARY_COLOR = "#0EA5E9"  # 天蓝色 (sky-500)
SECONDARY_COLOR = "#0284C7"  # 深天蓝
BG_COLOR = "#FFFFFF"  # 白色背景
DARK_BG = "#0F172A"  # 深色背景

def create_maco_logo(size, bg_color=BG_COLOR, include_text=False, transparent_bg=False):
    """创建 Maco 品牌图标"""
    if transparent_bg:
        img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    else:
        img = Image.new('RGBA', (size, size), bg_color)
    draw = ImageDraw.Draw(img)
    
    # 计算中心点和尺寸
    center_x = size // 2
    center_y = size // 2
    logo_size = int(size * 0.7)
    
    # 绘制圆形背景
    padding = int(size * 0.1)
    draw.ellipse(
        [padding, padding, size - padding, size - padding],
        fill=PRIMARY_COLOR
    )
    
    # 绘制字母 "M" 和音乐波形
    m_width = int(size * 0.45)
    m_height = int(size * 0.35)
    m_left = center_x - m_width // 2
    m_top = center_y - m_height // 2
    
    # M 的四个关键点
    p1 = (m_left, m_top + m_height)  # 左下
    p2 = (m_left, m_top)  # 左上
    p3 = (center_x, m_top + m_height * 0.4)  # 中间凹陷
    p4 = (m_left + m_width, m_top)  # 右上
    p5 = (m_left + m_width, m_top + m_height)  # 右下
    
    # 绘制 M 的轮廓
    line_width = max(4, size // 40)
    draw.line([p1, p2], fill='white', width=line_width)
    draw.line([p2, p3], fill='white', width=line_width)
    draw.line([p3, p4], fill='white', width=line_width)
    draw.line([p4, p5], fill='white', width=line_width)
    
    # 在 M 下方添加音乐波形线条
    wave_y = m_top + m_height + int(size * 0.08)
    wave_width = int(size * 0.3)
    wave_left = center_x - wave_width // 2
    
    for i in range(3):
        x = wave_left + i * (wave_width // 2)
        bar_height = int(size * 0.05) * (1 if i == 1 else 0.6)
        draw.rounded_rectangle(
            [x, wave_y, x + int(size * 0.04), wave_y + bar_height],
            radius=2,
            fill='white'
        )
    
    return img

def create_icon_sizes():
    """生成所有需要的图标尺寸"""
    
    # 定义需要的尺寸 - 只保留必要的
    sizes = {
        'tauri': [32, 64, 128],
        'tauri_2x': [128],  # 128x128@2x = 256
        'web': [512],  # 只生成 logo.png
    }
    
    base_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = base_dir  # 脚本就在项目根目录
    web_dir = os.path.join(project_root, 'public')
    
    print("🎨 开始生成 Maco 品牌图标...")
    
    # 1. 生成 Tauri 图标
    tauri_icons_dir = os.path.join(project_root, 'src-tauri', 'icons')
    for size in sizes['tauri']:
        img = create_maco_logo(size)
        img.save(os.path.join(tauri_icons_dir, f'{size}x{size}.png'))
        print(f"  ✓ 生成 Tauri icon: {size}x{size}")
    
    # 128x128@2x
    img_2x = create_maco_logo(256)
    img_2x.save(os.path.join(tauri_icons_dir, '128x128@2x.png'))
    print(f"  ✓ 生成 Tauri icon: 128x128@2x (256x256)")
    
    # icon.png (主图标，使用 512x512)
    main_icon = create_maco_logo(512)
    main_icon.save(os.path.join(tauri_icons_dir, 'icon.png'))
    print(f"  ✓ 生成主图标 icon.png: 512x512")
    
    # 2. 生成 Web logo.png
    logo_png = create_maco_logo(512)
    logo_png.save(os.path.join(web_dir, 'logo.png'))
    print(f"  ✓ 生成 logo.png: 512x512")
    
    print("\n✅ 所有图标生成完成！")
    print(f"📁 Tauri 图标: {tauri_icons_dir}")
    print(f"📁 Web 图标: {project_root}/public")

def create_favicon():
    """生成 favicon.ico"""
    base_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = base_dir  # 脚本就在项目根目录
    
    # 创建多个尺寸用于 favicon
    sizes = [16, 32, 48]
    images = []
    
    for size in sizes:
        img = create_maco_logo(size)
        images.append(img)
    
    # 保存为 ICO 格式
    ico_path = os.path.join(project_root, 'public', 'favicon.ico')
    images[0].save(ico_path, format='ICO', sizes=[(s, s) for s in sizes])
    print(f"  ✓ 生成 favicon.ico")

if __name__ == '__main__':
    try:
        create_icon_sizes()
        create_favicon()
        print("\n🎉 图标生成任务完成！")
    except Exception as e:
        print(f"\n❌ 错误: {e}")
        import traceback
        traceback.print_exc()
