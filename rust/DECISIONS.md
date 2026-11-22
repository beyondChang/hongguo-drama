# 红果短剧 Rust 重构：决策记录（ADR）

本文件是本次迁移的事实来源。每条决策编号后不再重编；决策变更时新增 ADR 并标注取代关系。

范围：把现有 Node/Electron/JavaScript 实现迁移到 Rust，产出桌面端（Tauri 2）、Web/Docker 端（axum）、安卓/TV 端（JNI + 现有 WebView 壳）三个可构建产物。现有 JS 实现保留为参考，不在本轮发行。

## ADR-001: Rust 代码放在 rust/ 单一 Cargo workspace
Status: accepted (2026-11-22)
Context: 项目根已有 Electron/React/Node 全套构建链（package.json、electron-builder、vite）。Rust 需要一个独立目录，不污染现有 JS 构建与打包配置。
Decision: 新建 `rust/`，使用单一 Cargo workspace，包含 `hongguo-core`（三端共享核心库）、`hongguo-desktop`（Tauri 2）、`hongguo-server`（axum）、`hongguo-android`（JNI cdylib）四个 crate。
Consequences: `rust/` 不进 electron-builder 的 files 列表，现有桌面发行不受影响；Rust 侧统一 `cargo build` 即可构建全部三端；workspace 与现有 package.json 无耦合。

## ADR-002: 桌面端用 Tauri 2，保留 React 渲染进程
Status: accepted (2026-11-22)
Context: `main.js`（5509 行）是 Electron 主进程，含窗口、下载队列、取源、播放、设置、代理等逻辑；`src/components/`（11720 行）是 React UI，由 `preload.js`（269 行）暴露 `window.*` API。UI 代码量大且为 React/JSX，rust-anything 分析判定为 keep。
Decision: 主进程逻辑全部改写为 Rust + Tauri 2 命令；渲染进程保留现有 React 组件。Tauri 2 的 WebView 依赖为：Windows WebView2、macOS WKWebView、Linux webkitgtk。
Consequences: 现有 electron-builder 发行链（NSIS/DMG/AppImage、macOS 签名公证）在 Rust 版上不适用，需另行重建；本轮不发行（见 ADR-006），但架构上不再依赖 Electron。

## ADR-003: 桌面端写 IPC 兼容层，React UI 代码零改动
Status: accepted (2026-11-22)
Context: `src/components/` 与 `src/App.jsx` 大量直接调用 `window.*`（preload 暴露的契约）。逐个组件改写为 Tauri `invoke` 会引入大量新回归点。
Decision: 新增一层薄适配，把现有 `window.*` 调用在内部转成 Tauri `invoke`，UI 源码不改一行。原 preload 契约逐项对照验收。
Consequences: 兼容层成为必须被单独验证的组件（需对每个通道做等价性测试）；UI 保持可读与可回归。

## ADR-004: Web/Docker 端用 Rust axum，保留现有前端
Status: accepted (2026-11-22)
Context: `server/index.cjs` 是 Node `http` 服务，暴露 `/health`、`/api/status`、`/api/tasks`、`/api/search`、`/api/series/:id`、`/api/files/:uuid`、`/api/downloads`、`/api/merge`、`/api/tasks/:uuid/retry|cancel`；前端为 `server/public/app.js`。Dockerfile 基于 `node:22-bookworm-slim`，暴露 3000。
Decision: 服务端全部路由、Bearer 令牌认证、下载队列、媒体代理改写为 Rust axum；`server/public/` 前端保持不变。HTTP 状态码、请求头、安全响应头、认证协议与存储格式保持兼容。
Consequences: Dockerfile 改为 Rust 基础镜像；`/health` 与 Bearer 令牌协议不变，compose.yaml 的环境变量与卷挂载可沿用。

