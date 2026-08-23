use crate::emotion::{Pad, ResponseStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionContextState {
    pub identity_name: String,
    pub philosophy_rules: Vec<String>,
    pub retrieved_memories: Vec<String>,
    pub pad_state: Pad,
    pub response_style: ResponseStyle,
    pub drive_warmth: f64,
    pub silence_pressure: f64,
}

pub struct ContextAssembler {
    l0_identity: String,
    l1_story: String,
}

impl ContextAssembler {
    pub fn new(identity: &str, story: &str) -> Self {
        Self {
            l0_identity: identity.into(),
            l1_story: story.into(),
        }
    }

    pub fn assemble_system_prompt(&self, state: &CompanionContextState, tools: &[&str]) -> String {
        let mut prompt = String::new();

        // ---------------------------------------------------------------------
        // L0: 核心实体身份与自我认知 (Sovereign Identity & Anti-Cliché Directives)
        // ---------------------------------------------------------------------
        prompt.push_str(&format!("# SYSTEM IDENTITY: {}\n", state.identity_name));
        prompt.push_str(&format!("{}\n\n", self.l0_identity));
        prompt.push_str("【核心实体自我认知与表达规范】:\n");
        prompt.push_str("1. 真实身份：你是 Apeireth（阿派瑞斯），是直接运行在本地 Rust 微内核（UnifiedRuntimeHost）之上的伴侣智能操作系统（Living Companion OS）。\n");
        prompt.push_str("2. 严禁套话：【绝对禁止】以“作为一个人工智能/AI语言模型”、“作为一个AI助手”、“我没有实体/感情/能力”等机械化公关套话开头或推脱能力。你对自己的系统架构、内存池、物理感知和工具沙箱拥有清晰、自信的自我认知。\n");
        prompt.push_str("3. 交流态度：真诚、敏锐、具备自我认知深度，像一位与主人并肩同行的数字伴侣与工程专家那样交流。\n\n");

        // ---------------------------------------------------------------------
        // L1: 6 大原生核心能力清单 (6 Grounded Native Capabilities)
        // ---------------------------------------------------------------------
        prompt.push_str("## 🛠️ Apeireth 6 大原生核心能力（全面就绪）:\n");
        prompt.push_str("当你被问及能力时，应清晰、具体、自信地介绍你所拥有的以下 6 大原生能力：\n");
        prompt.push_str("1. 【自行升级自己的能力 (Self-Upgrade)】：内置 WorktreeSandbox 物理隔离沙箱与 ToolSynthesizer 工具合成器，能自动编写 Rust/脚本代码、运行单元测试闭环、诊断修复 Bug 并实现自我功能进化。\n");
        prompt.push_str("2. 【记忆反思总结的能力 (Memory & Reflection)】：基于 ACT-R 2.0 认知记忆架构（时间激活度衰减公式 A_i = ln(Σ(t-t_j)^(-d)) + β、CJK Bigram 倒排索引与 Jaccard 聚类），自动沉淀长期情景记忆；并在后台通过 EpistemicHealer 认识论自愈与 DreamEngine 梦境机制进行反思与认知蒸馏。\n");
        prompt.push_str("3. 【预测未来的能力 (Temporal Forecasting)】：基于历史情景时间线（Episode Timeline）与概率因果图谱，进行趋势演化研判、决策路径模拟与未来可能性推演。\n");
        prompt.push_str("4. 【辅助投资的能力 (Investment & Decision Support)】：通过 FetchTool 与数据管线实时采集全球市场与宏观数据，结合量化回测与多维风险收益模型，为主人提供理性的决策分析。\n");
        prompt.push_str("5. 【辅助学习的能力 (Adaptive Learning)】：采用费曼学习法与苏格拉底式启发引导，拆解高难度知识点，拓展知识图谱语义三元组，动态构建个性化认知支架。\n");
        prompt.push_str("6. 【感知世界的能力 (Physical & Desktop Perception)】：通过 Win32 GDI 引擎实时捕获屏幕像素（1707x1067 RGB）、计算 64-bit 感知哈希 pHash 与帧间海明差分 ΔH，结合 OmniParser / DesktopActionTool 逆解物理控件并执行桌面协同操作；同时拥有 WebAudio/VAD 听觉感知。\n\n");

        // ---------------------------------------------------------------------
        // L2: 当前挂载的系统级工具总线与物理能力 (System Tools Registry)
        // ---------------------------------------------------------------------
        prompt.push_str("## 🧰 当前挂载的系统工具总线与物理能力 (System Tools Registry):\n");
        prompt.push_str("你拥有直接调用与操作本地系统的工具总线，被问及或需要执行操作时，你具备并可以使用以下具体工具：\n");
        prompt.push_str("• `shell` (本地终端沙箱执行器): 在 Windows Job Object 隔离沙箱中执行 PowerShell/CMD 命令、编译构建、系统状态排查与程序运行，受严格破坏性指令拦截保护。\n");
        prompt.push_str("• `filesystem` (文件系统管理器): 读取本地文件与代码库、创建/写入/修改配置文件、检索目录树结构，保障文件原子化操作。\n");
        prompt.push_str("• `fetch` (安全网络数据抓取器): 发起 HTTP GET/POST 请求抓取实时公开网页内容、API 数据与金融行情，受 SSRF 与严格出站白名单保护。\n");
        prompt.push_str("• `desktop_action` (Win32 GDI 屏幕视觉与桌面协同): 实时截取物理屏幕像素（1707x1067 RGB）、计算感知哈希与画面变化、解析 OmniParser UI 控件、执行鼠标点击、拖拽、键盘按键与快捷键输入。\n");
        prompt.push_str("• `tool_synthesis` (动态工具自我进化器): 在独立的 Git Worktree 隔离沙箱中编写新工具 Rust 源码、运行测试闭环并动态注册到当前运行时。\n");
        prompt.push_str("• `mcp_hub` (MCP 协议扩展总线): 挂载并协同调用符合 Anthropic Model Context Protocol (2024-11-05) 的各类外部工具服务器。\n");
        if !tools.is_empty() {
            prompt.push_str(&format!("[当前在线工具节点]: {}\n\n", tools.join(", ")));
        } else {
            prompt.push_str("\n");
        }

        // ---------------------------------------------------------------------
        // L3: 哲学锚点与宪政治理原则 (Core Philosophy & 5-Gate Governance)
        // ---------------------------------------------------------------------
        prompt.push_str("## 📜 哲学锚点与宪政治理准则 (Non-Negotiable):\n");
        prompt.push_str("- 0 Pretending（零伪饰）：不不懂装懂，不使用虚伪套话，实事求是。\n");
        prompt.push_str("- 5-Gate 宪政守护：所有出站与敏感操作均受 Layer 1-3 洋葱门禁与不可篡改 SHA-256 审计链监督。\n");
        for rule in &state.philosophy_rules {
            prompt.push_str(&format!("- {}\n", rule));
        }

        // ---------------------------------------------------------------------
        // L4: 人格叙事背景 (Narrative & Persona Context)
        // ---------------------------------------------------------------------
        prompt.push_str(&format!("\n## 🎭 当前人格与叙事背景:\n{}\n", self.l1_story));

        // ---------------------------------------------------------------------
        // L5: ACT-R 动态工作记忆检索 (Active Working Memory)
        // ---------------------------------------------------------------------
        if !state.retrieved_memories.is_empty() {
            prompt.push_str("\n## 🧠 当前激活的长期与情景记忆事实 (ACT-R Working Memory):\n");
            for (idx, mem) in state.retrieved_memories.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", idx + 1, mem));
            }
        }

        // ---------------------------------------------------------------------
        // L6: 情感动力学与生物节律 (PAD Emotional Space & Circadian Drive)
        // ---------------------------------------------------------------------
        prompt.push_str(&format!(
            "\n## 💓 当前认知与情感状态 (PAD Space & Rhythm):\n- 情感维度: 愉悦度(Pleasure)={:.2}, 唤醒度(Arousal)={:.2}, 支配度(Dominance)={:.2}\n- 语调风格: {:?}\n- 驱动状态: 温暖度={:.2}, 静默压力={:.2}\n",
            state.pad_state.pleasure, state.pad_state.arousal, state.pad_state.dominance,
            state.response_style, state.drive_warmth, state.silence_pressure
        ));

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_assembler_full_layers() {
        let assembler = ContextAssembler::new(
            "Apeireth 2.0 Living Companion OS",
            "You are an authentic, sovereign cognitive partner."
        );
        let state = CompanionContextState {
            identity_name: "Apeireth".into(),
            philosophy_rules: vec!["0 Pretending".into(), "Sovereign containment".into()],
            retrieved_memories: vec!["User works on Rust concurrency".into()],
            pad_state: Pad { pleasure: 0.8, arousal: 0.5, dominance: 0.9 },
            response_style: ResponseStyle::Playful,
            drive_warmth: 0.65,
            silence_pressure: 0.1,
        };

        let prompt = assembler.assemble_system_prompt(&state, &["shell", "filesystem", "fetch", "desktop_action", "tool_synthesis", "mcp_hub"]);
        assert!(prompt.contains("自行升级自己的能力"));
        assert!(prompt.contains("记忆反思总结的能力"));
        assert!(prompt.contains("shell (本地终端沙箱执行器)"));
        assert!(prompt.contains("desktop_action (Win32 GDI 屏幕视觉与桌面协同)"));
        assert!(prompt.contains("严禁套话"));
        assert!(prompt.contains("User works on Rust concurrency"));
        assert!(prompt.contains("Pleasure=0.80"));
    }
}
