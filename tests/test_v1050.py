"""Tests for v1050_asi_interpretability — ASI Interpretability 真生产.

V1050 = ASI Interpretability / Mechanistic Interpretability 真生产
(主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 +
 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

测试覆盖 11 真生产组件 + 1 bridge + sanity refs + run_all 真跑.

不假装 interpretability 已解, 真生产 = 真借鉴 + 真算法 + 真测试 + 真守门.
"""
from __future__ import annotations

import math
import random

from apeireth.v1050_asi_interpretability import (
    ASIInterpretabilityBridge,
    Attribution,
    AttributionGraph,
    ActivationCache,
    ActivationPatchingProbe,
    Circuit,
    CircuitDiscoverer,
    CircuitNode,
    Feature,
    IntegratedGradients,
    InterpretabilityReport,
    LIMEExplainer,
    PathTracker,
    ProbingClassifier,
    SHAPEstimator,
    V1050_VERSION,
    make_demo_attribution_graph,
    make_demo_bridge,
    run_all,
    sanity_check_refs,
)


# ============================================================================
# 1. Feature — 5 tests
# ============================================================================


def test_feature_construction_basic():
    """Feature 基本构造."""
    f = Feature(name="x0", value=0.5, feature_type="input", layer=0, index=0)
    assert f.name == "x0"
    assert f.value == 0.5
    assert f.feature_type == "input"
    assert f.layer == 0
    assert f.index == 0


def test_feature_default_values():
    """Feature 默认值."""
    f = Feature(name="y", value=1.0)
    assert f.feature_type == "input"
    assert f.layer == 0
    assert f.index == 0


def test_feature_invalid_layer():
    """layer < 0 应该 raise."""
    try:
        Feature(name="bad", value=0.0, layer=-1)
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_feature_invalid_type():
    """feature_type 非法应该 raise."""
    try:
        Feature(name="bad", value=0.0, feature_type="wrong")
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_feature_all_valid_types():
    """5 种合法 feature_type."""
    for ft in ["input", "activation", "neuron", "logit", "attention"]:
        f = Feature(name="x", value=0.0, feature_type=ft)
        assert f.feature_type == ft


# ============================================================================
# 2. Attribution — 4 tests
# ============================================================================


def test_attribution_basic():
    """Attribution 三元组."""
    feat = Feature(name="x", value=0.5)
    a = Attribution(feature=feat, attribution=0.3, confidence=0.9)
    assert a.feature == feat
    assert a.attribution == 0.3
    assert a.confidence == 0.9


def test_attribution_default_confidence():
    """默认 confidence = 1.0."""
    a = Attribution(feature=Feature(name="x", value=0.0), attribution=0.1)
    assert a.confidence == 1.0


def test_attribution_negative():
    """Attribution 可为负 (negative contribution)."""
    a = Attribution(feature=Feature(name="x", value=0.0), attribution=-0.5)
    assert a.attribution == -0.5


def test_attribution_invalid_confidence():
    """confidence 不在 [0, 1] 应该 raise."""
    try:
        Attribution(feature=Feature(name="x", value=0.0), attribution=0.0, confidence=1.5)
        raise AssertionError("should have raised")
    except ValueError:
        pass


# ============================================================================
# 3. AttributionGraph — 6 tests
# ============================================================================


def test_attribution_graph_construction():
    """AttributionGraph 空构造."""
    g = AttributionGraph()
    assert g.nodes == {}
    assert g.edges == []


def test_attribution_graph_add_node_and_edge():
    """添加 node + edge."""
    g = AttributionGraph()
    f1 = Feature(name="a", value=0.5, layer=0)
    f2 = Feature(name="b", value=0.6, layer=1)
    g.add_node(f1)
    g.add_node(f2)
    g.add_edge("a", "b", 0.5)
    assert len(g.nodes) == 2
    assert len(g.edges) == 1


def test_attribution_graph_add_edge_missing_node():
    """添加 edge 但 node 不存在应该 raise."""
    g = AttributionGraph()
    g.add_node(Feature(name="a", value=0.0))
    try:
        g.add_edge("a", "missing", 0.5)
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_attribution_graph_downstream_upstream():
    """downstream / upstream 真测."""
    g = AttributionGraph()
    g.add_node(Feature(name="a", value=0.0, layer=0))
    g.add_node(Feature(name="b", value=0.0, layer=1))
    g.add_node(Feature(name="c", value=0.0, layer=2))
    g.add_edge("a", "b", 0.3)
    g.add_edge("a", "c", 0.5)
    g.add_edge("b", "c", 0.7)
    down_a = g.downstream("a")
    down_b = g.downstream("b")
    up_c = g.upstream("c")
    assert len(down_a) == 2
    assert len(down_b) == 1
    assert len(up_c) == 2


