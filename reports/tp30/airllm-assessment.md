# airllm 评估

## 机制（What it does）

- 核心功能：家用单 GPU 跑 70B 模型（70B model on a single consumer GPU via layer swapping）
- 解决什么问题：70B 模型需要多卡 → 单卡家用设备跑不动 → airllm 通过层交换（layer swapping）让单卡跑 70B
- 关键技术：
  - 层交换（layer-by-layer inference，前向/后向逐层 swap 到 GPU/CPU）
  - 模型并行层调度
  - 内存映射 + 懒加载
  - HuggingFace transformers 集成

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-local-inference`（本地推理，待评估是否有占位）
  - `apeireth-provider`（5 Provider 真合并）
- 差异化优势：
  - airllm 解决「单卡跑大模型」，APEIRETH 当前是「云端 + 小模型本地」策略
  - airllm Python，APEIRETH Rust
- 可借鉴：
  - **层交换思想**：在 Rust 端用 `candle` 或 `tch-rs` 重写核心调度（参考 airllm 的 layer swapping 算法）
  - **内存映射**：参考 airllm 的 mmap 模型加载，加速冷启动

## 吸收建议（Action items）

- P0 立即做：**不动**。当前主人场景未触发 70B 本地推理需求。
- P1 评估后做：仅当主人设备升级（如新 4090/5090）+ 想跑 70B 本地时，再评估 candle-rs 生态成熟度决定是否借鉴 airllm 算法。
- P2 长期调研：列入观察项，每季度回看 candle-rs 进展。
- 不做（重复 / 价值低）：与 exo 同类（家用推理），exo 更适合多设备场景，airllm 更适合单卡极限场景。当前 APEIRETH 不主推家用推理。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无 airllm 源码；本评估基于 GitHub README 公开信息。
- 未调研不写结论：airllm 的层交换算法具体实现 / 性能数据 / 与 candle 兼容性均为推理判断。如需落地建议，必须先实测 airllm 源码 + 在 Rust 端做 POC。