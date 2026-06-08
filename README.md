# ADOFAI Music Box 🎵

<div align="center">

一个专为 ADOFAI 和 Rhythm Doctor 谱面设计的本地音乐播放器

![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri)
![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react)
![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?style=flat-square&logo=typescript)
![Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust)

</div>

---

## 📖 项目简介

**ADOFAI Music Box** 是一个基于 Tauri 框架开发的桌面音乐播放器，专注于提供节奏游戏谱面的完整音乐体验。它不仅播放音乐，还能精确同步谱面中的打拍音、音效和视频，让你在欣赏音乐的同时体验原汁原味的节奏感。

### ✨ 核心特性

- 🎮 **双游戏支持** - 同时支持 ADOFAI 和 Rhythm Doctor 两款游戏的谱面
- 🎵 **智能谱面解析** - 自动读取 `.adofai` / `.rdlevel` 文件，提取音乐、封面、视频等资源
- 🎹 **实时音效同步** - 精确播放打拍音、三球音效、长按音效和谱面音效（PlaySound）
- 🎬 **视频背景支持** - 沉浸式播放页面，支持视频背景与音乐完美同步
- 🎨 **现代化 UI** - QQ 音乐风格的界面，动态封面配色，流畅的交互动画
- 📂 **本地曲库管理** - 文件夹扫描、单曲添加、收藏、最近播放、隐藏曲目管理
- ⚡ **快速启动** - 智能缓存机制，后台扫描，启动速度快
- 🔊 **独立音量控制** - 主音量、音乐、打拍音、谱面音效四路独立调节

---

## 🖼️ 界面预览

```
┌─────────────────────────────────────────────────────────────┐
│  [ADOFAI Music Box]  [ADOFAI | 节奏医生]        🔍 搜索    │
├──────────┬──────────────────────────────────────────────────┤
│ ❤ 喜欢   │  ┌────┐ ┌────┐ ┌────┐ ┌────┐                    │
│ 🎵 最近  │  │封面│ │封面│ │封面│ │封面│  [网格视图]        │
│ 📚 本地  │  └────┘ └────┘ └────┘ └────┘                    │
│          │  曲名1   曲名2   曲名3   曲名4                    │
│ [管理]   │                                                  │
│ [扫描]   │  ────────────────────────────────────            │
│          │  曲名     作曲     谱师     时长   [列表视图]    │
│          │  ────────────────────────────────────            │
├──────────┴──────────────────────────────────────────────────┤
│ [封面] 当前曲名 - 艺术家  ⏮ ⏯ ⏭  ━━━━━━━●──  🔊 ⚙        │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ 技术栈

### 前端
- **框架**: React 19 + TypeScript 5.8
- **构建工具**: Vite 7
- **UI**: 原生 CSS + 设计令牌系统
- **图标**: Lucide React
- **音频**: Web Audio API + Lookahead Scheduler

### 后端
- **框架**: Tauri 2.0
- **语言**: Rust (Edition 2021)
- **核心库**:
  - `serde` + `serde_json` - JSON 序列化
  - `json5` - 宽松 JSON 解析
  - `walkdir` - 文件系统遍历
  - `dirs` - 系统目录定位

---

## 📦 安装与运行

### 环境要求

- **Node.js**: 18+ 
- **pnpm**: 8+
- **Rust**: 1.70+
- **操作系统**: Windows 10/11

### 开发环境

```powershell
# 1. 克隆仓库
git clone <repository-url>
cd ADOFAIMusicBox

# 2. 安装依赖
pnpm install

# 3. 启动开发服务器
pnpm tauri dev
```

### 生产构建

```powershell
# 构建应用
pnpm tauri build

# 输出位置
# src-tauri/target/release/bundle/
```

### 代码检查

```powershell
# TypeScript 类型检查
pnpm exec tsc --noEmit

# Rust 代码检查
cargo check --manifest-path src-tauri/Cargo.toml