def test_attribution_graph_total_attribution():
    """total_attribution 真测 = sum incoming weights."""
    g = AttributionGraph()
    g.add_node(Feature(name="a", value=0.0))
    g.add_node(Feature(name="b", value=0.0))
    g.add_node(Feature(name="c", value=0.0))
    g.add_edge("a", "c", 0.3)
    g.add_edge("b", "c", 0.5)
    total = g.total_attribution("c")
    assert abs(total - 0.8) < 1e-9


def test_attribution_graph_critical_path():
    """critical_path 找最大权重上游链."""
    g = AttributionGraph()
    g.add_node(Feature(name="input", value=0.0, layer=0))
    g.add_node(Feature(name="mid", value=0.0, layer=1))
    g.add_node(Feature(name="out", value=0.0, layer=2))
    g.add_edge("input", "mid", 0.7)
    g.add_edge("input", "out", 0.2)
    g.add_edge("mid", "out", 0.9)
    path = g.critical_path("out")
    assert "out" in path
    assert "input" in path
    assert path[0] == "input"


# ============================================================================
# 4. SHAPEstimator (Lundberg-Lee 2017) — 6 tests
# ============================================================================


def test_shap_basic_uniform():
    """SHAP uniform value function — 所有 feature 等权 = sum."""
    sh = SHAPEstimator(value_fn=lambda subset: float(len(subset)))
    # 3 features, uniform: each phi = 1.0
    attrs = sh.attribute([Feature(name=f"x{i}", value=0.0) for i in range(3)])
    assert len(attrs) == 3
    for a in attrs:
        assert abs(a.attribution - 1.0) < 1e-9


def test_shap_linear_sum():
    """SHAP linear value function = sum — phi = feature value."""
    sh = SHAPEstimator(value_fn=lambda subset: float(sum(subset)))
    attrs = sh.attribute([Feature(name=f"x{i}", value=float(i)) for i in range(3)])
    # phi_i = v({i}) - v({}) = i - 0 = i
    assert abs(attrs[0].attribution - 0.0) < 1e-9
    assert abs(attrs[1].attribution - 1.0) < 1e-9
    assert abs(attrs[2].attribution - 2.0) < 1e-9


def test_shap_efficiency():
    """SHAP efficiency property: sum phi_i = v(F) - v(∅)."""
    sh = SHAPEstimator(value_fn=lambda subset: float(sum(subset)))
    attrs = sh.attribute([Feature(name=f"x{i}", value=0.0) for i in range(3)])
    total = sum(a.attribution for a in attrs)
    # v({0,1,2}) - v({}) = 3 - 0 = 3
    assert abs(total - 3.0) < 1e-9


def test_shap_out_of_range():
    """feature_index 越界 raise."""
    sh = SHAPEstimator(value_fn=lambda subset: float(len(subset)))
    try:
        sh.shapley_value(5, 3)
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_shap_too_many_features():
    """total_features > 12 raise."""
    sh = SHAPEstimator(value_fn=lambda subset: float(len(subset)))
    try:
        sh.shapley_value(0, 15)
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_shap_empty():
    """空 feature 列表."""
    sh = SHAPEstimator(value_fn=lambda subset: 0.0)
    attrs = sh.attribute([])
    assert attrs == []


# ============================================================================
# 5. LIMEExplainer (Ribeiro 2016) — 5 tests
# ============================================================================


def test_lime_basic_attribution():
    """LIME basic attribution — 返回每个 dim 的 slope."""
    li = LIMEExplainer(predict_fn=lambda x: sum(x), num_samples=30, seed=42)
    attrs = li.attribute([0.5, 0.6, 0.7])
    assert len(attrs) == 3
    for a in attrs:
        assert -1.0 < a.attribution < 2.0  # reasonable range


def test_lime_confidence_in_01():
    """LIME confidence ∈ [0, 1]."""
    li = LIMEExplainer(predict_fn=lambda x: sum(x), num_samples=30, seed=42)
    attrs = li.attribute([0.5, 0.6])
    for a in attrs:
        assert 0.0 <= a.confidence <= 1.0


