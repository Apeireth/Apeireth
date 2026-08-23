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
        // Generated dynamically from actually-registered tools only.
        // ---------------------------------------------------------------------
        if !tools.is_empty() {
            prompt.push_str("## 🧰 当前挂载的系统工具总线 (System Tools Registry):\n");
            prompt.push_str("你拥有直接调用与操作本地系统的工具总线。以下是当前已注册并可用的工具：\n");
            for tool_name in tools {
                let desc = Self::tool_description(tool_name);
                prompt.push_str(&format!("• `{}`: {}\n", tool_name, desc));
            }
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

    /// Maps registered tool names to their human-readable descriptions for the LLM.
    /// This is the single source of truth — only tools that appear in ToolRegistry
    /// will have their descriptions injected into the system prompt.
    fn tool_description(name: &str) -> &'static str {
        match name {
            "shell" => "本地终端沙箱执行器。在 Windows Job Object 隔离沙箱中执行 PowerShell/CMD 命令、编译构建、系统排查，受严格破坏性指令拦截保护。参数: {\"command\": \"...\"}",
            "filesystem" => "文件系统管理器。读取/创建/写入/修改本地文件与配置，检索目录树结构，保障原子化操作。参数: {\"operation\": \"read|write|list\", \"path\": \"...\"}",
            "fetch" => "安全网络数据抓取器。发起 HTTP GET/POST 请求抓取公开网页、API 数据与金融行情，受 SSRF 与出站白名单保护。参数: {\"url\": \"...\", \"method\": \"GET|POST\"}",
            "desktop_action" => "Win32 桌面操作。通过 SendInput API 执行鼠标点击/移动/拖拽、键盘输入/快捷键、滚轮操作，受坐标边界与速率限流保护。参数: {\"action\": \"click|move|drag|type|hotkey|scroll\", ...}",
            "screen_observe" => "屏幕视觉感知与 UI 逆解工具。抓取 Windows 物理屏幕像素与分辨率，计算感知哈希，识别窗口与按钮/输入框/菜单等子控件并生成 Set-of-Marks (SoM) 结构化坐标标记。参数: {\"detect_elements\": true, \"max_elements\": 30}",
            "browser" => "网页阅读器。抓取指定网页并解析为干净文本，用于信息检索与知识提取。参数: {\"url\": \"https://...\", \"max_chars\": 8000}",
            "search" => "本地代码与文件检索器。递归搜索代码库文件名与文本内容。参数: {\"query\": \"keyword\", \"path\": \".\", \"max_results\": 20}",
            "repo" => "Git 仓库协同检查工具。安全查询 Git 状态、提交记录、分支与差异。参数: {\"command\": \"status|log|diff|branch|summary\"}",
            "invest_analysis" => "辅助投资与金融分析工具。抓取实时市场行情，记录投资标的推演假设与 Brier 预测胜率，计算风险敞口与止损仓位。参数: {\"action\": \"quote|hypothesis|risk_plan\", ...}",
            "learning_digest" => "辅助学习与知识萃取工具。提炼文档/论文/代码的核心知识点、概念三元组与自测闪卡，准备存入 ACT-R 长期记忆。参数: {\"action\": \"digest|generate_flashcards|explain_concept\", ...}",
            _ => "已注册的系统工具",

        }
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

        let prompt = assembler.assemble_system_prompt(&state, &["shell", "filesystem", "fetch", "desktop_action"]);
        assert!(prompt.contains("自行升级自己的能力"));
        assert!(prompt.contains("记忆反思总结的能力"));
        assert!(prompt.contains("`shell`: 本地终端沙箱执行器"));
        assert!(prompt.contains("`desktop_action`: Win32 桌面操作"));
        assert!(prompt.contains("严禁套话"));
        assert!(prompt.contains("愉悦度(Pleasure)=0.80"));
    }
}

