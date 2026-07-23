<div align="center">
  <img src="src-tauri/icons/128x128.png" alt="Codex Beacon Logo" width="96" height="96">

  <h1>Codex Beacon</h1>

  <p>Windows 上的 Codex 任务灵动岛。离开 Codex 窗口，也能看见任务是否在运行、用了多久、在哪个项目以及最终回复。</p>

  <p>
    <img alt="Version" src="https://img.shields.io/badge/version-0.4.0-55E49B">
    <img alt="License" src="https://img.shields.io/badge/license-MIT-55E49B">
    <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D4">
    <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB">
    <img alt="React" src="https://img.shields.io/badge/React-19-61DAFB">
  </p>
</div>

> Codex Beacon 是独立社区项目，与 OpenAI 无隶属、合作或官方背书关系。Codex、OpenAI 及相关商标归其各自权利人所有。

## 下载与安装

前往 [GitHub Releases](../../releases/latest) 下载 `Codex Beacon_0.4.0_x64-setup.exe`，运行安装程序即可。

首次启动后需要在 Codex 的“设置 → Hooks”或 CLI `/hooks` 中审核并信任 Codex Beacon Hook。所有任务状态都保存在本机。

## 产品定位

Codex Beacon 只服务一个场景：在 Windows 桌面顶部持续呈现真实的 Codex 任务状态。

它通过 Codex Hook 在本机接收任务生命周期事件，并只读解析对应的本地 Codex rollout 来补充当前活动；不使用进程或 CPU 占用猜测状态，也不会把任务内容上传到外部服务。

## 当前功能

| 功能 | 说明 |
| --- | --- |
| 折叠态任务条 | 显示任务标题、项目、状态和耗时。 |
| 展开态会话面板 | 显示真实提示词、工作目录、模型、会话标识和最后回复。 |
| 实时活动 | 从本地 Codex rollout 显示最近动作、计划步骤以及待批准/待回复状态，不伪造总体完成百分比。 |
| 回到对话 | 使用任务 `session_id` 精确打开对应的 Codex Desktop 对话。 |
| 审批提醒 | 检测需要批准的操作并切换为琥珀色提醒，点击“前往批准”回到 Codex 完成审核。 |
| 最近任务 | 按 `session_id + turn_id` 保留最近 6 个 Codex 任务，支持并行任务识别。 |
| 状态动画 | 运行、完成、失败和长时间失联采用不同的克制反馈。 |
| 本地 Hook 安装 | 一键安装或修复 Codex Hook，并保留用户现有的非 Codex Beacon Hook。 |
| Windows 集成 | 始终置顶、系统托盘、开机启动、单实例和透明点击穿透。 |

## 使用方法

1. 启动 Codex Beacon。
2. 展开灵动岛，点击“连接 Codex”；也可以在设置中执行“安装/重装 Codex Hook”。
3. 完全退出并重新打开 Codex，在“设置 → Hooks”中审核并信任 Codex Beacon Hook。CLI 中也可使用 `/hooks`。
4. 在 Codex 中发送一条新任务。首次真实事件到达后，灵动岛会从“等待审核”切换为“已验证”。
5. 后续任务会自动显示标题、工作项目、耗时、当前活动、完成状态与最后回复摘要。
6. 任务需要批准或回复时，点击“前往批准”或“回到对话”即可打开对应的 Codex Desktop 对话。

命令行修复 Hook：

```powershell
codex-beacon.exe --install-codex-hooks
```

## 环境要求

- Windows 10 或 Windows 11
- Node.js
- pnpm
- Rust 与 Cargo
- Microsoft Visual Studio Build Tools，包含 C++ 桌面开发工作负载
- Microsoft Edge WebView2 Runtime
- 已安装 Codex

## 本地开发

```powershell
pnpm install
pnpm tauri dev
```

常用验证命令：

```powershell
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

完整构建会生成 Windows 可执行文件与安装包：

```text
src-tauri/target/release/codex-beacon.exe
src-tauri/target/release/bundle/nsis/Codex Beacon_0.4.0_x64-setup.exe
```

## 本地数据

```text
%APPDATA%\app.codexbeacon.desktop\codex-status.json
%APPDATA%\app.codexbeacon.desktop\codex-hook-verification.json
%APPDATA%\app.codexbeacon.desktop\codex-beacon-event.ps1
%USERPROFILE%\.codex\hooks.json
%USERPROFILE%\.codex\config.toml
```

- `codex-status.json` 最多保留 6 条会话状态。
- `codex-hook-verification.json` 只在 Codex 实际执行 Hook 后生成，用于区分“文件已配置”和“连接已验证”。
- 安装或重装 Hook 会清除旧验证标记，下一条真实 Codex 任务会重新完成验证。
- `codex-beacon-event.ps1` 只接收 Codex Hook 的标准输入并原子写入状态文件。
- 运行中的任务只读访问 `%USERPROFILE%\.codex\sessions` 下对应的 rollout；Codex Beacon 不修改会话记录。
- 安装器只替换 Codex Beacon 管理的 Hook，其他 Hook 保持不变。
- 首次启动或修复 Hook 时会把旧数据目录中的任务状态迁移到 `app.codexbeacon.desktop`，旧 Hook 信任不会沿用。
- 安装器会在 `config.toml` 的 `[features]` 中显式启用 `hooks = true`，并保留其他设置。
- 开机启动使用 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`。

## 项目结构

```text
.
├── src/
│   ├── App.tsx                     # 状态与界面组合入口
│   ├── codex/                      # Codex 类型、展示转换和 Tauri API
│   ├── desktop/                    # Windows 桌面 API
│   ├── components/                 # 任务、空状态和设置组件
│   ├── styles/                     # 按界面区域拆分的视觉样式
│   ├── App.css                     # 样式导入入口
│   └── main.tsx
├── scripts/
│   └── codex-beacon-event.ps1      # Codex 生命周期事件写入
├── src-tauri/
│   ├── src/codex.rs                # Codex 模块入口与数据模型
│   ├── src/codex/                  # 状态、rollout、Hook 与存储模块
│   ├── src/desktop.rs              # Windows 窗口、托盘与开机启动
│   ├── src/lib.rs                  # Tauri 组合入口
│   ├── src/main.rs
│   └── tauri.conf.json
└── package.json
```

## 已知边界

- 当前只支持 Windows 和 Codex。
- 当前通过稳定的 `UserPromptSubmit` 与 `Stop` Hook 获取任务开始和完成事件，并从本地 rollout 补充最近动作与计划步骤；计划计数不等于总体完成百分比。
- Codex Desktop 的审批通道没有向第三方窗口开放。Codex Beacon 只检测待批准状态并精确跳回对应对话，最终批准仍由 Codex 完成。
- “回到对话”依赖 Codex Desktop 当前提供的 `codex://threads/{threadId}` 协议。
- Codex 异常退出时可能收不到 `Stop`，运行状态超过 2 小时后会标记为“连接中断”。

## 许可证

本项目使用 [MIT License](LICENSE)。

## 作者

[0731koukou](https://github.com/0731koukou)