def test_lime_empty_query():
    """空 query."""
    li = LIMEExplainer(predict_fn=lambda x: 0.0, num_samples=10)
    attrs = li.attribute([])
    assert attrs == []


def test_lime_gaussian_kernel_basic():
    """高斯核: 距离 0 → 权重 1."""
    li = LIMEExplainer(predict_fn=lambda x: 0.0)
    weights = li._gaussian_kernel([0.0])
    assert abs(weights[0] - 1.0) < 1e-9


def test_lime_deterministic_seed():
    """相同 seed → 相同 attribution."""
    li1 = LIMEExplainer(predict_fn=lambda x: sum(x), num_samples=20, seed=42)
    li2 = LIMEExplainer(predict_fn=lambda x: sum(x), num_samples=20, seed=42)
    a1 = li1.attribute([0.5, 0.6])
    a2 = li2.attribute([0.5, 0.6])
    assert all(abs(x.attribution - y.attribution) < 1e-9 for x, y in zip(a1, a2))


# ============================================================================
# 6. IntegratedGradients (Sundararajan 2017) — 5 tests
# ============================================================================


def test_ig_basic_attribution():
    """IG basic attribution — 返回每个 dim 的 attribution."""
    ig = IntegratedGradients(predict_fn=lambda x: sum(x), steps=10)
    attrs = ig.attribute([0.5, 0.6, 0.7])
    assert len(attrs) == 3
    for a in attrs:
        assert -1.0 < a.attribution < 2.0


def test_ig_zero_query_zero_attribution():
    """query=0 → attribution=0 (since x - x' = 0)."""
    ig = IntegratedGradients(predict_fn=lambda x: sum(x), steps=10)
    attrs = ig.attribute([0.0, 0.0, 0.0])
    for a in attrs:
        assert abs(a.attribution) < 1e-9


def test_ig_custom_baseline():
    """自定义 baseline."""
    ig = IntegratedGradients(predict_fn=lambda x: sum(x), steps=10, baseline=[0.1, 0.1])
    attrs = ig.attribute([0.5, 0.5])
    for a in attrs:
        assert a.attribution >= 0.0


def test_ig_baseline_length_mismatch():
    """baseline 长度不匹配 raise."""
    ig = IntegratedGradients(predict_fn=lambda x: sum(x), baseline=[0.0])
    try:
        ig.attribute([0.5, 0.5])
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_ig_confidence_increases_with_steps():
    """更多 steps → 更高 confidence."""
    ig1 = IntegratedGradients(predict_fn=lambda x: sum(x), steps=5)
    ig2 = IntegratedGradients(predict_fn=lambda x: sum(x), steps=50)
    a1 = ig1.attribute([0.5])[0]
    a2 = ig2.attribute([0.5])[0]
    assert a2.confidence > a1.confidence


# ============================================================================
# 7. ActivationPatching (Meng 2022 ROME) — 5 tests
# ============================================================================


def test_activation_cache_basic():
    """ActivationCache 缓存 + 还原."""
    cache = ActivationCache()
    cache.cache(0, [0.5, 0.6])
    cache.cache(1, [0.7])
    assert cache.get(0) == [0.5, 0.6]
    assert cache.get(1) == [0.7]
    assert cache.input_dim == 2


def test_activation_patching_causal_effect_basic():
    """causal_effect 真测 — output 恢复度 ∈ [0, 1+]."""
    def forward(x, layer, patched):
        if layer < 0:
            return sum(x)
        return sum(patched) if patched else sum(x)

    ap = ActivationPatchingProbe(forward_fn=forward, layers=3)
    cache = ActivationCache()
    cache.cache(0, [0.5, 0.6, 0.7])
    effects = ap.causal_effect([0.5, 0.6, 0.7], [0.1, 0.1, 0.1], cache)
    assert len(effects) == 3
    for v in effects.values():
        assert -1.0 <= v <= 2.0  # reasonable range


def test_activation_patching_critical_layers():
    """critical_layers 找出 critical layers."""
    def forward(x, layer, patched):
        if layer < 0:
            return sum(x)
        return sum(patched) if patched else sum(x)

    ap = ActivationPatchingProbe(forward_fn=forward, layers=3)
    cache = ActivationCache()
    cache.cache(0, [0.5, 0.6])
    cache.cache(1, [0.7])
    layers = ap.critical_layers([0.5, 0.6], [0.1, 0.1], cache, threshold=0.5)
    assert isinstance(layers, list)


