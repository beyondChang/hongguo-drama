# 红果短剧 Rust 重构：迁移地图

本文件是跨会话的迁移状态载体：目标、ADR 索引、每个单元的状态（open / claimed by <会话> <日期> / done）、待决问题、以及明确不在范围内的部分。

它只指向其他文件，不复制其内容。

- 决策：见 `DECISIONS.md`（ADR-001 至 ADR-013）
- 术语：见 `GLOSSARY.md`
- 依赖调研：见 `research/`
- 单元代码：见 `crates/`
- 共享语义与共享域逻辑：见 `crates/hongguo-js`、`crates/hongguo-core`（ADR-012）
- 应用外壳：见 `crates/hongguo-desktop`、`crates/hongguo-server`、`crates/hongguo-android`
- 交接记录：见 `HANDOFF.md`

## 目标

把现有 Node/Electron/JavaScript 实现迁移到 Rust，产出三端可构建产物：桌面端（Tauri 2）、Web/Docker 端（axum）、安卓/TV 端（JNI + 现有 WebView 壳）。原 JS 实现保留为参考，本轮不发行（ADR-006）。

## 工作策略

先打通一条端到端纵切（ADR-009），再按波次铺开剩余单元。每单元完成后必须 `rust-anything gate` 通过才可标为 done。

## 纵切路径（第一阶段）

按依赖自底向上，每步都要有 Rust 用例与 gate 结果。

| 步骤 | 内容 | 状态 |
|---|---|---|
| L0 | workspace 骨架 + 三层 crate 骨架可构建 | done |
| L1 | 共享核心：纯逻辑单元（无 Electron/Node 依赖） | in progress（12/63） |
| L2 | 共享核心：取源与签名链路 | open |
| L3 | 共享核心：存储、下载队列、本地库 | open |
| L4 | 桌面端：Tauri 2 外壳 + IPC 兼容层，接通核心 | in progress（纵切骨架 done：命令层 3 项单测通过；Tauri bin feature-gated，未构建真实窗口） |
| L5 | Web 端：axum 服务 + 保留前端，接通核心 | in progress（纵切骨架 done：axum 服务真实运行，health/认证/同源/安全头/2 单元路由通过 HTTP 验收） |
| L6 | 安卓端：Rust cdylib + JNI 桥，接通核心 | in progress（纵切骨架 done：cdylib 导出函数 host FFI 往返 3 项通过；未做 cargo-ndk 交叉编译） |
| L7 | 纵切验收：三端同一用例集全绿 | in progress（三端各自接通 `hongguo-app-update` + `hongguo-window-layout` 同一用例；完整用例集待铺开） |

## Crate 分层（ADR-012）

| crate | 类型 | 依赖 | 职责 |
|---|---|---|---|
| `hongguo-js` | rlib | `serde_json`、`regex` | JavaScript 语义：`undefined`/`null` 区分、真值判断、数值与字符串强制转换、对象展开、`Set` 按值比较、V8 TimSort 排序 |
| `hongguo-core` | rlib | `hongguo-js` | 共享域逻辑：剧名季号解析、来源判定、标题归一与相似度、合并计划（`merge_plan`） |
| `hongguo-<unit>` | cdylib + rlib | `hongguo-js`，按需 `hongguo-core` | 单个原模块的移植，可独立桥接与 gate；体积超限时把域逻辑下沉到 `hongguo-core` |
| `hongguo-desktop` / `hongguo-server` / `hongguo-android` | 各自形态 | 所需单元 crate | 三端外壳（ADR-002/004/005） |

单元 crate 一律 `crate-type = ["cdylib", "rlib"]`：cdylib 供 rust-anything 桥接与安卓 JNI 使用，rlib 供其他单元 crate 与三端外壳直接依赖。

## 已完成单元

