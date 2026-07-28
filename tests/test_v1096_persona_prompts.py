"""Tests for R8 v1096 persona prompt contracts."""
import pytest

from apeireth.v1096_persona_prompts import (
    ANTI_CONFORMITY_ARBITRATION_PROMPT,
    PERSONA_NAMES,
    PERSONA_PROMPTS,
    SWITCH_PROMPT_TEMPLATE,
    V1096_VERSION,
    get_persona_prompt,
    render_anti_conformity,
    render_switch_prompt,
)


def test_version():
    assert V1096_VERSION == "0.1.0"


def test_exactly_four_personas():
    assert PERSONA_NAMES == ("调度者", "学习者", "思考者", "助手")
    assert set(PERSONA_PROMPTS) == set(PERSONA_NAMES)


@pytest.mark.parametrize("name", PERSONA_NAMES)
def test_prompt_is_chinese_and_bounded(name):
    prompt = get_persona_prompt(name)
    assert len(prompt) <= 500
    assert prompt is PERSONA_PROMPTS[name]
    assert "性格基线" in prompt
    assert "典型使用场景" in prompt
    assert "与其他 persona 边界" in prompt


@pytest.mark.parametrize("name", PERSONA_NAMES)
def test_prompt_denies_consciousness_claims(name):
    prompt = get_persona_prompt(name)
    assert "没有意识" in prompt
    assert "不要声称有意识" in prompt


@pytest.mark.parametrize("name", PERSONA_NAMES)
def test_prompt_preserves_v1072_identity(name):
    prompt = get_persona_prompt(name)
    assert "v1072" in prompt
    assert "identity_id" in prompt
    assert "不得覆盖" in prompt


def test_switch_template_has_required_slots():
    for slot in ("from_persona", "to_persona", "task", "evidence"):
        assert "{" + slot + "}" in SWITCH_PROMPT_TEMPLATE


def test_switch_render_contains_context():
    rendered = render_switch_prompt("学习者", "思考者", "评估方案", "证据 E1")
    assert "学习者 → 思考者" in rendered
    assert "评估方案" in rendered
    assert "证据 E1" in rendered
    assert "v1072 identity_id" in rendered


def test_switch_empty_evidence_marks_unknown():
    rendered = render_switch_prompt("助手", "调度者", "安排任务")
    assert "无（请标记 unknown）" in rendered


def test_switch_rejects_unknown_source():
    with pytest.raises(ValueError):
        render_switch_prompt("陌生者", "助手", "任务")


def test_switch_rejects_unknown_target():
    with pytest.raises(ValueError):
        render_switch_prompt("助手", "陌生者", "任务")


def test_switch_rejects_empty_task():
    with pytest.raises(ValueError):
        render_switch_prompt("助手", "调度者", " ")


def test_anti_conformity_template_has_slots():
    for slot in ("issue", "opinions", "evidence"):
        assert "{" + slot + "}" in ANTI_CONFORMITY_ARBITRATION_PROMPT


def test_anti_conformity_render_triggers_dissent():
    rendered = render_anti_conformity("是否发布", "四个 persona 都同意", "E1")
    assert "conformity" in rendered
    assert "E1" in rendered
    assert "dissent" in rendered


def test_anti_conformity_requires_structured_output():
    rendered = render_anti_conformity("风险", "观点 A", "")
    assert '"consensus_risk"' in rendered
    assert '"dissent"' in rendered
    assert '"unknowns"' in rendered
    assert '"needs_confirmation"' in rendered


def test_anti_conformity_empty_issue_rejected():
    with pytest.raises(ValueError):
        render_anti_conformity("", "观点")


def test_anti_conformity_empty_opinions_rejected():
    with pytest.raises(ValueError):
        render_anti_conformity("事项", "")


def test_unknown_persona_rejected():
    with pytest.raises(ValueError):
        get_persona_prompt("裁判者")


def test_prompts_are_distinct():
    assert len(set(PERSONA_PROMPTS.values())) == 4


def test_common_boundary_blocks_execution_claim():
    for prompt in PERSONA_PROMPTS.values():
        assert "只给建议" in prompt or "建议" in prompt
        assert "不越权执行" in prompt or "不负责" in prompt