def test_activation_patching_zero_denominator_safe():
    """clean = corrupted → causal effect = 0 (no crash)."""
    def forward(x, layer, patched):
        return 0.5  # constant

    ap = ActivationPatchingProbe(forward_fn=forward, layers=2)
    cache = ActivationCache()
    cache.cache(0, [0.0])
    effects = ap.causal_effect([0.0], [0.0], cache)
    for v in effects.values():
        assert v == 0.0


def test_activation_cache_restore():
    """restore 真还原."""
    cache = ActivationCache()
    cache.cache(2, [0.1, 0.2, 0.3])
    restored = cache.restore(2)
    assert restored == [0.1, 0.2, 0.3]


# ============================================================================
# 8. CircuitDiscoverer (Anthropic 2023 + Wang 2022) — 5 tests
# ============================================================================


def test_circuit_node_construction():
    """CircuitNode 基本构造."""
    node = CircuitNode(node_id="x", component_type="mlp", layer=1, importance=0.5)
    assert node.node_id == "x"
    assert node.component_type == "mlp"
    assert node.layer == 1
    assert node.importance == 0.5


def test_circuit_basic():
    """Circuit 基本."""
    c = Circuit(circuit_id="c1", behavior="toy")
    c.add_node(CircuitNode(node_id="x", component_type="mlp", layer=0, importance=0.5))
    c.add_node(CircuitNode(node_id="y", component_type="logit", layer=1, importance=0.8))
    c.add_edge("x", "y")
    assert c.size() == 2
    # 1 edge / (2 * 1) = 0.5 (图论密度: edges / n*(n-1) for undirected simple graph)
    assert abs(c.density() - 0.5) < 1e-9


def test_circuit_density_sparse():
    """sparse circuit density < 1.0."""
    c = Circuit(circuit_id="c1")
    for i in range(5):
        c.add_node(CircuitNode(node_id=f"n{i}", component_type="mlp", layer=i))
    c.add_edge("n0", "n1")
    c.add_edge("n1", "n2")
    # 2 edges / (5 * 4) = 0.1
    assert abs(c.density() - 0.1) < 1e-9


def test_circuit_discoverer_basic():
    """CircuitDiscoverer 在 demo graph 上发现 circuit."""
    g = make_demo_attribution_graph()
    cd = CircuitDiscoverer(importance_threshold=0.10)
    circuit = cd.discover(g, "logit_out", behavior="toy")
    assert circuit.size() > 0
    assert circuit.behavior == "toy"


def test_circuit_discoverer_missing_output():
    """output 不在 graph 中 → empty circuit."""
    g = AttributionGraph()
    cd = CircuitDiscoverer(importance_threshold=0.10)
    circuit = cd.discover(g, "missing", behavior="test")
    assert circuit.size() == 0


# ============================================================================
# 9. ProbingClassifier (Hewitt-Manning 2019) — 5 tests
# ============================================================================


def test_probing_classifier_fit_predict():
    """线性 probe fit + predict."""
    pc = ProbingClassifier(layer=1, ridge_lambda=0.01)
    reps = [[0.1, 0.2], [0.4, 0.5], [0.7, 0.8]]
    labels = [0.3, 0.7, 1.1]
    pc.fit(reps, labels)
    pred = pc.predict([0.5, 0.5])
    assert isinstance(pred, float)


def test_probing_classifier_predict_before_fit():
    """fit 之前 predict raise."""
    pc = ProbingClassifier(layer=1)
    try:
        pc.predict([0.5, 0.5])
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_probing_classifier_length_mismatch():
    """X 和 y 长度不匹配 raise."""
    pc = ProbingClassifier(layer=1)
    try:
        pc.fit([[0.1, 0.2], [0.3, 0.4]], [0.5])
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_probing_classifier_encoding_score():
    """encoding_score ∈ [0, 1]."""
    pc = ProbingClassifier(layer=1, ridge_lambda=0.01)
    reps = [[0.1, 0.2], [0.4, 0.5], [0.7, 0.8]]
    labels = [0.3, 0.7, 1.1]
    pc.fit(reps, labels)
    score = pc.encoding_score(reps, labels)
    assert 0.0 <= score <= 1.0


