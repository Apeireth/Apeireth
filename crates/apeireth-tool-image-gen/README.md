# apeireth-tool-image-gen

**R141** — 图像生成工具

## 职责

抽象图像生成 provider: 9 商业 SDK (OpenAI DALL-E / Stability / Midjourney / Ideogram ...) + 自托管 (Stable Diffusion / ComfyUI).

## 核心能力

- 9 provider enum 抽象
- prompt-to-image 真生成
- 异步任务推送 (复用 AsyncTaskStore)
- 进度回调

## 借鉴

VCP v1.1 多 provider 抽象.

## 0 假装

✅ 29 单元测试 | ⚠️ 9 provider 仅有 enum/skeleton, R146+ 续真接
