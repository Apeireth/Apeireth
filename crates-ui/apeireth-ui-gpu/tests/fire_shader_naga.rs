//! 生产 `fire.ts` 的 **4 条 GLSL** 能否被 **naga** 解析 —— **仓内固化**的回归测试。
//!
//! ## 为什么要有这个测试
//!
//! 场景层迁移的**最大未知**是「生产侧着色器能不能被引擎的 naga 吃下」。
//! 2026-10-24 实测结论：**4/4 条全部通过**（见 ADR-0015 §2.6.5）⇒
//! 但**当时**那个探针**只存在于 `/tmp`** ⇒ **证据不可复核、也不进 CI**。
//!
//! ⇒ 本测试把那条结论**固化成仓内门禁** ⇒ `cargo test -p apeireth-ui-gpu` 会跑它
//! ⇒ CI 的 `cargo test (crates-ui)` job ⇒ **runner 上也验证**。
//!
//! ## 口径（与 P5-b 生产对照**同一口径**：`glsl::parse`）
//!
//! 1. **只读**引入生产源（`frontend/companion-desktop/src/lib/intro/fire.ts`），
//!    **一个字都不改**生产文件；
//! 2. 生产着色器是**模板字符串** ⇒ 有 `${…}` 插值 ⇒ 按**生产同一口径**替换
//!    （值取自 `scene/tokens.ts`）—— ���条**声明外壳**变换；
//! 3. GLSL ES 3.00 的 varying location 由**声明顺序**自动分配；而 naga 的 GLSL 前端
//!    （**无 ES profile**）把未声明 location 的 varying 全当 location 0
//!    ⇒ `BindingCollision` ⇒ 按声明顺序补 `0,1,2,…` —— **第二条声明外壳**变换；
//! 4. ⇒ **着色器主体一个字不动** ⇒ 测的**就是生产那份着色器**。
//!
//! ## 本测试会红的三种情况（都是**该红**的）
//!
//! | 情况 | 判据 | 含义 |
//! |---|---|---|
//! | 生产新增了 `${…}` 插值 | 替换后仍含 `${` | 替换表要更新（**不要**猜值） |
//! | 生产改了 varying 声明 | 补 location 后仍解析失败 | 声明外壳的变换规则要更新 |
//! | 生产着色器本身超出 naga 能力 | `glsl::parse` 失败 | **上游不支持** ⇒ 需要决策 |

use apeireth_ui_gpu::glsl;

/// ⭐ 生产 `fire.ts` 的**源文本**（**编译期**嵌入 ⇒ 零运行时 IO、零新依赖）。
///
/// ## 为什么用 `include_str!` 而不是运行时读文件
///
/// - 仓库的 `clippy.toml` **禁用** `std::fs::read_to_string` / `std::fs::File::open`
///   （要求 `fs_err`）⇒ 运行时读文件要给本 crate **加依赖**；
/// - `include_str!` 是**编译期**读取 ⇒ **零依赖**；
/// - 且它在**编译期**就 fail-fast：生产源不在仓内 ⇒ **编译不过**
///   （而不是运行到一半才失败）⇒ 这正是我们要的。
///
/// ## 路径层数
///
/// 本文件在 `crates-ui/apeireth-ui-gpu/tests/` ⇒ 上跳 **3 级** = 仓库根
/// （`..` = `apeireth-ui-gpu/` ⇒ `../..` = `crates-ui/` ⇒ `../../..` = 仓库根）。
/// ⚠️ **层数必须对着「本文件所在目录」数**（本项目已因此踩过两次：
/// ADR-0015 与 `ui-engine-ci.yml` 的 `production-compare` job）。
const PRODUCTION_FIRE_TS: &str =
    include_str!("../../../frontend/companion-desktop/src/lib/intro/fire.ts");

/// 从 `fire.ts` 里按「反引号模板」取出一条程序。
fn extract(source: &str, name: &str) -> Option<String> {
    let head = format!("const {name} = `");
    let start = source.find(&head)? + head.len();
    let rest = &source[start..];
    let end = rest.find("`;")?;
    Some(rest[..end].to_string())
}

