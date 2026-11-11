#version 300 es
precision highp float;
in vec3 vCol;
in float vAlpha;
in float vStreak;
in vec2 vAxis;
in float vSoft;
out vec4 fragColor;
void main(){
  // ============================================================
  // ⭐⭐ **纯色探针**（2026-10-27 诊断用，**不是**生产着色器）
  // ============================================================
  // 目的：把粒子画成**纯色** ⇒ ⇒ 画布上只剩**粒子并集**
  // ⇒ ⇒ 于是「生产 vs 引擎」的差分**只反映覆盖范围**（几何）
  // ⇒ ⇒ ⇒ 可直接判断「差异是否只出现在粒子边缘」。
  //
  // ⚠⚠️ **必须触碰全部 5 个 varying**（实测踩过）：
  // 未被使用的 varying 会被**编译器优化剔除** ⇒ ⇒ 顶点输出集合与片元输入集合
  // **不匹配** ⇒ ⇒ wgpu 报 `FRAGMENT varying vCol does not match any VERTEX varying`
  // ⇒ ⇒ 建管线失败。手法：用「**不可能成立的分支**」引用它们
  // ⇒ ⇒ 编译器无法证明它恒假 ⇒ ⇒ varying 被保留；而输出值**不变**。
  float unusedCoord = gl_PointCoord.x;
  float guard = vSoft * vAxis.x;
  float guard2 = unusedCoord * vSoft;
  if (guard < -1.0 || guard2 < -1.0) {
    fragColor = vec4(0.0, 0.0, 0.0, 1.0);
  }
  fragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