# Rust 代码格式化
cargo fmt --manifest-path src-tauri/Cargo.toml
```

---

## 📁 项目结构

```
ADOFAIMusicBox/
├── src/                          # 前端源码
│   ├── App.tsx                   # 应用主入口
│   ├── components/               # 通用组件
│   │   ├── AppShell.tsx         # 应用外壳、侧边栏
│   │   ├── FolderManager.tsx    # 本地来源管理
│   │   └── ...
│   ├── features/                 # 功能模块
│   │   ├── library/             # 曲库视图
│   │   ├── player/              # 播放器组件
│   │   └── settings/            # 设置页面
│   ├── audio/                    # 音频引擎
│   │   ├── useChartAudio.ts     # Web Audio 播放器
│   │   └── audioResources.ts    # 音效资源管理
│   ├── lib/                      # 工具库
│   │   ├── tauri.ts             # Tauri 命令封装
│   │   ├── text.ts              # 文本处理
│   │   └── ...
│   ├── types/                    # 类型定义
│   │   └── domain.ts            # 领域模型
│   └── styles/                   # 样式文件
│       ├── tokens.css           # 设计令牌
│       └── ...
│
├── src-tauri/                    # 后端源码
│   ├── src/
│   │   ├── lib.rs               # Tauri 应用初始化
│   │   ├── library.rs           # 通用数据结构
│   │   ├── adofai/              # ADOFAI 模块
│   │   │   ├── commands.rs      # Tauri 命令
│   │   │   ├── parser.rs        # 谱面解析
│   │   │   ├── scanner.rs       # 文件扫描
│   │   │   ├── timeline.rs      # 音频时间线生成 ⭐
│   │   │   └── settings.rs      # 设置管理
│   │   └── rhythm_doctor/       # Rhythm Doctor 模块
│   │       ├── parser.rs
│   │       ├── scanner.rs
│   │       └── timeline.rs
│   ├── Cargo.toml               # Rust 依赖配置
│   └── tauri.conf.json          # Tauri 配置
│
├── public/                       # 静态资源
│   └── audio/                   # 内置音效
│       ├── adofai/              # ADOFAI 音效包
│       └── rhythm-doctor/       # RD 音效包
│
├── package.json
├── vite.config.ts
└── README.md
```

---

## 🎯 核心功能详解

### 1. 谱面解析

支持标准和非标准格式的谱面文件：

```rust
// 两阶段解析策略
1. 尝试严格 JSON 解析
2. 失败后进入宽松模式：
   - 移除 BOM 标记
   - 修复尾逗号
   - 转义字符串内的换行符
   - 清理控制字符
3. 使用 json5 解析
```

**提取内容**：
- 元数据：曲名、艺术家、谱师、BPM
- 资源：音频、封面、图标、视频文件
- 事件：地板序列、速度变化、音效配置

### 2. 音频时间线生成 ⭐

这是项目的核心算法，完全按照 ADOFAI 游戏源码逻辑实现：

**处理流程**：
```
1. 读取角度数据 → 生成地板序列
2. 应用地板事件：
   • SetSpeed - 改变速度
   • Twirl - 改变旋转方向
   • Pause/FreeRoam - 增加等待
   • Midspin - 中旋地板
   • MultiPlanet - 三球 DLC
   • Hold - 长按音效
3. 计算每个地板的精确入口时间
4. 生成三类音效事件：
   • hitEvents - 普通打拍音
   • playSoundEvents - PlaySound 音效
   • holdSoundEvents - 长按音效
```

**关键公式**：
```rust
// 地板时间间隔
delta_time = (2π / angle) / bpm * 60 / speed / pitch

// Pause 事件额外时间
extra_beats = pause_duration * bpm / 60
```

### 3. Web Audio 播放引擎

**技术亮点**：
- **Lookahead Scheduler**: 提前 160ms 调度音效，消除延迟
- **独立音量通道**: 四路 GainNode（主音量/音乐/打拍音/谱面音效）
- **精确同步**: 使用 `AudioContext.currentTime` 作为主时钟

```typescript
// 播放流程
1. 加载主音乐 → AudioBuffer
2. 预加载所有音效 → Map<name, AudioBuffer>
3. 启动 scheduler (每 25ms 执行)
4. 在 lookahead 窗口内调度即将播放的音效
5. 拖动进度时重新计算窗口
```

### 4. 曲库管理

**智能缓存机制**：
```
启动时：
  ├─ 只读取缓存 (tracks-{game}.json)
  ├─ 没有缓存？后台扫描
  └─ 有缓存？快速启动

扫描时：
  ├─ 后台线程执行
  ├─ 遍历文件夹
  ├─ 解析谱面
  ├─ 提取资源
  └─ 更新缓存

手动操作：
  ├─ 添加单曲 → 单文件解析 + 更新缓存
  ├─ 移除曲目 → 加入 hiddenTracks
  └─ 恢复曲目 → 从 hiddenTracks 移除
```

**数据存储位置**：
```
Windows: %APPDATA%\ADOFAI Music Box\
  ├─ settings.json              # 设置、来源、收藏
  ├─ tracks-adofai.json         # ADOFAI 曲库缓存
  └─ tracks-rhythm-doctor.json  # RD 曲库缓存
```

### 5. UI 设计

**QQ 音乐风格**：
- 浅色主题 + 圆角卡片
- 左侧导航栏（喜欢/最近/本地）
- 底部固定播放控制栏
- 点击封面进入沉浸式全屏播放页

**动态配色**：
```typescript
// 从封面图片提取主色调
useCoverPalette() → {
  accent,        // 强调色
  accentText,    // 文字色
  backgroundA,   // 背景渐变起点
  backgroundB,   // 背景渐变终点
  soft          // 柔和色
}

