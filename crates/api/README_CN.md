# codineer-api

[Codineer](https://github.com/andeya/codineer) 的 AI Provider API 客户端与流式传输。

[English](README.md)

---

本 crate 负责与 AI 模型 Provider（Anthropic、OpenAI 兼容、xAI/Grok）的通信，包括请求构建、SSE 流解析、认证和重试逻辑。

## 说明

本 crate 是 Codineer 项目的内部组件，作为 `codineer-cli` 的依赖发布到 crates.io，不用于独立使用。在 Codineer 工作区之外不保证 API 稳定性。

## 许可证

MIT — 详见 [LICENSE](https://github.com/andeya/codineer/blob/main/LICENSE)。