/// **声明外壳变换 ①**：把生产模板字符串里的 `${…}` 插值按**生产同一口径**替换。
///
/// ⚠️ 这 5 处插值实测全是**设计令牌的 `RgbTriple.join(',')`**
/// ⇒ 值取自 `scene/tokens.ts`（生产侧只读）。
/// ⇒ **生产若新增插值** ⇒ 本表失效 ⇒ 下面的「无残留插值」断言会**明确报错**。
fn interpolate(source: &str) -> String {
    let subs = [
        ("${COLOR_VOID.baseRgb.join(',')}", "0.027,0.027,0.047"),
        ("${COLOR_STARFIELD.tintWarmRgb.join(',')}", "0.95,0.85,0.62"),
        ("${COLOR_STARFIELD.tintBoneRgb.join(',')}", "0.98,0.93,0.8"),
        ("${COLOR_STARFIELD.tintBlueRgb.join(',')}", "0.72,0.8,1.0"),
        ("${COLOR_STARFIELD.milkyWayRgb.join(',')}", "0.9,0.8,0.58"),
    ];
    let mut out = source.to_owned();
    for (from, to) in subs {
        out = out.replace(from, to);
    }
    out
}

/// **声明外壳变换 ②**：给没有显式 location 的 `in`/`out` 声明补上顺序 location。
///
/// GLSL ES 3.00 的 varying location 由**声明顺序**自动分配；naga 的 GLSL 前端
/// （无 ES profile）把未声明 location 的 varying 全当 location 0 ⇒ `BindingCollision`。
/// ⇒ 按声明顺序补 `0,1,2,…` ⇒ **与 ES 的自动分配同义** ⇒ 着色器主体一个字不动。
fn add_varying_locations(source: &str) -> String {
    let mut next = 0usize;
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let is_varying = (trimmed.starts_with("in ") || trimmed.starts_with("out "))
            && !trimmed.contains("layout(");
        if is_varying {
            let indent = &line[..line.len() - trimmed.len()];
            out.push_str(&format!("{indent}layout(location={next}) {trimmed}\n"));
            next += 1;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// ⭐ **主判据**：生产的 **4 条** GLSL 全部能被 naga 解析**并回吐** GLSL ES 300。
#[test]
fn all_fire_glsl_sources_compile_with_naga() {
    let source = PRODUCTION_FIRE_TS;

    let cases = [
        ("BG_VERT", glsl::ShaderStage::Vertex),
        ("BG_FRAG", glsl::ShaderStage::Fragment),
        ("FIRE_VERT", glsl::ShaderStage::Vertex),
        ("FIRE_FRAG", glsl::ShaderStage::Fragment),
    ];

    for (name, stage) in cases {
        let raw = extract(source, name).unwrap_or_else(|| {
            panic!("从生产 fire.ts 里取不到 `{name}` ⇒ 生产改��了着色器的命名 ⇒ 请更新本测试")
        });

        // 声明外壳变换 ① ⇒ 插值替换
        let interp = interpolate(&raw);
        assert!(
            !interp.contains("${"),
            "`{name}` 里仍有未替换的 `${{…}}` 插值\n\
             ⇒ **生产新增了插值** ⇒ 插值表要更新（取值请查 `scene/tokens.ts`，**不要**猜）\n\
             ⇒ 残留片段：{:?}",
            interp
                .match_indices("${")
                .map(|(at, _)| &interp[at..(at + 40).min(interp.len())])
                .collect::<Vec<_>>()
        );

        // 声明外壳变换 ② ⇒ varying location
        let located = add_varying_locations(&interp);

        let module = glsl::parse(&located, stage).unwrap_or_else(|error| {
            panic!(
                "naga 拒绝生产着色器 `{name}`（{} 行）：{error}\n\
                 ⇒ 分两类：① **声明外壳**变了（varying 声明规则要更新本测试的变换）\n\
                 ⇒        ② **着色器主体**超出 naga 30.0.1 能力 ⇒ 属**上游不支持** ⇒ 需要决策",
                raw.lines().count()
            )
        });

        assert!(
            !module.entry_points.is_empty(),
            "`{name}` 解析通过但**没有入口点** ⇒ 解析结果不可用"
        );

        // 回吐 GLSL ES 300 ⇒ 「能解析」还不够，得「能吐回 ES」。
        glsl::emit_glsl_es_300(&module, stage).unwrap_or_else(|error| {
            panic!("naga 解析了 `{name}` 却**吐不回** GLSL ES 300：{error}")
        });
    }
}