| 单元 | crate | 用例 | gate | 备注 |
|---|---|---|---|---|
| `src/native/watch-history.js` | `hongguo-watch-history` | 1123 录制 + 120787 fuzz | 全绿（12 项） | 11 个导出；季号解析与来源判定已下沉到 `hongguo-core` |
| `src/native/download-quality.js` | `hongguo-download-quality` | 424 录制 + 9537 fuzz | 全绿（12 项） | 2 个导出；`parseInt`/`toInteger` 下沉到 `hongguo-js` |
| `src/native/error-text.js` | `hongguo-error-text` | 402 录制 + 1633 fuzz | 全绿（12 项） | 2 个导出；`sanitizeLogValue` 因 `instanceof Error` 留在 JS |
| `src/native/merge-plan.js` | `hongguo-merge-plan` | 284 录制 + 2 fuzz | 全绿（12 项） | 3 个导出；实现下沉到 `hongguo-core::merge_plan`（含 ffprobe 签名与码率规划） |
| `src/native/media-sources.js` | `hongguo-media-sources` | 244 录制 + 535 fuzz | 全绿（12 项） | 2 个导出；内置来源开关与自定义来源 |
| `src/native/stealth-region.js` | `hongguo-stealth-region` | 62 录制 + 0 fuzz | 全绿（12 项） | 2 个导出；小窗唤醒区域校验（fuzz 生成不出 4 键对象） |
| `src/native/ui-language.mjs` | `hongguo-ui-language` | 494 录制 + 57938 fuzz | 全绿（12 项） | 4 个导出；语言解析/系统区域/偏好读写，`js_trim` 下沉到 `hongguo-js` |
| `src/native/child-env.js` | `hongguo-child-env` | 9 录制 + 6 fuzz（显式 env） | 全绿（12 项） | 1 个导出；删除代理环境变量；默认 `env = process.env` 路径读活环境，三端只传显式 env，故按空对象处理 |
| `src/native/app-update.js` | `hongguo-app-update` | 114 录制 + 83 fuzz | 全绿（12 项） | 1 个导出；版本比较与发布判断下沉到 `hongguo-core::app_update`，`encodeURIComponent` 下沉到 `hongguo-js` |
| `src/native/window-layout.js`（经 `_wrapper-window-layout.js`） | `hongguo-window-layout` | 50 录制 + 6618 fuzz | 全绿（12 项） | 2 个纯函数导出；`saveWindowZoom`/`restore`/`remember` 回调控制器归 L4；wrapper 只转接纯函数 |
| `src/native/hardware-encoder.js`（经 `_wrapper-hardware-encoder.js`） | `hongguo-hardware-encoder` | 13 录制 + 5 fuzz | 全绿（12 项） | 仅桥接 `hardwareEncoderArgs` 纯映射；异步子进程 `pickHardwareEncoder` 归 L4 |
| `src/native/danmaku.js`（经 `_wrapper-danmaku-pure.js`） | `hongguo-danmaku-pure` | 78 录制 + 6179 fuzz | 全绿（12 项） | 6 个纯函数；逻辑下沉到 `hongguo-core::danmaku`，`asText` 下沉到 `hongguo-js`；网络/缓存/设备身份归 L2/L5 |

## 三端纵切验收（第一阶段，ADR-009）

三端外壳均接通同一批已 gate 单元（`hongguo-app-update`、`hongguo-window-layout`，桌面另接 `hongguo-media-sources`、`hongguo-download-quality`），用同一组输入验证行为一致：

| 外壳 | crate | 验收方式 | 结果 |
|---|---|---|---|
| 桌面 Tauri 2 | `hongguo-desktop` | `cargo test -p hongguo-desktop`：命令层单测 3 项 | 全过：zoom 钳制/reset、来源固定红果不可关、画质与版本委托 |
| Web axum | `hongguo-server` | 真实启动 + 本机 HTTP 请求 | 全过：`/health` 200、无/错令牌 401、跨源 403、安全响应头齐全、`/api/window-scale` 钳制、`/api/release` 版本判断与 draft 拒绝 |
| 安卓 JNI | `hongguo-android` | `cargo test -p hongguo-android`：host FFI 往返 3 项 | 全过：windowScale 往返/钳制、release 判断、null 入参安全 |

边界：桌面 Tauri bin 用 `--features tauri-runtime` 门控，本轮未构建真实 WebView 窗口；安卓未做 cargo-ndk 交叉编译，仅 host 验证 C ABI 导出。纵切证明"三端外壳 → 同一批 Rust 单元"链路可构建、可调用、行为一致，不等于三端完整功能移植完成。

## 单元清单

来源：`rust-anything analyze --whole-app` 于 2026-11-22 的输出。

### 共享核心（三端共用）

