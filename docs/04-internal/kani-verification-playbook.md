# Kani 形式化验证实战手册（踩坑与配方）

```text
[Document-Meta]
性质:    经验沉淀手册 —— 2026-10-03~11 kani 证明门攻坚的全部坑与配方, 可复用。
来源:    4 轮 CI 盲改 + WSL 本地化后 3 轮精确打击的实测记录 (含假绿事故)。
适用:    动 research/verification/kani 或已验证面 canonical 文件前先读 §2;
         本地验证环境搭建照 §5; 新增 harness 照 §6 配方写。
维护:    新坑新配方随手补, 保持"症状 → 根因 → 药方 → 证据"四段式。
```

## 1. 背景：这场仗怎么打的

证明门（`.github/workflows/kani.yml`）在 2026-09-26「证明失败必须可见」可信化后
连红——考古发现**三个 harness 从未证明完过**（25m/25m/45m 全是超时），早年被
`continue-on-error` 静默掩盖。此后 4 轮 CI 盲改（每轮 45 分钟反馈）连败，直到
本地 WSL + Kani 落地（分钟级反馈）才逐个击穿。

**第一条总经验：验证工程必须有本地快速反馈回路。CI 每 45 分钟一轮的盲改是
最贵的调试方式。**

## 2. 坑清单（症状 → 根因 → 药方）

### K1. 静默失败掩盖「从未证完」
- 症状: 证明 job 历史全绿, 拆开看每件 harness 都是超时被杀。
- 根因: `continue-on-error: true` 把超时吞了; 25m 超时贴线运行（黄金 run
  实测 25m04s）。
- 药方: ① 证明步骤永不静默; ② 超时放宽要有实测依据（25m→45m 附证据链）;
  ③ 钉 Kani 版本（`--version 0.68.0 --locked`）防发版漂移。

### K2. canonical 文件的 `crate::` 路径在镜像里失联
- 症状: mirror（`#[path]` 零复制包含）编译炸 E0433。
- 根因: `use crate::mcp_bridge::...` 锚定宿主 crate root; mirror 嵌套一层
  （typecheck 桩 `crate::mirror::`）即失联。
- 药方: 可移植约定 = **canonical 文件不锚定 crate root**（自包含或 `super::`）;
  确需外围引用面时在 mirror 根补**编译面 shim**（注释写明"不进证明面"）,
  漂移防线交给生产侧测试锁真值。

### K3. HashMap 是 CBMC 的路径爆炸源（两层）
- 症状: harness 45m+ 跑不完; 日志 sip.rs / hashbrown raw.rs 展开到 unwind 上界。
- 根因: ① SipHash `Hasher::write` 在符号长度下无限展开; ② **换存根哈希器后
  hashbrown 桶探测循环照样展开到上界**（对新鲜分配内存无法收敛, raw.rs
  max-iter=31 实测）——容器机器本体就是证明面噪音。
- 药方: `cfg(kani)` 下容器整个换成**线性表**（`kani_collections::HashMap` drop-in,
  insert/get/get_mut/remove/contains_key/keys/values/retain/entry 三件套/
  Index/IndexMut/IntoIterator×3/set 语义 PartialEq/Serde 全套）; canonical
  文件统一 `#[cfg(kani)] use super::kani_collections::HashMap;`（真 crate
  编译期剔除, 生产类型逐字节不变）。

### K4. 字符串生成器的"符号性结构"比值域大小更致命（三易其稿）
- 症状: `bounded_string` 换了三版才收住。
- 演化: ① `String::from_utf8_lossy(任意字节)` → Utf8Chunks 验证机器爆 52+ 层;
  ② Latin-1 码点直构 → **编码 1-2 字节变长 = 字节长度符号化** → memchr/搜索
  全线按 unwind 上界展开（8757 次 × 63 层实测）; ③ **ASCII（0..=127）→
  字节长度恒 = N 具体值** → 根串搜索精确收敛。