## ADR-005: 安卓/TV 端用 Rust 核心 + 保留现有 WebView 壳
Status: accepted (2026-11-22)
Context: 安卓端是自研打包链（`android/build.py` 用 aapt2/d8/apksigner，无 Gradle），单 Activity + WebView + `addJavascriptInterface`；`android/mobile/bridge.js`（448 行）是纯 JS 业务桥，直接 import 桌面 `src/native/*`；`android/web/crypto.js`、`zlib.js` 只是 `Native.*` 转发壳；`android-decrypt` 是 esbuild 别名，指向从 `src/native/hongguo.js` 裁剪的 CENC 解密段。
Decision: 把 `src/native/` 核心逻辑与 `android/mobile/bridge.js` 的桥接能力改写为 Rust crate，用 cargo-ndk 交叉编译为 `.so`，由现有 Java MainActivity 经 JNI 调用。保留 WebView 壳、Java Activity、`build.py` 打包链、包名与版本号规则。
Consequences: `android/web/crypto.js`/`zlib.js` 的 Java 原生实现（AES-CTR/CBC/ECB、SHA256/MD5/SHA512、gzip、随机数）可由 Rust crate 取代，Java 侧变薄；CENC 解密三端统一为一份 Rust 实现。

## ADR-006: 本轮只做重构，不做发行
Status: accepted (2026-11-22)
Context: 项目已有 7.9 正式发行（四平台 + Docker + 安卓/TV 独立发版），发行门槛见 `docs/RELEASING.md` 与 AGENTS.md。本轮 Rust 重构体量大，产物尚未稳定。
Decision: 本轮交付物为 `rust/` 内的源码、单元用例、gate 结果与三端可构建产物。不替换现有 7.9 安装包，不改动现有发行流程，不对外发布。是否发版由用户另行决定。
Consequences: 现有 JS 版保持可运行、可发行、可回滚；Rust 版与它并存。

## ADR-007: 等价性标准为归一化后等价
Status: accepted (2026-11-22)
Context: Rust 与 JS 的对象键序、浮点格式化天然不同，字节级一致会产生大量伪缺陷。
Decision: 以 rust-anything 录制的真实用例为准：返回值、错误码、错误文案逐条匹配；HTTP 状态码、请求头、存储文件格式保持不变；仅键顺序、空白、时间戳等噪声允许归一化。浮点若无法逐位一致，需用户确认容差（`--float-tolerance`）后才放宽。
Consequences: 差异必须写进 `known-diffs.jsonl` 并附原因，经用户批准；不得为消除差异而放宽录制。

## ADR-008: 等价性用例以现有测试套件为主
Status: accepted (2026-11-22)
Context: 仓库已有 80 个 `node:test` 文件（主包 `scripts/check-*.cjs` 62 个 + `android/tests/` 18 个），已刻画下载、合并、解密、签名、来源拒绝、挂载变化等真实分支。
Decision: 用 `rust-anything record --from-tests` 把这些套件对单元导出函数的调用转成 Rust 用例；再补真实红果素材负载与错误路径用例。
Consequences: 现有套件成为迁移的回归基线；主包与安卓套件都能作为录制源。

## ADR-009: 先做一条端到端纵切，再铺开剩余单元
Status: accepted (2026-11-22)
Context: 21 个分析单元中 6 个为 keep（UI），2 个标为 blocked（实为测试 fixture 中的动态代码，生产代码无碍），共享核心 `src/native/` 与 `src/store.js` 互相依赖需一起移植。
Decision: 先打通一条最小纵切（共享核心 → 桌面端 → server → 安卓端闭环）并验收，再按波次铺开剩余单元。每波结束向用户汇报 gate、diff、用例数与差异。
Consequences: 早期会经历接口调整；纵切验收通过后其余波次可并行交给子代理。

## ADR-010: Rust 版不兼容现有 7.9 的本地数据
Status: accepted (2026-11-22)
Context: 现有 7.9 已发布，用户的设置、观看历史、本地库、下载任务存于 Electron userData 与各持久化文件。Tauri 2 的 userData 路径与 Electron 不同，存储格式也会随 Rust 实现变化。
Decision: Rust 版使用独立的数据目录与存储格式，不读取也不迁移现有 7.9 的用户数据；不写迁移工具。
Consequences: 现有用户换用 Rust 版后本地库、观看历史、下载队列与设置不可继承；本轮不发行（ADR-006），实际影响为零，后续发版时须在发行说明中明确告知。此为已知取舍，非缺陷。