| 单元 | 原文件 | 行数 | 备注 | 状态 |
|---|---|---|---|---|
| core/search-history | `src/searchHistory.js` | 41 | 纯逻辑，analyze start_here | open |
| core/shortcuts | `src/shortcuts.js` | 91 | 纯逻辑，analyze start_here | open |
| core/theme | `src/theme.js` | 44 | 纯逻辑，analyze start_here | open |
| core/local-library | `src/localLibrary.js` | 81 | 纯逻辑，analyze start_here | open |
| core/player-display | `src/playerDisplay.mjs` | 12 | 纯逻辑 | open |
| core/software-video | `src/softwareVideo.mjs` | 145 | 纯逻辑 | open |
| core/video-enhancement | `src/videoEnhancement.js` | 147 | 依赖 anime4k-webgpu | open |
| core/native-sm3 | `src/native/sm3.js` | 37 | 纯 JS SM3，桌面与安卓共用 | open |
| core/native-danmaku-sign | `src/native/danmaku-sign.js` | 388 | 签名 | open |
| core/native-playback-signature | `src/native/playback-signature.js` | 144 | 签名 | open |
| core/native-playback-v5 | `src/native/playback-v5.js` | 305 | 取源 | open |
| core/native-hongguo | `src/native/hongguo.js` | 1219 | 核心客户端，含 CENC 解密 | open |
| core/native-hongguo-search | `src/native/hongguo-search.js` | 245 | 搜索 | open |
| core/native-hongguo-app-search | `src/native/hongguo-app-search.js` | 103 | App 搜索 | open |
| core/native-hongguo-app-catalog | `src/native/hongguo-app-catalog.js` | 165 | App 目录 | open |
| core/native-hongguo-app-new | `src/native/hongguo-app-new.js` | 229 | App 详情 | open |
| core/native-public-details | `src/native/public-details.js` | 79 | 官网详情 | open |
| core/native-app-source | `src/native/app-source.js` | 649 | 取源编排 | open |
| core/native-media-sources | `src/native/media-sources.js` | 21 | 纯数据 | done |
| core/native-episode-panel | `src/native/episode-panel.js` | 51 | 分集面板 | open |
| core/native-episode-comments | `src/native/episode-comments.js` | 31 | 评论 | open |
| core/native-account-session | `src/native/account-session.js` | 438 | 账号会话 | open |
| core/native-account-controller | `src/native/account-controller.js` | 213 | 账号控制 | open |
| core/native-danmaku | `src/native/danmaku.js` | 584 | 弹幕 | open |
| core/native-watch-history | `src/native/watch-history.js` | 168 | 观看记录 | done |
| core/native-history-service | `src/native/history-service.js` | 301 | 记录分组 | open |
| core/native-series-search | `src/native/series-search.js` | 75 | 续作 | open |
| core/native-catalog | `src/native/catalog.js` | 257 | 榜单，唯一依赖 electron 的 native 模块 | open |
| core/native-download-quality | `src/native/download-quality.js` | 17 | 画质判定 | done |
| core/native-merge-plan | `src/native/merge-plan.js` | 43 | 合并计划 | done |
| core/native-merge-runner | `src/native/merge-runner.js` | 210 | 调用 librempeg | open |
| core/native-request-signal | `src/native/request-signal.js` | 43 | 取消 | open |
| core/native-child-env | `src/native/child-env.js` | 10 | 子进程环境 | open |
| core/native-error-text | `src/native/error-text.js` | 57 | 错误文案 | done |
| core/native-ipc-boundary | `src/native/ipc-boundary.js` | 36 | IPC 白名单 | open |
| core/native-text-export | `src/native/text-export.js` | 180 | 文字导出 | open |
| core/native-download-task-view | `src/native/download-task-view.js` | 57 | 任务视图 | open |
| core/native-file-identity | `src/native/file-identity.js` | 88 | 文件身份 | open |
| core/native-atomic-json | `src/native/atomic-json.js` | 65 | 原子写 | open |
| core/native-portable-data | `src/native/portable-data.js` | 184 | 便携模式 | open |
| core/native-proxy-config | `src/native/proxy-config.js` | 176 | 代理配置 | open |
| core/native-proxy-manager | `src/native/proxy-manager.js` | 91 | 代理管理 | open |
| core/native-web-media-guard | `src/native/web-media-guard.js` | 162 | 域名白名单 | open |
| core/native-window-layout | `src/native/window-layout.js` | 81 | 窗口布局 | open |
| core/native-stealth-region | `src/native/stealth-region.js` | 17 | 隐藏区域 | done |
| core/native-stealth-window | `src/native/stealth-window.js` | 107 | 隐藏窗口 | open |
| core/native-boss-key | `src/native/boss-key.js` | 122 | 老板键 | open |
| core/native-renderer-security | `src/native/renderer-security.js` | 38 | 渲染进程安全 | open |
| core/native-ui-language | `src/native/ui-language.mjs` | 18 | 系统语言 | done |
| core/native-download-temporary | `src/native/download-temporary.js` | 242 | 临时文件 | open |
| core/native-bytevc2 | `src/native/bytevc2.js` | 164 | 兼容转换 | open |
| core/native-hardware-encoder | `src/native/hardware-encoder.js` | 77 | 硬件编码探测 | open |
| core/native-hls-media | `src/native/hls-media.js` | 166 | HLS | open |
| core/native-media-access | `src/native/media-access.js` | 74 | 媒体访问 | open |
| core/native-playback-device | `src/native/playback-device.js` | 117 | 播放设备注册 | open |
| core/native-custom-source | `src/native/custom-source.js` | 219 | 自定义源 | open |
| core/native-xifan | `src/native/xifan.js` | 151 | 喜番 | open |
| core/native-hema | `src/native/hema.js` | 172 | 河马 | open |
| core/native-reelshort | `src/native/reelshort.js` | 167 | 海外源 | open |
| core/native-dramabox | `src/native/dramabox.js` | 126 | 海外源 | open |
| core/native-nas-export | `src/native/nas-export.js` | 370 | NAS 导出 | open |
| core/native-app-announcements | `src/native/app-announcements.js` | 184 | 公告 | open |
| core/native-floating-player | `src/native/floating-player-window.js` | 256 | 画中画窗口 | open |
| core/store | `src/store.js` | 417 | 与 native 循环依赖，需一起移植 | open |