- 药方: 生成器要让**长度具体、内容可符号**; 优先字面量混符号尾字节;
  值域收窄如实改 claim（"收窄面 = 不覆盖多字节编码分支, 由单测互补"）。

### K5. 符号长度收集 = OOM 的结构根源（分支定长法）
- 症状: 数值系 harness（river/semantic_axis/residual_pyramid/dual_scaled）
  15GB 内存 + 求解阶段永不落地; SSA 75k VCC 直接 OOM。
- 根因: `(0..n).map().collect()` 的 n 是符号值（`any % 4`）→ Vec 长度符号 →
  slice 循环全部按 unwind 上界展开（river 32 层、semantic_axis 96 层）。
- 药方: **分支定长**——
  ```rust
  let v: Vec<f32> = match kani::any::<u8>() % 4 {
      0 => vec![],
      1 => vec![bounded_f32()],
      2 => vec![bounded_f32(), bounded_f32()],
      _ => vec![bounded_f32(), bounded_f32(), bounded_f32()],
  };
  ```
  每分支定长 → 循环精确展开 3-4 层, unwind 可从 32/96 收到 8。

### K6. 符号 f32 全域（含 NaN 位型）软浮点位爆炸
- 症状: `kani::any::<f32>()` 全域输入令每条浮点指令位爆炸; 另有
  `--nan-check` 把中间 NaN 算术判负（哪怕终值被 `max(0.01)` 硬救回来）。
- 药方: 有限域量词（整数映射 `bounded_f32()`, |x|<20）+ **具名 NaN/±∞/边界
  案例**（fold sim 五态口径: "任意有限 f32 + 具名 NaN/±∞/0.0"）; 生产侧对
  垃圾输入**源头过滤**（`is_finite()`）而非终值硬救。

