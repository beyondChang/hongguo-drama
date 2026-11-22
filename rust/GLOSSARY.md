# 术语表

原实现的领域术语 → Rust 侧的含义与命名。单元移植时新增术语请追加到此表，保证三端与各单元命名一致。

## 剧集与来源

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 剧 / series | 一部短剧，含多集 | `Series` |
| 集 / episode | 剧的一集 | `Episode` |
| 剧集号 / episodeNo | 集在剧内的序号 | `episode_no: u32` |
| 来源 / source | 取片源的渠道：红果 App、官网、红果海外、喜番、河马、ReelShort、DramaBox、自定义 | `SourceKind` |
| 片源 / mediaSource | 某集的一个可播放地址候选（含清晰度、加密信息） | `MediaSource` |
| 备用片源 | 首选片源不可用时的受检回退候选 | `MediaSource` 列表中的次项 |
| 画质 / quality | 1080P / 720P / 兼容片源 | `Quality` |
| 分类 / category | 榜单分类 | `Category` |
| 榜单 / ranking | 热播榜等有序列表 | `Ranking` |
| 换一批 | 沿当前来源与分类真实翻页，不打乱榜单顺序 | 分页参数变更 |

## 账号与身份

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 匿名身份 | 播放取源使用的独立匿名设备身份 | `AnonymousIdentity` |
| 账号会话 | 红果账号的登录态与加密存储 | `AccountSession` |
| 设备注册 | 上游设备标识注册，含 10 秒冷却 | `DeviceRegistration` |
| 登录方式 | 网页登录、扫码登录 | `LoginMethod` |
| 身份签名 | 按主机、query、正文与时间戳计算的请求签名 | `RequestSignature` |

## 播放

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 取源 | 为某集取得可播放地址的全过程 | `resolve_source` |
| V5 取源 | player 签名族的无正文 V5 GET 取源 | `PlaybackV5` |
| 解密元数据 | 加密片源所需的密钥与算法信息 | `DecryptionMeta` |
| CENC 解密 | MP4 通用加密（senc/saiz/saio）逐采样解密 | `cenc_decrypt` |
| 兼容片源 | 无法解密时使用的低清直链 | `Quality::Compatible` |
| 降档 | 1080P 不可用时依次尝试 720P、兼容片源 | 降级序列 |
| 固定画质 | 用户手动指定画质后不自动降级 | `QualityPolicy::Fixed` |
| 兼容转码 | ByteVC2 等格式的软解转码 | `transcode_compatible` |
| 硬件编码 | VideoToolbox / NVENC / AMF 优先编码 | `HardwareEncoder` |
| 首帧 | 视频首帧渲染完成 | `first_frame` |
| 保持帧 | 切集时保留当前画面，避免黑屏 | `keep_frame` |
| 画中画 | 独立小窗播放 | `PictureInPicture` |
| 连播 | 一集结束自动播下一集 | `autoplay_next` |

## 下载与文件

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 下载任务 | 一个待完成的下载单元 | `DownloadTask` |
| 任务状态 | queued / running / paused / completed / failed | `TaskStatus` |
| 文件身份 | dev/ino/birthtime 组合，用于防误删与防替换 | `FileIdentity` |
| 临时文件 | 下载中的未完成文件（带 .part 语义） | `DownloadTemporary` |
| 挂载身份 | NAS 挂载点的标识，变化时暂停队列 | `MountIdentity` |
| 合并 | 用 librempeg 合并多集 | `merge` |
| 合并计划 | 合并的目标路径与参数 | `MergePlan` |
| 目录身份 | 分集目录的标识 | `DirectoryIdentity` |
| 本地库 | 已下载内容的本地索引 | `LocalLibrary` |
| 观看记录 | 播放进度与已看状态 | `WatchHistory` |
| 续作 | 同来源同剧名的后续作品 | `Sequel` |
| 观看记录分组 | 按季归组的观看记录 | `HistoryGroup` |
| 隐藏低季 | 不在续作提醒中的低季 | `hidden_low_season` |
| NAS 导出 | 把下载内容导出到 NAS 目录 | `NasExport` |
| 文字导出 | 导出剧集信息文本与 NFO | `text_export` |

## 存储与进程

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 原子写 | 先写临时文件再改名，避免半写状态 | `atomic_write` |
| 便携模式 | 数据随程序目录存放 | `PortableMode` |
| 存储根 | 数据目录（DATA_DIR / userData） | `data_root` |
| 下载根 | 下载目录（DOWNLOAD_DIR） | `download_root` |
| 错误文案 | 面向用户的错误描述 | `ErrorText` |
| 明确拒绝 | 上游主动拒绝请求（如 BUSINESS_REJECTED） | `explicit_rejection` |
| 共享冷却 | 明确拒绝后全队列暂停 | `shared_cooldown` |

## 三端外壳

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 主进程 | Electron 主进程 | `hongguo-desktop`（Tauri 2 Rust 后端） |
| preload 契约 | 主进程暴露给渲染进程的 API 集合 | `desktop/preload-compat` 适配层 |
| Web 端 | Docker 部署的 HTTP 服务 | `hongguo-server`（axum） |
| 安卓桥 | WebView JS 与原生能力的桥 | `hongguo-android`（cdylib + JNI） |
| 文件令牌 | 安卓侧临时媒体文件的登记标识 | `media_token` |
| 远程焦点 | TV 遥控方向键的焦点移动算法 | `next_focus` |

## 迁移工程用语

| 原术语 | 含义 | Rust 命名 |
|---|---|---|
| 单元 / unit | 一个待移植的原模块，对应一个可独立桥接与 gate 的 crate | `crates/hongguo-<name>` |
| 桥接 / bridge | 把原模块的导出函数暴露为原生调用的测试夹具 | `index.cjs` + `crate-type = ["cdylib", "rlib"]` |
| 录制 / record | 采集原模块在真实与随机输入下的返回值与错误，作为等价性基准 | `cases.jsonl` |
| 差分 / diff | 用录制用例逐条比对 Rust 移植的输出 | `rust-anything diff` |
| 闸门 / gate | 一组就绪判据：格式、静态检查、测试、体积、行为等价等 | `rust-anything gate` |
| 影随 / shadow | 让原模块与 Rust 移植并排运行的对照模块 | `shadow/*.shadow.ts` |
| JS 语义层 | `undefined` 与 `null`、真值判断、强制转换、展开、集合比较、排序等 | `hongguo-js` |
| 共享域逻辑 | 多个原模块共用的业务解析（季号、来源、标题归一） | `hongguo-core` |
| undefined 标记 | 结构化值中代表 JS `undefined` 的 `{"$undefined": true}` | `hongguo_js::undefined()` |
| 默认参数语义 | JS 默认参数只对 `undefined` 生效，`null` 原样保留 | ADR-013 |
