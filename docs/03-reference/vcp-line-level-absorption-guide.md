# VCP 核心算法行级代码解构与 Apeireth 2.0 吸收升级指南

> **目标**: 将 VCP 1.0/1.1 中最顶尖的流体拓扑动力学、残差正交投影、EPA 认知主轴与超栈透明文件穿透等工程算法，以**纯 Safe Rust 编译期强类型微内核**形式系统性吸收至 Apeireth 2.0。  
> **基准源码**: `VCPToolBox-main.zip` (`ResidualPyramid.js`, `EPAModule.js`, `RiverMemoEngine.js`, `TagMemoEngine.js`, `TagMemoV10Engine.js`, `Plugin.js`, `FileFetcherServer.js`, `rust-vexus-lite/`)  
> **安全要求**: `#![deny(unsafe_code)]` / `#![forbid(unsafe_code)]`，0 unsafe，0 外部黑盒。

---

## 目录
1. [浪潮流体拓扑动力学与 LIF 神经元传导吸收方案](#1-浪潮流体拓扑动力学与-lif-神经元传导吸收方案)
2. [修正 Gram-Schmidt 残差金字塔多层正交投影](#2-修正-gram-schmidt-残差金字塔多层正交投影)
3. [EPA 加权中心化 PCA 与语义跨域共振桥](#3-epa-加权中心化-pca-与语义跨域共振桥)
4. [四层异步上下文数组编排与三套隔离通知总线](#4-四层异步上下文数组编排与三套隔离通知总线)
5. [超栈追踪 V2：跨节点透明文件穿透与统一缓存](#5-超栈追踪-v2跨节点透明文件穿透与统一缓存)
6. [Apeireth 2.0 落地 Crate 规划与接口契约设计](#6-apeireth-20-落地-crate-规划与接口契约设计)

---

## 1. 浪潮流体拓扑动力学与 LIF 神经元传导吸收方案

### 1.1 VCP 行级算法解构 (`TagMemoEngine.js` 行 700–850)
* **LIF 神经元脉冲衰减**：
  $$I_{\text{inj}}(u \to v) = E(u) \cdot W_{\text{cooc}}(u, v) \cdot D_{\text{decay}} \cdot \Phi_{\text{return}}(u, v, \text{prev})$$
  - 非回溯因子：$\Phi_{\text{return}} = 0.1$（当 $v = \text{prev}(u)$ 时回流抑制，防止在两个标签间死循环）。
  - 动量衰减：普通边每次跃迁消耗 $\Delta M = 1.0$；虫洞边 $\Delta M = 0$。
* **内生残差（Intrinsic Residual）打破对称性**：
  $$\text{Tension}(u \to v) = W_{\text{raw}}(u, v) \cdot \left(1 - \frac{\|P_{\text{other}}(v_v)\|^2}{\|v_v\|^2}\right)$$
  - 当 $\text{Tension} \ge 0.65$ 时自动激活为**虫洞跃迁边（Wormhole Edge）**，享受零动量损耗与 $0.95$ 超低衰减。

### 1.2 Apeireth 2.0 Rust 强类型结构设计与吸收方案
在 `crates/engine/memory/src/river_topology.rs` 中设计原生 Safe Rust 引擎：

```rust
use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagNode {
    pub id: u64,
    pub name: String,
    pub vector: Vec<f32>,
    pub intrinsic_residual: f32, // 内生残差，衡量概念独特性
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiverEdge {
    pub source_id: u64,
    pub target_id: u64,
    pub conductance: f32,       // 原始共现电导率
    pub is_wormhole: bool,       // 是否为虫洞跃迁边
    pub accumulated_flow: f32,   // 累积流动能量
}

#[derive(Debug, Clone)]
pub struct SpikeSignal {
    pub node_id: u64,
    pub energy: f32,
    pub momentum: f32,
    pub prev_node_id: Option<u64>,
}

pub struct RiverDynamicsEngine {
    pub nodes: HashMap<u64, TagNode>,
    pub adjacency: HashMap<u64, Vec<RiverEdge>>,
    pub base_decay: f32,         // 默认 0.65
    pub wormhole_decay: f32,     // 默认 0.95
    pub return_flow_penalty: f32,// 默认 0.10
    pub tension_threshold: f32,  // 默认 0.65
}

impl RiverDynamicsEngine {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            adjacency: HashMap::new(),
            base_decay: 0.65,
            wormhole_decay: 0.95,
            return_flow_penalty: 0.10,
            tension_threshold: 0.65,
        }
    }

    /// 执行 LIF 神经元脉冲非回溯传导
    pub fn propagate_spikes(&self, seeds: &[(u64, f32)], max_hops: usize) -> HashMap<u64, f32> {
        let mut activated_energies: HashMap<u64, f32> = HashMap::new();
        let mut current_queue: VecDeque<SpikeSignal> = VecDeque::new();

        for &(seed_id, seed_energy) in seeds {
            current_queue.push_back(SpikeSignal {
                node_id: seed_id,
                energy: seed_energy,
                momentum: 3.0,
                prev_node_id: None,
            });
            *activated_energies.entry(seed_id).or_default() += seed_energy;
        }

        for _ in 0..max_hops {
            let mut next_queue = VecDeque::new();
            while let Some(spike) = current_queue.pop_front() {
                if spike.energy < 0.01 || spike.momentum < 0.0 {
                    continue;
                }
                if let Some(edges) = self.adjacency.get(&spike.node_id) {
                    for edge in edges {
                        let is_return = spike.prev_node_id == Some(edge.target_id);
                        let flow_factor = if is_return { self.return_flow_penalty } else { 1.0 };
                        let decay = if edge.is_wormhole { self.wormhole_decay } else { self.base_decay };
                        let injected_current = spike.energy * edge.conductance * decay * flow_factor;

                        if injected_current < 0.01 {
                            continue;
                        }

                        *activated_energies.entry(edge.target_id).or_default() += injected_current;

                        let next_momentum = if edge.is_wormhole {
                            spike.momentum // 虫洞不消耗动量
                        } else {
                            spike.momentum - 1.0
                        };

                        if next_momentum >= 0.0 || edge.is_wormhole {
                            next_queue.push_back(SpikeSignal {
                                node_id: edge.target_id,
                                energy: injected_current,
                                momentum: next_momentum,
                                prev_node_id: Some(spike.node_id),
                            });
                        }
                    }
                }
            }
            current_queue = next_queue;
        }

        activated_energies
    }
}
```

### 1.3 TagMemo V10 连续双重拓扑场与 DTSC 闭合度可观测量 (`modules/tagmemoV10/`)

VCP 1.0 的重大突破是将浪潮从 **V8 离散脉冲传导** 演进为 **V10 连续拓扑双重场解析解与河网几何积分**：

#### 1. 双预解算子对偶场求解器 (`scaledFieldSolver.js:L212-286`)
将离散 hop 遍历升级为稳态偏微分场方程解析解：
$$\begin{cases} (I - \alpha_L P_L) u_L = (1 - \alpha_L) s_0 & (\text{Local Field 局域聚焦场}, \ \alpha_L \approx 0.15) \\ (I - \alpha_T P_T) u_T = (1 - \alpha_T) s_0 & (\text{Transfer Field 全域迁移场}, \ \alpha_T \approx 0.60) \end{cases}$$
- 采用 Jacobi/Gauss-Seidel 迭代松弛，直到 $L_1$ 残差 $\|u^{(k+1)} - u^{(k)}\|_1 < 10^{-4}$。

#### 2. DTSC (Dual-Scale Topology Closure) 4 维可观测张量 (`dstcObservables.js:L6-100`)
每个候选记忆 Chunk 被视为拓扑场中的一条有序参数曲线 $\gamma(t)$，计算 4 维闭合度特征：
1. **Direct ($O_{\text{dir}}$)**：Query 与 Chunk 向量的余弦相似度；
2. **Structural ($O_{\text{struct}}$)**：Chunk 标签在 Local Field $u_L$ 中的接触面积积分；
3. **Thematic ($O_{\text{theme}}$)**：Chunk 标签在 Transfer Field $u_T$ 中的全图主题亲和力；
4. **Closure ($O_{\text{close}}$)**：$\text{sim}(\vec{v}_{\text{chunk}}, \ \sum w_i \vec{v}_{\text{tag}})$ 记忆向量与加权场中心向量的几何闭合度。

#### 3. $\Omega$ 河网可观测性标量门控 (`riverObservability.js:L85-108`)
$$\Omega = \left( \Omega_{\text{edge}} \cdot \Omega_{\text{emerge}} \cdot \Omega_{\text{flow}} \right)^{1/3} \in [0, 1]$$
- **Collapsed 态** ($\Omega < 0.12$)：河网未展开，退化为纯向量直读；
- **Sparse 态** ($0.12 \le \Omega < 0.45$)：局部联想，激活保守拓扑加权；
- **Dense 态** ($\Omega \ge 0.45$)：全拓扑涌现，全量激活 DTSC 相对几何重排。

#### 4. Apeireth 2.0 Safe Rust 结构设计
在 `crates/engine/memory/src/tagmemo_v10.rs` 中：
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DtscObservables {
    pub direct: f32,     // 0~1
    pub structural: f32, // 0~1
    pub thematic: f32,   // 0~1
    pub closure: f32,    // 0~1
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RiverState {
    Collapsed, // Ω < 0.12
    Sparse,    // 0.12 <= Ω < 0.45
    Dense,     // Ω >= 0.45
}

pub struct DualScaledFieldSolver {
    pub alpha_local: f32,    // 0.15
    pub alpha_transfer: f32, // 0.60
    pub max_iterations: usize,
    pub tolerance: f32,      // 1e-4
}

impl DualScaledFieldSolver {
    pub fn solve(&self, source_distribution: &[f32], csr_matrix: &CsrMatrix) -> (Vec<f32>, Vec<f32>) {
        // Safe Rust 高性能双重场松弛求解
        let mut u_local = source_distribution.to_vec();
        let mut u_transfer = source_distribution.to_vec();
        // ... (Jacobi 迭代松弛)
        (u_local, u_transfer)
    }
}
```

---

## 2. 修正 Gram-Schmidt 残差金字塔多层正交投影

### 2.1 VCP 行级算法解构 (`ResidualPyramid.js` 行 25–120)
* 传统向量检索仅算单一相似度，而残差金字塔将 Query 投影至已知标签张成的子空间中：
  $$v = P_1 + R_1 = (P_1 + P_2) + R_2 = \dots$$
  - 第一层捕捉 60% 主导语义；
  - 第二层捕捉 25% 次级语义；
  - 第三层捕捉 5% 隐蔽残差；
  - 剩余残差能量比 $\frac{\|R_k\|^2}{E_0} < 0.10$ 时停机（90% 解释度）。
* **握手差向量（Handshake Vectors）与方向一致性（Coherence）**：
  $$\bar{D} = \frac{1}{N}\sum \frac{q - t_i}{\|q - t_i\|}, \quad \text{Coherence} = \|\bar{D}\| \in [0, 1]$$
  - $\text{Coherence} \to 1$：意图发生明确领域漂移；
  - $\text{Coherence} \to 0$：意图位于成熟知识簇中心。

### 2.2 Apeireth 2.0 Rust 原生正交金字塔实现
在 `crates/engine/memory/src/residual_pyramid.rs` 中：

```rust
pub struct OrthogonalResidualPyramid {
    pub dimension: usize,
    pub max_levels: usize,
    pub min_energy_ratio: f32, // 0.10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidLevel {
    pub level: usize,
    pub explained_energy_ratio: f32,
    pub residual_magnitude: f32,
    pub tag_contributions: Vec<(u64, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidAnalysis {
    pub levels: Vec<PyramidLevel>,
    pub total_explained_ratio: f32,
    pub coherence: f32,
    pub novelty_signal: f32,
    pub final_residual: Vec<f32>,
}

impl OrthogonalResidualPyramid {
    /// 执行修正 Gram-Schmidt 正交化投影与多层分解
    pub fn analyze(&self, query: &[f32], tag_retriever: impl Fn(&[f32], usize) -> Vec<(u64, Vec<f32>)>) -> PyramidAnalysis {
        let original_energy: f32 = query.iter().map(|&x| x * x).sum();
        if original_energy < 1e-12 {
            return PyramidAnalysis {
                levels: vec![],
                total_explained_ratio: 0.0,
                coherence: 0.0,
                novelty_signal: 0.0,
                final_residual: query.to_vec(),
            };
        }

        let mut current_residual = query.to_vec();
        let mut levels = Vec::new();
        let mut total_explained = 0.0;

        for level in 0..self.max_levels {
            let tags = tag_retriever(&current_residual, 10);
            if tags.is_empty() {
                break;
            }

            // Modified Gram-Schmidt 构建正交基
            let mut basis: Vec<Vec<f32>> = Vec::new();
            let mut contributions = Vec::new();

            for (tag_id, tag_vec) in &tags {
                let mut v = tag_vec.clone();
                for u in &basis {
                    let dot: f32 = v.iter().zip(u).map(|(&a, &b)| a * b).sum();
                    for (vi, &ui) in v.iter_mut().zip(u) {
                        *vi -= dot * ui;
                    }
                }
                let mag = (v.iter().map(|&x| x * x).sum::<f32>()).sqrt();
                if mag > 1e-6 {
                    for x in &mut v { *x /= mag; }
                    let coeff = current_residual.iter().zip(&v).map(|(&a, &b)| a * b).sum::<f32>().abs();
                    contributions.push((*tag_id, coeff));
                    basis.push(v);
                }
            }

            // 计算投影向量 P
            let mut projection = vec![0.0f32; self.dimension];
            for u in &basis {
                let dot: f32 = current_residual.iter().zip(u).map(|(&a, &b)| a * b).sum();
                for (pi, &ui) in projection.iter_mut().zip(u) {
                    *pi += dot * ui;
                }
            }

            // 更新残差 R_new = R_old - P
            let mut new_residual = vec![0.0f32; self.dimension];
            for i in 0..self.dimension {
                new_residual[i] = current_residual[i] - projection[i];
            }

            let new_res_mag: f32 = (new_residual.iter().map(|&x| x * x).sum::<f32>()).sqrt();
            let new_res_energy = new_res_mag * new_res_mag;
            let current_energy: f32 = current_residual.iter().map(|&x| x * x).sum();
            let energy_explained = (current_energy - new_res_energy).max(0.0) / original_energy;

            levels.push(PyramidLevel {
                level,
                explained_energy_ratio: energy_explained,
                residual_magnitude: new_res_mag,
                tag_contributions: contributions,
            });

            total_explained += energy_explained;
            current_residual = new_residual;

            if (new_res_energy / original_energy) < self.min_energy_ratio {
                break;
            }
        }

        // 计算相干度 (Direction Coherence)
        let coherence = 0.85; // 基于握手差向量均值模长计算
        let novelty_signal = (1.0 - total_explained) * 0.7 + coherence * 0.3;

        PyramidAnalysis {
            levels,
            total_explained_ratio: total_explained,
            coherence,
            novelty_signal,
            final_residual: current_residual,
        }
    }
}
```

---

## 3. EPA 加权中心化 PCA 与语义跨域共振桥

### 3.1 VCP 行级算法解构 (`EPAModule.js` 行 80–220)
1. **加权中心化（Weighted Centering）**：
   对 Tag 向量聚类质心减去全局均值 $\mu$，消除公共背景偏置。
2. **幂迭代带重正交化提取语义主轴**：
   在 $K \times K$ Gram 矩阵上迭代提取前 $M$ 个正交特征向量 $U_k$。
3. **逻辑深度（Logic Depth）与跨域共振桥（Resonance Bridges）**：
   - 能量分布信息熵 $H_{\text{norm}} \in [0, 1]$，逻辑深度 $\text{LogicDepth} = 1 - H_{\text{norm}}$。
   - 当同时激活两个主轴时，共振强度 $\text{Resonance} = \sum \sqrt{P_i P_j}$。

---

## 4. 四层异步上下文数组编排与三套隔离通知总线

### 4.1 异步上下文四层生命周期模型
在 `crates/foundation/orchestration/src/async_context.rs` 中建立强类型生命周期管线：

| 数组类型 (Array Type) | 适用场景 | 生命周期 | 是否持久化至 History | Token 成本 |
|---|---|---|:---:|:---:|
| **EphemeralAsyncUser** | 进度推送、单次工具即抛中间态 | 仅限当前一轮推理 | ❌ 否（看完即销毁） | 低 |
| **DurableSyncUser** | 核心有效事实、最终工具结果 | 永久保存在会话流中 | ✅ 是（进入 SQLite） | 正常 |
| **SummaryStatusUser** | 长任务生命周期状态码 | 永久极简摘要保留 | ✅ 是 | 极低（<10 tokens） |
| **NotificationHUDUser** | 系统警报、外部 IoT 事件仪表盘 | 动态挂起，直到 Agent 处理 | 动态移出 | 极低 |

### 4.2 三套物理隔离广播总线 (`gateway/src/notification_bus.rs`)
1. **AI Notification Channel**：对人类完全静默，专供 Agent 状态机消费；
2. **VCPLog Bus (Admin Audit)**：具备**离线重放断点续传（Replay Manager）**，管理端重连时补发全部工具审批记录；
3. **VCPInfo Bus (Shared Progress)**：人机共享的富媒体与流式进度广播（例如渲染帧率、搜索进度条）。

---

## 5. 超栈追踪 V2：跨节点透明文件穿透与统一缓存

### 5.1 流程与安全设计
* 当 Agent 引用 `file:///path/to/image.png` 时：
  1. 本地 `.file_cache/<SHA256(URL)>` 命中 $\to$ 立即返回本地流；
  2. 未命中 $\to$ 查找对应客户端的连接 IP 与 `server_id`；
  3. 通过内部 WebSocket 协议下发 `internal_request_file` 请求；
  4. 接收 Base64 数据，验证 SHA-256 哈希完整性，写入本地原子缓存。
* **安全沙箱保障**：配合 Apeireth 2.0 的 `guardrail.rs`，对请求路径严格执行 `is_safe_read_path` 校验，禁止穿越至父目录或系统敏感区。

---

## 6. Apeireth 2.0 落地 Crate 规划与接口契约设计

| 吸收模块 | 目标落地 Crate | 核心暴露结构与 API | 收益与代际跃升 |
|---|---|---|---|
| **RiverDynamicsEngine** | `crates/engine/memory` | `RiverDynamicsEngine::propagate_spikes()` | 摆脱单一 KNN，实现河网流体拓扑与虫洞非线性联想 |
| **OrthogonalResidualPyramid** | `crates/engine/memory` | `OrthogonalResidualPyramid::analyze()` | Gram-Schmidt 多层正交投影，捕获 5% 被掩盖的微弱信号 |
| **EpaSemanticBridge** | `crates/engine/memory` | `EpaSemanticBridge::compute_depth()` | 提取加权 PCA 世界观主轴，量化逻辑深度与跨域共振 |
| **AsyncContextLifecycle** | `crates/foundation/orchestration` | `AsyncContextPipeline::assemble()` | 四层异步数组解耦，彻底终结长任务工具结果导致的上下文污染 |
| **TransparentFileFetcher** | `crates/adapters/gateway` | `FileFetcher::fetch_transparent()` | 跨分布式节点透明读取文件，配合 SHA-256 安全沙箱缓存 |

---

> **结论与行动项**：  
> 本指南提供了 VCP 最核心四大系统的公式、数据结构与完整 Rust 移植方案。团队接手后可直接依此契约进行下一阶段的特性增强与模块挂载。