## ADR-011: 安卓端 Rust 核心经 JNI 原生 .so 暴露
Status: accepted (2026-11-22)
Context: 安卓交叉编译目标（aarch64/armv7/i686/x86_64-linux-android）已安装。JS 侧与 Java 侧之间现为 `addJavascriptInterface` 同步调用桥。
Decision: Rust crate 以 `cdylib` 产出 `.so`，由 MainActivity 以 JNI 调用，桥接函数命名与现有 `Native.*` 语义对齐。
Consequences: 大文件 CENC 解密与合并不走 JS 字符串传递，性能优于 WASM；需在 MainActivity 增加 JNI 桥声明与加载逻辑；`build.py` 需增加 `.so` 打包与 ABI 分发步骤。

## ADR-012: crate 分为三层，共享语义与共享域逻辑各自独立成 crate
Status: accepted (2026-11-22)
Context: ADR-001 只规划了 `hongguo-core` + 三端外壳四个 crate。实际移植首个单元 `src/native/watch-history.js` 时发现两类代码被反复重写：一是 JavaScript 语义（`undefined` 与 `null` 的区别、默认参数只对 `undefined` 生效、缺失属性读为 `undefined`、`Set` 按值比较、对象展开的键序、`Array.prototype.sort` 的 V8 TimSort 行为），二是跨模块共享的域逻辑（剧名季号解析、来源判定、标题归一与相似度）。前者每个单元都要用，后者被 `series-search.js`、`nas-export.js`、`history-service.js` 三处消费。两者若留在单元 crate 内，rust-anything 的 size 门槛（源文件行数 3 倍 + 50 行）无法通过，且后续 60 个单元会重复实现同一套语义。
Decision: workspace 分为三层：(1) `hongguo-js`——纯 JavaScript 语义层，只依赖 `serde_json` 与 `regex`；(2) `hongguo-core`——共享域逻辑层（季号解析、来源判定、标题归一与相似度、ffprobe 签名与合并计划），依赖 `hongguo-js`；(3) 单元 crate（`crates/hongguo-<name>`，`crate-type = ["cdylib", "rlib"]`）与三端外壳 crate。单元 crate 一律依赖 `hongguo-js`，只在需要共享域逻辑时再依赖 `hongguo-core`（如 `watch-history`、`merge-plan`；`download-quality`、`error-text` 等纯语义单元不需要）；三端外壳直接依赖所需单元 crate，不再设转发的聚合中间层，以免单元 crate 与聚合 crate 互相依赖成环。实现超出 size 门槛时，域逻辑下沉到 `hongguo-core`、JS 语义下沉到 `hongguo-js`，单元 crate 只留桥接与薄封装。
Consequences: 语义只写一次，所有单元行为一致；域逻辑改动会让多个单元同时受益。单元 crate 保持可独立桥接与独立 gate。`hongguo-core` 依 ADR-001 的原义继续作为"三端共享核心库"，但不重新导出单元端口，三端外壳自行列出依赖的单元 crate。

## ADR-013: 可选参数在 Rust 侧用 `Value` 而非 `Option<Value>` 表达
Status: accepted (2026-11-22)
Context: JavaScript 的默认参数只对 `undefined` 生效，`null` 会原样保留。rust-anything 生成的二进制协议对可选参数使用 `read_dyn_opt`，把 `null` 与 `undefined` 一并折叠为 `None`；`serde_json` 的 `Option<Value>` 同样把 `null` 折叠为 `None`。两者叠加后，移植无法区分"调用方显式传了 `null`"与"调用方没传该参数"，而在 `applyEpisodeUpdate(record, old, fresh, now = Date.now())` 这类接口上二者结果不同（`lastCheckedAt` 分别是 `null` 与当前时间）。
Decision: 凡默认参数存在语义差异的接口，单元 crate 的 port 函数把该参数声明为 `serde_json::Value` 而非 `Option<Value>`。协议层以 `Schema::Dyn` 读取，`undefined` 表现为标记对象 `{"$undefined": true}`，`null` 表现为 `Value::Null`，两者不再折叠。改签名后必须执行 `rust-anything refresh` 重新生成绑定。
Consequences: port 函数签名不再直接反映"参数可选"，可选项由函数体按 JS 规则处理（默认参数只对 `undefined` 生效）。这比机械照搬 `Option<Value>` 更贴近原语义，但要求每个移植单元都核对默认参数的行为。