// 应用到：进度条、按钮、文字高亮
```

**唱片机动画**：
- 播放时唱片旋转 (`animation: spin`)
- 视频背景模糊融合 (`backdrop-filter: blur()`)
- 流畅的进入/退出转场

---

## 🔧 已知问题与解决方案

### 1. 非标准 JSON 谱面
**问题**: 部分谱面含有 BOM、尾逗号、非法字符  
**方案**: 两阶段解析（严格 → 宽松）

### 2. Unity 富文本标签
**问题**: `<color=#FF0000>标题</color>` 直接显示在 UI  
**方案**: `cleanDisplayText()` 正则清理所有标签

### 3. 打拍音时间错位
**问题**: 简单按 BPM 等分会在复杂谱面中错乱  
**方案**: 按地板序列逐个累计真实时间

### 4. 三球音效延迟
**问题**: 激活音效和地板音效播放时机错开  
**方案**: 同一时间点插入多个音效事件

### 5. 视频同步偏移
**问题**: 不同谱面的 `videoOffset` 方向不统一  
**方案**: 使用谱面自身配置，不做全局补偿

### 6. 启动缓慢
**问题**: 启动时同步扫描大量谱面  
**方案**: 启动只读缓存，扫描移至后台线程

---

## 🎮 使用方法

### 添加谱面

1. **添加文件夹**: 点击侧边栏 "管理本地来源" → "添加文件夹"
   - 选择包含 `.adofai` 或 `.rdlevel` 文件的文件夹
   - 推荐：Steam Workshop 目录 `steamapps/workshop/content/977950`

2. **添加单曲**: 点击 "添加 ADOFAI/RD 谱面"
   - 选择单个谱面文件
   - 适合临时添加几首歌

3. **重新扫描**: 添加新谱面后点击 "重新扫描" 更新曲库

### 播放控制

- **播放/暂停**: 点击播放按钮或空格键
- **上一曲/下一曲**: `⏮` / `⏭` 按钮
- **进度拖动**: 拖动进度条快速跳转
- **播放模式**: 点击 🔁 按钮切换（顺序/循环/单曲/随机）
- **音量调节**: 点击 🔊 打开音量混音器

### 沉浸式播放

- 点击底部播放器的封面图片
- 进入全屏播放页面，显示：
  - 旋转唱片动画
  - 视频背景（如果有）
  - 动态配色的控制界面
  - 谱面详细信息

### 曲库管理

- **收藏**: 点击 ❤ 按钮添加到喜欢
- **搜索**: 顶部搜索框输入关键词
- **排序**: 按歌名/作曲/时长/BPM 排序
- **视图**: 切换列表/网格视图
- **右键菜单**:
  - 打开文件夹位置
  - 显示谱面文件
  - 从曲库移除

---

## 🧪 已验证谱面

项目在以下真实谱面上通过完整测试：

- ✅ Steam Workshop 全库扫描 (`content/977950/*`)
- ✅ `Light Years Away` (三球谱面)
- ✅ `ViLLAGE OF CHRYSANTHEMUM` (视频谱面)
- ✅ 多个中旋、长按、复杂事件谱面
- ✅ 非标准格式、富文本标题谱面

---

## 🚀 后续优化计划

### 性能优化
- [ ] 首屏骨架屏
- [ ] 扫描进度实时显示
- [ ] 曲库缓存版本管理
- [ ] 增量扫描（只处理新增/修改）
- [ ] 虚拟滚动（大曲库优化）

### 功能扩展
- [ ] 自定义歌单
- [ ] 批量操作（移除/恢复）
- [ ] 文件系统变动监听
- [ ] 导出/导入曲库数据
- [ ] 更多播放统计（播放次数、最喜欢等）
- [ ] 快捷键自定义

### UI/UX 改进
- [ ] 深色模式
- [ ] 更多主题配色
- [ ] 列表 hover 动效优化
- [ ] 空状态精美插图
- [ ] 加载动画优化
- [ ] 更流畅的转场动画

### 音频增强
- [ ] 更多内置音效支持
- [ ] 自定义音效包
- [ ] 均衡器
- [ ] 淡入淡出
- [ ] 音频可视化

---

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

### 开发建议

1. **代码规范**:
   - TypeScript: 使用 ESLint 配置
   - Rust: 使用 `cargo fmt` 格式化

2. **提交信息**:
   ```
   feat: 添加新功能
   fix: 修复 bug
   docs: 更新文档
   style: 代码格式调整
   refactor: 重构代码
   perf: 性能优化
   test: 添加测试
   ```

3. **测试**:
   - 确保类型检查通过
   - 在真实谱面上测试
   - 检查音效同步准确性

---

## 📄 许可证

本项目遵循 MIT 许可证。

---

## 🙏 致谢

- **ADOFAI 游戏** - 灵感来源
- **Rhythm Doctor** - 游戏支持
- **Tauri 团队** - 优秀的跨平台框架
- **所有谱师** - 创造了精彩的谱面

---

## 📞 联系方式

- **Issues**: 在 GitHub 提交问题
- **Discussions**: 参与社区讨论

---

<div align="center">

**用音乐感受节奏，用代码创造体验 🎵**

Made with ❤️ by ADOFAI Community

</div>