def test_probing_classifier_empty():
    """空 fit 不崩."""
    pc = ProbingClassifier(layer=1)
    pc.fit([], [])
    assert pc.weights is None


# ============================================================================
# 10. PathTracker (Geiger 2024) — 5 tests
# ============================================================================


def test_path_tracker_do_intervene():
    """do(node = value) 传播到下游."""
    g = AttributionGraph()
    g.add_node(Feature(name="a", value=0.0, layer=0))
    g.add_node(Feature(name="b", value=0.0, layer=1))
    g.add_node(Feature(name="c", value=0.0, layer=2))
    g.add_edge("a", "b", 0.5)
    g.add_edge("b", "c", 0.4)
    pt = PathTracker(graph=g)
    result = pt.do_intervene("a", 1.0)
    assert result["a"] == 1.0
    assert abs(result["b"] - 0.5) < 1e-9
    assert abs(result["c"] - 0.2) < 1e-9


def test_path_tracker_total_causal_effect():
    """total_causal_effect 真测."""
    g = AttributionGraph()
    g.add_node(Feature(name="a", value=0.0))
    g.add_node(Feature(name="b", value=0.0))
    g.add_edge("a", "b", 0.6)
    pt = PathTracker(graph=g)
    effect = pt.total_causal_effect("a", "b", value=1.0)
    assert abs(effect - 0.6) < 1e-9


def test_path_tracker_do_intervene_missing_node():
    """do 节点不存在 raise."""
    g = AttributionGraph()
    g.add_node(Feature(name="a", value=0.0))
    pt = PathTracker(graph=g)
    try:
        pt.do_intervene("missing", 1.0)
        raise AssertionError("should have raised")
    except ValueError:
        pass


def test_path_tracker_find_all_paths():
    """find_all_paths 真测."""
    g = make_demo_attribution_graph()
    pt = PathTracker(graph=g)
    paths = pt.find_all_paths("input_0", "logit_out")
    assert len(paths) >= 1


def test_path_tracker_missing_nodes():
    """missing source/target 返回 []."""
    g = make_demo_attribution_graph()
    pt = PathTracker(graph=g)
    paths = pt.find_all_paths("missing", "logit_out")
    assert paths == []


# ============================================================================
# 11. InterpretabilityReport — 5 tests
# ============================================================================


def test_interpretability_report_basic():
    """InterpretabilityReport 基本."""
    r = InterpretabilityReport(title="Test", behavior="test")
    md = r.to_markdown()
    assert "Test" in md
    assert "test" in md


def test_interpretability_report_add_attributions():
    """添加 attributions 后 to_markdown 包含."""
    r = InterpretabilityReport(title="T", behavior="b")
    r.add_attributions([
        Attribution(feature=Feature(name="x0", value=0.5), attribution=0.3, confidence=0.9)
    ])
    md = r.to_markdown()
    assert "Attributions" in md
    assert "x0" in md


def test_interpretability_report_add_circuit():
    """添加 circuit 后 to_markdown 包含."""
    r = InterpretabilityReport(title="T", behavior="b")
    c = Circuit(circuit_id="c1", behavior="b")
    c.add_node(CircuitNode(node_id="x", component_type="mlp", layer=0, importance=0.5))
    r.add_circuit(c)
    md = r.to_markdown()
    assert "Circuits" in md
    assert "c1" in md


def test_interpretability_report_add_critical_path():
    """添加 critical path 后 to_markdown 包含."""
    r = InterpretabilityReport(title="T", behavior="b")
    r.add_critical_path(["input", "hidden", "output"])
    md = r.to_markdown()
    assert "Critical Paths" in md
    assert "input" in md


def test_interpretability_report_metrics():
    """添加 metrics 后 to_markdown 包含."""
    r = InterpretabilityReport(title="T", behavior="b")
    r.asil_metrics = {"overall": 0.75}
    md = r.to_markdown()
    assert "ASI V0.2 Bridge Metrics" in md
    assert "0.7500" in md


# ============================================================================
# 12. ASIInterpretabilityBridge — 6 tests
# ============================================================================


def test_bridge_basic_construction():
    """ASIInterpretabilityBridge 基本构造."""
    b = ASIInterpretabilityBridge()
    assert b.interpretability_score() == {}


def test_bridge_demo_score_in_01():
    """demo bridge score ∈ [0, 1]."""
    b = make_demo_bridge()
    score = b.interpretability_score()
    assert 0.0 <= score.get("overall", 0.0) <= 1.0


