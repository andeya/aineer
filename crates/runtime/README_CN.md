# codineer-runtime

[Codineer](https://github.com/andeya/codineer) 的核心运行时引擎。

[English](README.md)

---

本 crate 实现了会话生命周期、配置加载、系统提示组装、权限管理、沙箱、错误恢复和对话编排。MCP 传输由独立的 `codineer-mcp` crate 处理。

### 文件操作亮点

- **Grep / glob**：基于 ripgrep 核心库（`grep-regex`、`grep-searcher`、`ignore`），支持高性能、`.gitignore` 感知的多行正则搜索。
- **读取**：支持文本文件、PDF 文本提取（`lopdf`）和图片 base64 编码。
- **写入 / 编辑**：通过临时文件 + 重命名实现原子写入，基于 mtime 的冲突检测，保留行尾符，单次编辑歧义检测，以及可配置的文件大小限制。
- **差异对比**：使用 `similar` 库实现基于 LCS 的统一 diff 生成。

### 对话编排

`run_turn_with_blocks` 通过四个独立方法编排每一轮对话：

1. **`stream_with_recovery`** — 发送 API 请求，在瞬态故障时自动重试/恢复。
2. **`check_permissions`** — 对每个待执行的工具逐一进行权限检查和观察者前置钩子。
3. **`execute_tools`** — 执行已批准的工具（通过 `ToolExecutor` 的 `execute_batch` 安全并发，否则串行执行）。
4. **`apply_post_hooks`** — 运行观察者后置钩子并构建会话结果消息。

## 说明

本 crate 是 Codineer 项目的内部组件，作为 `codineer-cli` 的依赖发布到 crates.io，不用于独立使用。在 Codineer 工作区之外不保证 API 稳定性。

## 许可证

MIT — 详见 [LICENSE](https://github.com/andeya/codineer/blob/main/LICENSE)。
