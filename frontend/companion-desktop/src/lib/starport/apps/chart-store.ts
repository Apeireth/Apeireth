// 星仪 · 图表面纯逻辑（I 批 5，零依赖纯模块）
//
// 契约（I-starport-builtins.md「星仪」）：
//   1. 数据驱动 —— 输入数据序列，输出 SVG 几何（折线路径/柱体矩形/刻度），
//      图形从数据算出，不手画假图（0 装：非有限值/长度不齐一律抛错）；
//   2. 尺度自适应 —— 值域自动取界，min==max 退化为居中带（不除零）；
//   3. 渲染即纯函数 —— 同数据同参恒同几何（可测、可复算，性能批可基准）。
//
// 测试：tests/starport-chart.mjs（真实模块直测）。

export interface ChartSeries {
  name: string;
  points: readonly number[];
}

export interface ChartData {
  labels: readonly string[];
  series: readonly ChartSeries[];
}

export interface ChartFrame {
  width: number;
  height: number;
  padding: number;
}

export interface LineGeometry {
  paths: {name: string; d: string}[];
  ticks: {x: number; label: string}[];
  yTicks: {y: number; label: string}[];
  min: number;
  max: number;
}

export interface BarGeometry {
  bars: {name: string; index: number; x: number; y: number; width: number; height: number}[];
  min: number;
  max: number;
}

function validate(data: ChartData, frame: ChartFrame): void {
  if (!Number.isFinite(frame.width) || !Number.isFinite(frame.height) || frame.width <= 0 || frame.height <= 0) {
    throw new Error('chart frame 尺寸非法（必须正有限值）');
  }
  if (!Number.isFinite(frame.padding) || frame.padding < 0 || frame.padding * 2 >= Math.min(frame.width, frame.height)) {
    throw new Error('chart padding 非法');
  }
  if (!Array.isArray(data.series) || data.series.length === 0) {
    throw new Error('chart 数据至少一条序列');
  }
  for (const series of data.series) {
    if (!Array.isArray(series.points) || series.points.length === 0) {
      throw new Error(`序列 ${series.name} 无数据点`);
    }
    if (series.points.length !== data.labels.length) {
      throw new Error(`序列 ${series.name} 点数与标签数不齐`);
    }
    for (const value of series.points) {
      if (!Number.isFinite(value)) throw new Error(`序列 ${series.name} 含非有限值`);
    }
  }
}

function bounds(data: ChartData): {min: number; max: number} {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const series of data.series) {
    for (const value of series.points) {
      if (value < min) min = value;
      if (value > max) max = value;
    }
  }
  return {min, max};
}

/** 折线几何：像素坐标系（y 轴向下，值大者在上）。 */
export function computeLineGeometry(data: ChartData, frame: ChartFrame): LineGeometry {
  validate(data, frame);
  const {min, max} = bounds(data);
  const innerW = frame.width - frame.padding * 2;
  const innerH = frame.height - frame.padding * 2;
  const span = max - min;
  const n = data.labels.length;
  const xAt = (index: number): number =>
    frame.padding + (n === 1 ? innerW / 2 : (innerW * index) / (n - 1));
  const yAt = (value: number): number =>
    // min==max 时居中，不除零。
    frame.padding + (span === 0 ? innerH / 2 : innerH * (1 - (value - min) / span));

  const paths = data.series.map((series) => ({
    name: series.name,
    d: series.points
      .map((value, index) => `${index === 0 ? 'M' : 'L'}${xAt(index).toFixed(2)},${yAt(value).toFixed(2)}`)
      .join(' '),
  }));

  const ticks = data.labels.map((label, index) => ({x: xAt(index), label}));
  const yTicks = [
    {y: yAt(max), label: String(max)},
    {y: yAt(min), label: String(min)},
  ];
  return {paths, ticks, yTicks, min, max};
}

/** 柱状几何：每序列一组，组内均分槽位。 */
export function computeBarGeometry(data: ChartData, frame: ChartFrame): BarGeometry {
  validate(data, frame);
  const {min, max} = bounds(data);
  const innerW = frame.width - frame.padding * 2;
  const innerH = frame.height - frame.padding * 2;
  const span = max - min;
  const n = data.labels.length;
  const groups = data.series.length;
  const slot = innerW / n;
  const barW = slot / (groups + 1);

  const bars: BarGeometry['bars'] = [];
  for (let s = 0; s < groups; s += 1) {
    const series = data.series[s];
    for (let i = 0; i < n; i += 1) {
      const value = series.points[i];
      const zeroY = span === 0 ? frame.padding + innerH / 2 : frame.padding + innerH * (1 - (0 - min) / span);
      const valueY = span === 0 ? frame.padding + innerH / 2 : frame.padding + innerH * (1 - (value - min) / span);
      bars.push({
        name: series.name,
        index: i,
        x: frame.padding + slot * i + barW * (s + 0.5),
        y: Math.min(zeroY, valueY),
        width: barW,
        height: Math.abs(zeroY - valueY),
      });
    }
  }
  return {bars, min, max};
}