### 桌面端外壳（Tauri 2）

| 单元 | 原文件 | 行数 | 状态 |
|---|---|---|---|
| desktop/main | `main.js` | 5509 | in progress（命令层骨架 + Tauri bin feature-gated；真实窗口与完整 IPC 未移植） |
| desktop/preload-compat | `preload.js` | 269 | open（`window.*` 逐通道适配未做） |

### Web/Docker 端

| 单元 | 原文件 | 行数 | 状态 |
|---|---|---|---|
| server/index | `server/index.cjs` | — | in progress（axum 纵切骨架：health/status/window-scale/release + 安全契约；任务队列、媒体代理未移植） |
| server/storage | `server/storage.cjs` | — | open |
| server/media | `server/media.cjs` | — | open |
| server/search | `server/search.cjs` | — | open |
| server/live-check | `server/live-check.cjs` | — | open |

### 安卓/TV 端

| 单元 | 原文件 | 行数 | 状态 |
|---|---|---|---|
| android/bridge | `android/mobile/bridge.js` | 448 | open |
| android/web-public | `android/web/public.js` | 64 | open |
| android/web-media | `android/web/media.js` | 118 | open |
| android/web-transport | `android/web/transport.js` | 54 | open |
| android/tv | `android/web/tv.js` 等 | — | open |

### 保留在原语言（不移植）

| 单元 | 位置 | 行数 | 原因 |
|---|---|---|---|
| ui/components | `src/components/` | 11720 | React UI（ADR-002） |
| ui/app | `src/App.jsx` | 288 | React UI |
| ui/main | `src/main.jsx` | 16 | React UI |
| ui/i18n | `src/i18n/` | 74 | 界面文案 |
| ui/server-public | `server/public/app.js` | 401 | Web 前端（ADR-004） |
| scripts | `scripts/` | 22882 | 构建与验收脚本 |

## 待决问题

- 浮点容差：若核心数值逻辑（合并进度、时长、字节统计）无法逐位一致，需用户确认 `--float-tolerance` 数值（ADR-007）。
- Tauri 2 的具体依赖（WebView2 / webkitgtk）在 Linux 上的构建环境要求，取决于本机是否需要产出 Linux 产物。
- `hongguo-core` 是否需要同时产出 `cdylib` 供安卓复用：按 ADR-012，安卓复用走 `hongguo-android`（cdylib 包装），`hongguo-core` 保持 rlib；需确认 cargo-ndk 交叉编译可行性。
- 桥接开销：rust-anything 的 napi/JSON 桥对微小函数比 JS 原版慢约 5 倍（`sourceOf` 77ns → 433ns），重逻辑则更快（`listHistory` 6.9x）。三端外壳若走 Tauri IPC / HTTP / JNI，需实测边界开销是否可接受。

## 不在范围内

- 发行与发布（ADR-006）：不产出正式安装包，不改现有发行流程，不对外发布。
- 现有 7.9 用户数据迁移（ADR-010）。
- React UI 重写为原生 GUI。
- Web 前端 `server/public/app.js` 重写。
- 现有 JS 代码删除（本轮全程保留为参考）。
