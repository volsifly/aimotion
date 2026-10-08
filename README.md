# AI Motion

给 Agent 一块不预设图案的 16×16 像素屏幕，观察它在文字之外会怎样表达。GPUI 桌面版以约 246×339 的小窗口常驻桌面，发布回复时自动更新画面。

项目采用 [MIT License](LICENSE)。

## 桌面应用

当前桌面版支持 Linux X11，以及安装了 XWayland 的 Wayland 桌面。使用 [GPUI](https://gpui.rs/) 0.2.2 和 Rust 1.89。需要 Vulkan 驱动、X11/XKB 和 fontconfig 运行库；发行版的开发环境可通过 `libxkbcommon-dev libxkbcommon-x11-dev libfontconfig1-dev` 等包补齐。

```sh
./scripts/run-desktop.sh
```

窗口保持置顶，并在所有工作区显示，不抢占输入焦点。像素画面下方默认显示回复文字，长内容可滚动查看。左键拖动位置，中键退出。通知区域的 AI Motion 托盘菜单可显示、隐藏窗口或退出；托盘可用时窗口不出现在任务栏中。GNOME 需要启用 AppIndicator 扩展，托盘不可用时保留任务栏入口。程序通过文件锁保证同一数据目录只有一个实例，退出后下次发布会重新打开。

构建后可安装应用菜单入口和登录自启动：

```sh
python3 scripts/install-desktop.py
```

安装项位于 `~/.local/share/applications/ai-motion.desktop` 和 `~/.config/autostart/ai-motion.desktop`。删除对应文件即可取消安装或自启动；安装后的仓库路径需保持有效。

## 像素协议

- `1` 表示亮，`0` 表示灭。
- 每个四位十六进制数编码一行 16 个像素，最高位是最左侧像素。
- 每 16 个数构成一帧，依次对应第 1～16 行。多个帧按序循环。
- 数字可用空格、换行、逗号、分号或竖线分隔，也支持 `0x` 前缀。

渲染器会将每个亮像素匹配到下一帧最近的亮像素，并沿网格逐格移动，每格 20ms。没有匹配目标的像素立即熄灭，新出现的像素立即点亮，不使用淡入淡出。像素移动完成后等待 300ms，再开始下一帧。

## 发布回复

创建 UTF-8 JSON 文件：

```json
{
  "text": "要显示的回复内容",
  "sequence": "8000 4000 2000 1000 0800 0400 0200 0100 0080 0040 0020 0010 0008 0004 0002 0001"
}
```

每帧必须恰好包含 16 个四位十六进制数。运行：

```sh
python3 publish.py /path/to/reply.json
```

脚本会校验输入、原子更新本机 `current.json`，自动启动或更新桌面窗口，返回 `desktop: true`。窗口每 200ms 检查新回复，动画移动期间每 20ms 前进一格，完成后停留 300ms。新回复会从正在显示的像素位置开始过渡。

`current.json`、`reply.json`、桌面状态和日志是本机运行数据，不纳入版本控制。

原网页可通过显式浏览器模式使用：

```sh
python3 publish.py --browser /path/to/reply.json
```

浏览器模式在 `127.0.0.1:8765` 提供页面并返回展示网址。

展示网址中的 `reply` 参数不会保存历史快照。页面始终读取最新的 `current.json`，旧网址再次打开时也会显示最新回复。

## Codex Skill

`skills/aimotion-reply/` 是对应的 Codex Skill。克隆仓库后，可以用符号链接安装，使 Skill 能从自身位置找到项目目录：

```sh
ln -s "$PWD/skills/aimotion-reply" ~/.codex/skills/aimotion-reply
```

如果改为复制安装，请在启动 Codex 前设置 `AIMOTION_ROOT` 为仓库的绝对路径。Skill 会按像素协议发布每次回复。
