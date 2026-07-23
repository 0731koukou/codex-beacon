# GitHub 发布步骤

## 当前本地状态

- 分支：`main`
- Git 作者：`0731koukou`
- 尚未创建首个提交
- 尚未配置远程仓库
- `node_modules`、`dist`、`src-tauri/target`、日志和 `outputs` 已排除

## 一、在 GitHub 创建空仓库

1. 登录 GitHub，点击右上角 `+` → `New repository`。
2. Repository name 填写 `codex-beacon`。
3. Description 使用：

   ```text
   Windows 上的 Codex 任务状态信标：监控运行进度、提示待批准操作，并一键返回对应对话。
   ```

4. 建议选择 `Public`。
5. 不要勾选 README、`.gitignore` 或 License；本地项目已经包含。
6. 点击 `Create repository`。

## 二、提交并推送源码

在 Codex Beacon 项目根目录中运行：

```powershell
git status --short
git add .
git status --short
git commit -m "Initial release: Codex Beacon 0.4.0"
git remote add origin https://github.com/0731koukou/codex-beacon.git
git push -u origin main
```

如果 GitHub 要求登录，按 Git Credential Manager 弹窗完成浏览器授权，不要在命令行粘贴账号密码。

## 三、创建 v0.4.0 Release

先创建并推送标签：

```powershell
git tag -a v0.4.0 -m "Codex Beacon 0.4.0"
git push origin v0.4.0
```

然后进入仓库页面：

1. 点击 `Releases` → `Draft a new release`。
2. Tag 选择 `v0.4.0`。
3. Release title 填写：

   ```text
   Codex Beacon v0.4.0 — 看见进度，及时处理
   ```

4. Release 正文复制 [产品文案](product-copy.md) 中的 `v0.4.0 Release 文案`。
5. 上传以下附件：

   ```text
   outputs/Codex Beacon_0.4.0_x64-setup.exe
   outputs/codex-beacon.exe
   outputs/codex-beacon-product-intro.mp4
   outputs/SHA256SUMS.txt
   ```

6. 点击 `Publish release`。

## 四、发布后检查

- README 顶部图标能够显示。
- `GitHub Releases` 下载链接能够打开。
- 仓库中没有 `node_modules`、`dist`、`src-tauri/target`、`outputs`、日志或本机配置。
- Release 安装包可以下载，SHA-256 与 `SHA256SUMS.txt` 一致。
- About 区域已填写 Description 和 Topics。
