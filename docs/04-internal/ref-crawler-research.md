# 爬虫工程调研 + 基础工具原则（2026-08-16）

## 一、GitHub 调研结论（先进写法）

| 项目 | 要点 |
|---|---|
| [spider-rs/spider](https://github.com/spider-rs/spider) | Rust 爬虫标杆（「for AI agents and LLMs」）：异步并发 + 网站适配器 + 重试/限速/代理/去重全套 |
| [Liohtml/RUSTScrapling](https://github.com/Liohtml/RUSTScrapling) | Scrapling 的 Rust 移植：CSS selectors + async HTTP + spider 爬取 |
| [recluse-rs/recluse](https://github.com/recluse-rs/recluse) | Rust 爬虫框架 |
| [capsolver: Rust Web Scraping Architecture](https://dev.capsolver.com/blog/web-scraping/rust-web-scraping) | 反爬对抗层：验证码/指纹/代理轮换 |
| [decodo: Rust Web Scraping](https://decodo.com/blog/rust-web-scraping) | 实操教程 |

**先进爬虫的共性（可靠性关键）**：
1. **异步并发**（tokio 并发池抓取，不是串行 BFS）
2. **重试 + 指数退避**（网络失败/429 自动重试）
3. **礼貌限速**（延迟/页，防被封）与 **robots.txt 尊重**
4. **去重**（seen 集合 / bloom filter）
5. **上限控制**（页数/深度/字节）
6. **HTML 真解析**（CSS selector 提取，不是正则/手写扫描）
7. **对抗层**（UA 轮换 / TLS 指纹 / 代理池——需 fetcher 层支持）

## 二、Apeireth Crawl 工具 v2（本次升级）

基于调研升级：
- ✅ **异步并发**（FuturesUnordered ≤4 并发）
- ✅ **重试 + 退避**（单页失败重试 2 次）
- ✅ **礼貌限速**（50ms/页 延迟）
- ✅ 去重 / 上限（已有）
- ⏳ **HTML 真解析**（当前 href 手写扫描——诚实标注；scraper crate 真解析是下一步，需加依赖）
- ⏳ **对抗层**（UA/指纹/代理——需改 ReqwestWebFetch fetcher 层，下一步）

## 三、基础工具工程原则（强制）

> **高可靠性基础工具（爬虫/网络/文件/执行等）不得独写**：必须① GitHub 调研同类成熟实现 → ② 吸收先进写法 → ③ 实战验证（真环境跑通）→ 才可提交。调研结论记入 docs/ref-*.md 备查。**写代码前先查，写完必须真跑。**

## 四、toolify AI predictions 分类思路（丰富套件/插件）

[toolify.ai/zh/category/ai-predictions](https://www.toolify.ai/zh/category/ai-predictions)（动态页，抓取受限；按搜索与常识提炼）：
- 预测类工具常见形态（多垃圾，可提炼的形态）：**预测卡片（断言+概率显示）/ 结果追踪 / 校准展示 / 多源聚合**
- 对我们 forecast 的启示：**预测展示形态**（概率可视化）、**结果追踪 UI**（resolve 后展示校准）、**多源聚合**（不同模型/方法的预测对比）
- 插件思路：预测类小插件（体育/天气/日程预测——垃圾多但形态可抄）；我们的价值在**可证伪 + 校准 + 可审计**（它们没有的）