### K7. libm 外调不支持 + 恒等桩的解析闭合
- 症状: `VERIFICATION:- FAILED`, SUMMARY: `call to foreign "C" function cbrtf
  is not currently supported by Kani` (kani#2423)。
- 药方: `cbrt_compat` 双实现——生产走真 cbrt, `cfg(kani)` 恒等桩; **注释里
  写闭合论证**（cbrt 在 [0,1] 保界 → Ω 域断言解析闭合; 状态一致性两边同值
  蕴含传递）。桩的正当性 = 被桩函数的数学性质, 不能拍脑袋。

### K8. CBMC 资源学（OOM 与超时是两种病）
- 实测: symex 470s / 279 万步表达式 / 75k VCC → SSA 转换 15GB OOM;
  加内存到 25GB 后过 SSA 但求解 20m 未落地。
- 要点: ① GitHub runner 只有 7GB —— 数值 harness 在 CI 上可能是 OOM 而非
  慢; ② 并行 job 天然各占 runner（内存隔离）, 串行单 job 会连坐;
  ③ WSL2 默认只拿宿主 50%（`.wslconfig` memory= 可调）; ④ 两件 CBMC 并行
  = 双倍内存, 本地验证逐件跑。

### K9. ⚠️ 假绿事故: 管道截断杀验证进程（踩了三次）
- 症状: `cargo kani ... | grep ... | head -4` 报 exit 0、几秒"跑完"——**假的**。
- 根因: `head`/`Select-Object -First` 提前关管道 → SIGPIPE 杀掉 cargo kani/
  cbmc, 退出码是 head 的。三次事故: 幽灵 10s 成绩单、setup 被误判、
  判定行被噪音挤掉。
- 药方: 验证输出**落文件再 grep**（`> /tmp/x.log 2>&1` 然后单独 grep）;
  永远以 `^VERIFICATION` 判定行为准, 不以退出码/耗时下结论。

### K10. 证明门 CI 拓扑（并行化 + 路径豁免的契约坑）
- required check 名是**契约**: 拆矩阵 job 时新 job 名会顶掉 required 名 →
  所有 PR 永远等不到勾。药方: 矩阵 job 各自 `kani · xxx`（非必需）+ **同名
  汇总门 job**（`needs` 聚合, 名字 = required 原名）。
- 路径豁免: 触发放宽到全量 PR, `changes` job 判定是否触碰已验证面; 未触碰
  → 汇总门秒绿; 触碰 → 全矩阵必须绿。纪律不松, 无关 PR 免等。

## 3. 配方速查（新增 harness 检查单）

- [ ] 输入全部显式有界; **长度具体**（分支定长, 不用符号长度收集）;
- [ ] f32 用 `bounded_f32()` + 具名 NaN/±∞/边界案例;
- [ ] 字符串用 ASCII 生成器（字节长度 = N 具体值）, 优先字面量结构;
- [ ] 避开 HashMap/HashSet（`kani_collections::HashMap`）;
- [ ] 避开 libm 外调（cbrt/powf/ln→exp 族）; 必须用则恒等桩 + 闭合论证;
- [ ] 生产侧垃圾输入源头过滤, 不靠终值硬救;
- [ ] unwind 从实测收紧（循环界具体后 8 通常够）, 注释写明收紧依据;
- [ ] claim 文案与证明域**逐字对齐**（收窄面如实写, 单测互补要注明）;
- [ ] 输出落文件再 grep, 以 `^VERIFICATION` 为准。

## 4. 调试手法（CI 日志考古 + 本地解剖）

- CI 侧（匿名/API 均可）: check-runs → `details_url` 拿 job id →
  `actions/jobs/{id}/logs` 拉 zip → 统计 `Unwinding loop ... file X` 分布
  （按库分组 + max iteration）定位爆炸源; `SUMMARY` 段的 `Failed Checks`
  是判定真相; `check-runs` 的 steps 数组看死在哪一步（注意区分
  timeout/失败/cancelled——runner 基建取消是第三种）。
- 本地侧: WSL + Kani（见 §5）逐件跑, 分钟级; 同一件的 churn 采样
  （`grep 'Unwinding loop' | 统计`）直接指认热点函数。
- 时间学: `Verification Time` 是纯求解; wall 还含编译（首跑 1-2 分钟）。

## 5. 本地验证环境（WSL2 + Kani 0.68.0）

```powershell
# 一次性 (Windows): WSL MSI 直装 + rootfs import (细节见 ops-network-playbook.md)
wsl --import Ubuntu-24.04 C:\WSL\Ubuntu-24.04 <ubuntu-noble-wsl.rootfs.tar.gz> --version 2
# .wslconfig: memory=26GB swap=8GB (CBMC 吃内存; 默认 50% 不够)
```

```bash
# WSL 内: 工具链 (网络走宿主机代理 http://<网关IP>:7897)
apt-get install -y build-essential curl pkg-config libssl-dev
curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
cargo install kani-verifier --version 0.68.0 --locked
cargo kani setup --local-bundle <tar.gz>   # ★ 绕过 GitHub 下载 (网络差时必用)
# 源码进 WSL 原生盘 (9p 跨盘编译慢十倍):
tar -C /mnt/c/Users/31683/Apeireth-rust --exclude=./target --exclude=./frontend -cf - . | tar -C /root/apeireth -xf -
cd /root/apeireth/research/verification/kani
cargo kani --harness <name> > /tmp/x.log 2>&1; grep -E "^VERIFICATION" /tmp/x.log
```

要点: `cargo kani setup` 无视已放好的 tar 包坚持下载; 正道是
`--local-bundle <path>`（kani-verifier setup.rs 的 use_local_bundle 分支）。

## 6. 本地已验证面 harness 基线（2026-10-11 实测）

| harness | 本地 | 备注 |
|---|---|---|
| SM ×3 | 3-15s | 线性表后秒级 |
| fold_block | 待复测 | 结构化生成器后 churn 大减; TwoWaySearcher 残留 |
| river | 复测中 | 分支定长 + cbrt 桩 + NaN 过滤 |
| residual/semantic/dual/async | 待推广分支定长 | semantic_axis unwind 96 是最大嫌疑 |
