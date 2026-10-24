# exo 评估

## 机制（What it does）

- 核心功能：家用 GPU 集群推理引擎，把多台家用设备的 GPU/NPU 聚合为统一推理池
- 解决什么问题：单设备显存上限 → 多设备水平扩展 → 在家跑 70B+ 模型
- 关键技术：模型分区 (tensor/pipeline parallelism) + 设备发现 + RDMA/NVLink/USB-net 自适应 + MLX 后端

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-local-inference`（本地推理套件，待评估是否有占位）
  - `apeireth-provider`（5 Provider 真合并，模型路由）
- 差异化优势：
  - exo 是「设备异构聚合」，APEIRETH provider 是「云端 vs 本地模型路由」
  - exo 强调 Apple Silicon（MLX），APEIRETH 当前 ROCm/CUDA 为主
- 可借鉴：
  - **模型分区策略**：若主人后期想跑 70B 模型在家，exo 的 pipeline parallelism 可作为参考（不直接 fork，因为 exo 是 Python）
  - **设备发现协议**：mDNS / 手动 IP 列表的轻量级发现，可借鉴到 `apeireth-local-inference` 的多机调度

## 吸收建议（Action items）

- P0 立即做：**不动**。exo 是 Python 项目，与 APEIRETH Rust 主力栈不兼容。
- P1 评估后做：仅当主人想跑 70B+ 模型在家 + 有多设备条件时，考虑：
  - 用 `candle` 或 `cudarc` 在 Rust 端重写 exo 的核心调度逻辑（参考 exo 的设备发现 + 分区策略）
  - 或 fork exo 的 MLX 后端思路到 Rust（社区已有 `candle-metal` 实验）
- P2 长期调研：列入观察项，每季度回看 exo 进展。
- 不做（重复 / 价值低）：APEIRETH 当前主力是「云端 API + 小模型本地」，家用集群推理需求未触发。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无 exo 源码；本评估基于 GitHub README 公开信息。
- 未调研不写结论：exo 的 MLX 后端实现细节 / 分区算法具体步骤 / 设备发现协议均为推理判断。如需落地建议，必须先实测 exo 源码。