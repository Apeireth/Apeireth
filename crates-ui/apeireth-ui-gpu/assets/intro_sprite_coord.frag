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
  // ⭐⭐ **坐标编码探针**（2026-10-27 诊断用，**不是**生产着色器）
  // ============================================================
  // 目的：把**粒子内坐标**编码进 RGB（R = x、G = y）
  // ⇒ ⇒ 于是「生产 vs 引擎」的差分**直接读出坐标差多少、朝哪个方向**
  // ⇒ ⇒ ⇒ 一次实验就能区分：上下翻转 / 偏移 / 量化。
  //
  // ⚠ 引擎的**四边形路径**会把 `gl_PointCoord` 换成 varying `_engine_point_coord`
  //   ⇒ ⇒ 这里写 `gl_PointCoord` ⇒ ⇒ 引擎侧自动变成 `_engine_point_coord` ✔
  //   ⇒ ⇒ 而生产侧（原生 WebGL2）里 `gl_PointCoord` 是**内建** ⇒ ⇒ 合法 ✔
  fragColor = vec4(gl_PointCoord.x, gl_PointCoord.y, 0.0, 1.0);
}
