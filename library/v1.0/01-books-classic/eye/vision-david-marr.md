---
name: book-eye-vision-david-marr
description: "Use when designing eye organ's visual processing pipeline — Vision (Marr) gives the 3-level computational theory of vision: computational / algorithmic / implementational."
---

# Vision (David Marr, 1982)

> **Organ**: eye (眼) | **R125-21 经典书 #1** | **计算机视觉圣经**

## Overview

David Marr (MIT, 1945-1980, 35 岁早逝) 把视觉理解成 3 层: (1) 计算理论 (computational theory, 什么问题), (2) 算法 (algorithmic, 怎么解), (3) 实现 (implementational, 硬件). 视觉 = 从 2D 图像重建 3D 场景. 早期视觉: 边缘 / 纹理 / 深度. 中期: 表面. 晚期: 物体识别. 这是 Apeireth eye organ "视觉处理" 维度的算法基础.

## When to Use

- 设计 eye organ 视觉处理 pipeline
- 写 agent 的"看图/视频" 解释
- 多模态输入中的"图像信号"
- 主人思考"AI 视觉 vs 人视觉"

## Key Takeaways

**1. 3 层分析框架**:
| 层 | 问题 | 例 (视觉) |
|---|---|---|
| 计算理论 | 什么计算, 为什么 | 从 2D 重建 3D |
| 算法 | 用什么算法 | 边缘检测, 立体匹配 |
| 实现 | 硬件/神经 | V1 简单细胞, V2 复杂细胞 |

**2. 早期视觉 (Early Vision)**:
- 边缘 (edges) — 强度不连续
- 纹理 (texture) — 统计模式
- 深度 (depth) — 立体 / 运动
- 表面 (surfaces) — 法向 / 反射

**3. 视觉重建 = 逆向工程**:
- 视网膜图像是 3D 场景的"投影"
- 视觉 = 从投影重建 3D (欠定问题, 必加假设)
- 假设 = 自然场景统计先验

## Apply to Apeireth

**eye organ 视觉处理**:
- 主人 1.0 release 后: TUI 9 organ 屏幕渲染 = 早期视觉的"图像生成"
- 借鉴 Marr 3 层: 计算理论 (8 哲学锚) + 算法 (apeireth-tui 渲染) + 实现 (ratatui)
- 多模态输入图像: eye 接收, brain 理解, hand 输出反应

**Marr 跟现代 CNN 关系**:
- Marr 1982 时代 CNN 未成熟, 假设错误 (CNN 端到端学习, 0 必 3 层分离)
- 但 Marr 框架仍是 "思考视觉" 的金标准 (per Hawkins 大脑新皮层理论)

## Iron Law

**必读第 1 章 "The Philosophy and the Approach"** (15 页) + 第 2 章 "Representations and Processes" (40 页). 后面章节可跳.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 表格 1:1
- **本书 PDF**: 公开 (1982, 多个 PDF)
- **关联 Apeireth 模块**: `apeireth-perception` + `apeireth-tui` (渲染)
- **关联 R125-15a**: arxiv 论文 30+ (CV 论文在 arxiv/)