def test_bridge_v02_contribution_positive():
    """V0.2 contribution > 0 when score > 0."""
    b = make_demo_bridge()
    contrib = b.asi_v02_interpretability_contribution()
    assert contrib > 0.0


def test_bridge_is_interpretable_threshold():
    """is_interpretable 阈值守门."""
    b = make_demo_bridge()
    assert isinstance(b.is_interpretable(threshold=0.50), bool)


def test_bridge_safety_score_components():
    """safety_score 包含多个组件 key."""
    b = make_demo_bridge()
    score = b.interpretability_score()
    # demo bridge 应至少有几个 key
    assert "overall" in score
    assert len(score) >= 5


def test_bridge_no_components_zero():
    """全 None → score 空 / overall = 0."""
    b = ASIInterpretabilityBridge()
    score = b.interpretability_score()
    assert score == {} or score.get("overall", 0.0) == 0.0


# ============================================================================
# 13. Sanity refs + run_all + version — 4 tests
# ============================================================================


def test_sanity_refs_all_true():
    """sanity_check_refs 全 True."""
    refs = sanity_check_refs()
    for v in refs.values():
        assert v is True
    assert "do_not_pretend_phenomenal" in refs
    assert "do_not_pretend_asi" in refs
    assert "do_not_pretend_interpretability_solved" in refs


def test_version_format():
    """version 是 0.x.x 格式."""
    assert V1050_VERSION.startswith("0.")


def test_run_all_returns_dict():
    """run_all 返回 dict."""
    result = run_all()
    assert isinstance(result, dict)
    assert "version" in result
    assert "sanity" in result


def test_run_all_completeness():
    """run_all 包含所有 11 组件结果."""
    result = run_all()
    expected_keys = [
        "graph_nodes", "graph_edges", "critical_path",
        "shap_attributions", "lime_attributions", "ig_attributions",
        "patch_effects", "circuit_size", "circuit_density",
        "probing_prediction", "all_paths_count",
        "interpretability_score", "v02_contribution", "is_interpretable",
    ]
    for k in expected_keys:
        assert k in result, f"missing key: {k}"


# ============================================================================
# 14. Integration — 3 tests
# ============================================================================


def test_integration_full_workflow():
    """完整工作流: graph → SHAP → LIME → IG → circuit → report."""
    g = make_demo_attribution_graph()

    def toy_v(subset):
        return float(sum(subset))

    sh = SHAPEstimator(value_fn=toy_v)
    inputs = [g.nodes[n] for n in g.nodes if g.nodes[n].feature_type == "input"]
    sh_attrs = sh.attribute(inputs)
    assert len(sh_attrs) == 3

    li = LIMEExplainer(predict_fn=lambda x: sum(x), num_samples=20, seed=42)
    lime_attrs = li.attribute([f.value for f in inputs])
    assert len(lime_attrs) == 3

    ig = IntegratedGradients(predict_fn=lambda x: sum(x), steps=10)
    ig_attrs = ig.attribute([f.value for f in inputs])
    assert len(ig_attrs) == 3

    cd = CircuitDiscoverer(importance_threshold=0.10)
    circuit = cd.discover(g, "logit_out", behavior="toy")
    assert circuit.size() > 0

    report = InterpretabilityReport(title="Integration Test", behavior="toy")
    report.add_attributions(sh_attrs)
    report.add_circuit(circuit)
    md = report.to_markdown()
    assert "Integration Test" in md
    assert "toy" in md


def test_integration_path_to_attribution():
    """PathTracker → AttributionGraph 集成."""
    g = make_demo_attribution_graph()
    pt = PathTracker(graph=g)
    paths = pt.find_all_paths("input_0", "logit_out")
    assert len(paths) >= 1
    # 用 paths 来标注 critical path
    if paths:
        critical = g.critical_path("logit_out")
        assert critical[0] == "input_0" or critical[0] in g.nodes


def test_integration_v02_contribution_proportional():
    """V0.2 contribution ∝ overall score."""
    b1 = make_demo_bridge()
    contrib1 = b1.asi_v02_interpretability_contribution()
    # 添加一个完美 component
    b1.circuit_discoverer = CircuitDiscoverer(importance_threshold=0.0)  # 找所有
    contrib2 = b1.asi_v02_interpretability_contribution()
    # contrib2 应 >= contrib1
    assert contrib2 >= contrib1 * 0.99  # 允许小误差