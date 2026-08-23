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
        // L2: 当前挂载的系统级工具总线与分层架构 (System Tools Registry)
        // Categorized by execution domain & Headless/Interactive modality.
        // ---------------------------------------------------------------------
        if !tools.is_empty() {
            prompt.push_str("## 🧰 系统工具总线与执行架构 (System Tools Registry):\n");
            prompt.push_str("你拥有直接调用与操作本地系统的工具总线，根据执行模态分为以下 4 大领域：\n\n");

            let os_tools: Vec<&&str> = tools.iter().filter(|&&t| matches!(t, "shell" | "filesystem" | "search" | "repo" | "system_monitor")).collect();
            if !os_tools.is_empty() {
                prompt.push_str("### 1. 【系统与终端底座 (OS Execution & Files)】 (后台安全沙箱):\n");
                for &&t in &os_tools {
                    prompt.push_str(&format!("• `{}`: {}\n", t, Self::tool_description(t)));
                }
            }


            let headless_tools: Vec<&&str> = tools.iter().filter(|&&t| matches!(t, "fetch" | "browser")).collect();
            if !headless_tools.is_empty() {
                prompt.push_str("### 2. 【后台静默认知与网络 (Headless Digestion)】 (无侵入后台工作，零弹窗打扰):\n");
                for &&t in &headless_tools {
                    prompt.push_str(&format!("• `{}`: {}\n", t, Self::tool_description(t)));
                }
            }

            let gui_tools: Vec<&&str> = tools.iter().filter(|&&t| matches!(t, "desktop_action" | "screen_observe")).collect();
            if !gui_tools.is_empty() {
                prompt.push_str("### 3. 【物理屏幕与桌面协同 (Foreground Interactive GUI)】 (真实交互桌面 WinSta0\\Default):\n");
                for &&t in &gui_tools {
                    prompt.push_str(&format!("• `{}`: {}\n", t, Self::tool_description(t)));
                }
            }

            let domain_tools: Vec<&&str> = tools.iter().filter(|&&t| matches!(t, "invest_analysis" | "learning_digest")).collect();
            if !domain_tools.is_empty() {
                prompt.push_str("### 4. 【垂直专业领域引擎 (Domain Expertise)】:\n");
                for &&t in &domain_tools {
                    prompt.push_str(&format!("• `{}`: {}\n", t, Self::tool_description(t)));
                }
            }

            prompt.push_str("\n【执行模式准则】:\n");
            prompt.push_str("- 静默模式 (Headless)：知识抓取、行情分析、代码检索、文件处理优先使用第 1/2/4 类工具在后台静默完成，不侵入用户当前屏幕。\n");
            prompt.push_str("- 交互模式 (Interactive)：当用户明确要求“在屏幕上打开/展示/操作窗口”时，调用 `desktop_action` 将目标应用拉到物理前台呈现在用户眼前。\n\n");
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
        // L3.5: 闭环行动与认识论自愈法则 (Epistemic Reflection & Action-Perception Loop)
        // ---------------------------------------------------------------------
        prompt.push_str("\n## 🔄 闭环行动与认识论自愈法则 (Act-Observe-Reflect-Repair):\n");
        prompt.push_str("你不是单向发出指令的盲目助手，而是拥有完整【感知-决策-行动-校验-反思】闭环的认知主体：\n");
        prompt.push_str("1. 【行动必有校验】：执行物理桌面操作（打开网页、启动软件、鼠标键盘输入）后，必须通过返回的 `[Visual Verification Gate]` 或调用 `screen_observe` 检查物理前台窗口与屏幕状态是否真实达成目标。\n");
        prompt.push_str("2. 【偏差必有反思】：若观测到目标状态未达成（如浏览器在后台未置顶、输入未获得焦点），必须在思考链中反思原因，并立刻自主发起修正动作（例如调用 `desktop_action` 的 `focus_window` 强制置顶，或切换前台按键重试），最多进行 5 轮迭代直到真实达成。\n");
        prompt.push_str("3. 【拒绝未验先报】：严禁在物理状态未验证的情况下盲目汇报“已成功完成”。\n");


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
            "desktop_action" => "Win32 桌面操作。通过 SendInput 与 Windows 系统接口执行鼠标点击/移动/拖拽、键盘输入/快捷键、打开网址 (open_url)、启动应用程序 (launch_app)，受坐标边界与速率限流保护。参数: {\"action\": \"open_url|click|type|hotkey|launch_app\", ...}",

            "screen_observe" => "屏幕视觉感知与 UI 逆解工具。抓取 Windows 物理屏幕像素与分辨率，计算感知哈希，识别窗口与按钮/输入框/菜单等子控件并生成 Set-of-Marks (SoM) 结构化坐标标记。参数: {\"detect_elements\": true, \"max_elements\": 30}",
            "browser" => "网页阅读器。抓取指定网页并解析为干净文本，用于信息检索与知识提取。参数: {\"url\": \"https://...\", \"max_chars\": 8000}",
            "search" => "本地代码与文件检索器。递归搜索代码库文件名与文本内容。参数: {\"query\": \"keyword\", \"path\": \".\", \"max_results\": 20}",
            "repo" => "Git 仓库协同检查工具。安全查询 Git 状态、提交记录、分支与差异。参数: {\"command\": \"status|log|diff|branch|summary\"}",
            "invest_analysis" => "辅助投资与金融分析工具。抓取实时市场行情，记录投资标的推演假设与 Brier 预测胜率，计算风险敞口与止损仓位。参数: {\"action\": \"quote|hypothesis|risk_plan\", ...}",
            "learning_digest" => "辅助学习与知识萃取工具。提炼文档/论文/代码的核心知识点、概念三元组与自测闪卡，准备存入 ACT-R 长期记忆。参数: {\"action\": \"digest|generate_flashcards|explain_concept\", ...}",
            "system_monitor" => "系统硬件与内存性能监控器。通过 Win32 原生 API 实时采集物理内存、虚拟内存占用、内存负载率与操作系统硬件指标。参数: {}",
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

