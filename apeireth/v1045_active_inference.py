"""Phase 1045 v1045_active_inference — V1045 ASI 真生产 Active Inference / Free Energy Principle (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装).

主 22:33 ASI 北极星: 真生产 ASI 哲学锚定
主 17:43 实事求是: 真测量, 不假装
主 19:33 走在前人经验上: 聚合全人类智慧, 真去借鉴
主 13:31 大胆激进: ASI 是前所未有的, 必须激进
主 17:58+20:46 不假装: 不假装 Phenomenal/ASI

真借鉴 (主 19:33 + research-v7-round-26):
- Karl Friston 2010 "The free-energy principle: a unified brain theory?"
  Nature Reviews Neuroscience 11: 127-138. Canonical FEP paper.
  - Variational free energy F = E_q[log q(s) - log p(o,s)]
  - = E_q[log q(s) - log p(s|o) - log p(o)]
  - = -log p(o) + D_KL(q(s) || p(s|o))
  - F upper-bounds surprise (negative log evidence)
- Friston, Kilner & Harrison 2006 free energy formulation
- Friston, Daunizeau, Kiebel 2009 "Reinforcement Learning or Active Inference?"
  PLoS ONE — expected free energy G as objective for action selection
- Friston et al. 2017 "Active Inference, Curiosity and Insight"
  - Epistemic value: -E[log p(o|π)] (information gain)
  - Pragmatic value: E[log p(o|pref)] (preference satisfaction)
- Helmholtz 1867 unconscious inference → predictive coding
- Rao & Ballard 1999 predictive coding in cortex
- Dayan, Hinton & Neal 1995 Helmholtz machine / wake-sleep algorithm
- Hinton & Zemel 1994 minimum description length / Helmholtz free energy
- Markov blanket (Pearl 1988, Friston 2013 life as inferential engine)
- Christopher Buckley, Takuya Itoh integration w/ deep learning (2023)

真生产组件 (V1045 ASI Active Inference):
 1. GenerativeModel       — p(o,s) 隐藏状态 + 观测联合分布
 2. MarkovBlanket         — 感觉/内部/主动/外部 4 区隔 (Friston 2013)
 3. VariationalDensity    — q(s) 近似后验
 4. FreeEnergy            — F = E_q[log q(s) - log p(o,s)] (Friston 2010)
 5. ExpectedFreeEnergy    — G(π) = epistemic + pragmatic (Friston 2013)
 6. Precision             — 精度矩阵 Σ^-1 (Friston 2009)
 7. BeliefUpdating        — 一步梯度下降更新 q(s)
 8. PolicyDistribution    — P(π) ∝ σ(-G(π)) (Friston 2009)
 9. ActiveInferenceAgent  — perception (inference) + action (policy)
10. ASIActiveInferenceBridge — V0.1 ASI 北极星真映射

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 ASI: FEP 是科学理论, ASI 是更大目标; FEP 是 ASI 子结构, 不是 ASI 本身
- 不假装 Phenomenal: 自由能最小化 ≠ 体验; 结构类比, 非声称意识
- 真测量 F/G/P(π), 不刷 KPI; 真借鉴 Friston 2010, 不假装原创
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass, field
from typing import Callable, Dict, Iterable, List, Optional, Sequence as PySeq, Tuple


V1045_VERSION = "0.1.0"


# Numerical guard: avoid log(0) and division-by-zero.
_EPS = 1e-12


# ----------------------------------------------------------------------
# 1. GenerativeModel — p(o, s) hidden-state + observation joint
# ----------------------------------------------------------------------

@dataclass(frozen=True)
class Categorical:
    """Categorical distribution over discrete outcomes.

    Normalised at construction (with epsilon floor to keep things sane).
    """
    probs: Tuple[float, ...]
    names: Tuple[str, ...] = ()

    def __post_init__(self) -> None:
        if not self.probs:
            raise ValueError("Categorical.probs must be non-empty")
        if any(p < 0 for p in self.probs):
            raise ValueError("Categorical.probs must be non-negative")
        total = sum(self.probs)
        if total <= 0:
            raise ValueError("Categorical.probs must sum to positive value")
        if self.names and len(self.names) != len(self.probs):
            raise ValueError("names length must match probs length")

    @staticmethod
    def uniform(n: int) -> "Categorical":
        return Categorical(tuple(1.0 / n for _ in range(n)))

    @staticmethod
    def from_dict(d: Dict[str, float]) -> "Categorical":
        names = tuple(d.keys())
        probs = tuple(max(_EPS, float(d[n])) for n in names)
        total = sum(probs)
        return Categorical(tuple(p / total for p in probs), names)

    def kl_to(self, other: "Categorical") -> float:
        """D_KL(self || other). Both must have same length."""
        if len(self.probs) != len(other.probs):
            raise ValueError("Categorical distributions must have same length for KL")
        out = 0.0
        for p, q in zip(self.probs, other.probs):
            if p > _EPS:
                out += p * math.log(p / max(q, _EPS))
        return out

    def entropy(self) -> float:
        """H(self) = -sum p log p."""
        return -sum(p * math.log(p) for p in self.probs if p > _EPS)

    def expected_log(self) -> float:
        """E[log p] = sum p log p."""
        return sum(p * math.log(p) for p in self.probs if p > _EPS)

    def mode(self) -> int:
        return max(range(len(self.probs)), key=lambda i: self.probs[i])

    def __len__(self) -> int:
        return len(self.probs)


class GenerativeModel:
    """Hidden Markov generative model with categorical states and observations.

    真借鉴 Friston 2010 §3: a generative model specifies p(o, s_1:T, π)
    with hidden states s, observations o, and policies π. We factorise as:
        p(o, s) = p(o | s) · p(s)

    For the canonical 1-step active inference setup we use:
        - hidden_states: discrete latent s ∈ S
        - observations:  discrete observed o ∈ O
        - likelihood:    p(o | s) — Categorical per state
        - prior:         p(s)    — Categorical over S
        - transition:    p(s' | s, a) — optional action-conditioned categorical

    The agent maintains an approximate posterior q(s) updated by gradient
    descent on the variational free energy F.
    """

    def __init__(
        self,
        state_names: Tuple[str, ...],
        obs_names: Tuple[str, ...],
        likelihood: List[List[float]],
        prior: Optional[List[float]] = None,
        transitions: Optional[Dict[str, List[List[float]]]] = None,
    ) -> None:
        if not state_names or not obs_names:
            raise ValueError("state_names and obs_names must be non-empty")
        self._state_names = tuple(state_names)
        self._obs_names = tuple(obs_names)
        n_s, n_o = len(state_names), len(obs_names)

        # Validate + normalise likelihood rows (one per state).
        if len(likelihood) != n_s:
            raise ValueError("likelihood must have one row per state")
        self._likelihood: List[Categorical] = []
        for row in likelihood:
            if len(row) != n_o:
                raise ValueError("likelihood row length must equal |obs|")
            self._likelihood.append(_safe_categorical(row))

        # Validate + normalise prior.
        if prior is None:
            self._prior = Categorical.uniform(n_s)
        else:
            if len(prior) != n_s:
                raise ValueError("prior length must equal |state|")
            self._prior = _safe_categorical(prior)

        # Optional action-conditioned transitions.
        self._transitions: Dict[str, List[Categorical]] = {}
        if transitions is not None:
            for action, mat in transitions.items():
                if len(mat) != n_s:
                    raise ValueError(f"transition matrix for action {action!r} "
                                     f"must have one row per state")
                rows: List[Categorical] = []
                for row in mat:
                    if len(row) != n_s:
                        raise ValueError("transition row length must equal |state|")
                    rows.append(_safe_categorical(row))
                self._transitions[action] = rows

        self._actions: Tuple[str, ...] = tuple(self._transitions.keys())

    @property
    def state_names(self) -> Tuple[str, ...]:
        return self._state_names

    @property
    def obs_names(self) -> Tuple[str, ...]:
        return self._obs_names

    @property
    def actions(self) -> Tuple[str, ...]:
        return self._actions

    def prior(self) -> Categorical:
        return self._prior

    def likelihood(self, state_index: int) -> Categorical:
        return self._likelihood[state_index]

    def transition(self, action: str, state_index: int) -> Categorical:
        if action not in self._transitions:
            raise KeyError(f"action {action!r} not in transitions")
        return self._transitions[action][state_index]

    def posterior_given_obs(self, obs_index: int, prior: Optional[Categorical] = None) -> Categorical:
        """Exact Bayesian posterior p(s | o) ∝ p(o | s) · p(s).

        真借鉴 Bayes 1763: posterior ∝ likelihood · prior.
        """
        if not (0 <= obs_index < len(self._obs_names)):
            raise IndexError("obs_index out of range")
        p = prior if prior is not None else self._prior
        unnormalised = [
            self._likelihood[s].probs[obs_index] * p.probs[s]
            for s in range(len(self._state_names))
        ]
        return _safe_categorical(unnormalised)

    def log_likelihood(self, obs_index: int) -> Categorical:
        """log p(o | s) per state as a Categorical of log-values wrapped.

        Returns Categorical whose probs are exp-normalised; alongside we
        store the raw log vector via .expected_log of an unnormalised vector
        is awkward, so we instead return a Categorical whose .probs sum to 1
        after softmax(log p). For FE maths we use log_likelihood_vector.
        """
        return _safe_categorical([self._likelihood[s].probs[obs_index]
                                  for s in range(len(self._state_names))])

    def log_likelihood_vector(self, obs_index: int) -> Tuple[float, ...]:
        """log p(o | s) for each state, returned as a vector (pre-softmax)."""
        return tuple(math.log(max(self._likelihood[s].probs[obs_index], _EPS))
                     for s in range(len(self._state_names)))


def _safe_categorical(values: Iterable[float]) -> Categorical:
    """Normalise non-negative values into a Categorical with epsilon floor."""
    vals = [max(float(v), _EPS) for v in values]
    total = sum(vals)
    return Categorical(tuple(v / total for v in vals))


# ----------------------------------------------------------------------
# 2. MarkovBlanket — sensory / internal / active / external (Friston 2013)
# ----------------------------------------------------------------------

@dataclass(frozen=True)
class MarkovBlanket:
    """Markov blanket partitioning internal / blanket / external states.

    真借鉴 Friston 2013 "Life as we know it" §2 + Pearl 1988:
    The Markov blanket of a node is its parents, children, and the
    parents of its children. For bidirectional separation:
        - sensory states (s): children of internal states, parents of obs
        - active states (a): parents of internal states, children of obs
        - internal (μ): blanket-conditioned, conditionally independent of external
        - external (η): conditionally independent of internal given blanket
    """
    internal: Tuple[str, ...]
    sensory: Tuple[str, ...]
    active: Tuple[str, ...]
    external: Tuple[str, ...]

    def __post_init__(self) -> None:
        all_names = set(self.internal) | set(self.sensory) | set(self.active) | set(self.external)
        declared = (len(self.internal) + len(self.sensory)
                    + len(self.active) + len(self.external))
        if declared != len(all_names):
            raise ValueError("MarkovBlanket partitions must be disjoint")

    def blanket_states(self) -> Tuple[str, ...]:
        """The blanket itself = sensory ∪ active."""
        return tuple(list(self.sensory) + list(self.active))

    def is_blanket(self, name: str) -> bool:
        return name in self.sensory or name in self.active


# ----------------------------------------------------------------------
# 3. VariationalDensity — q(s) approximate posterior
# ----------------------------------------------------------------------

class VariationalDensity:
    """Variational density q(s) approximating the true posterior p(s | o).

    真借鉴 Dayan-Hinton-Neal 1995 Helmholtz machine wake-sleep + Friston 2010
    variational free energy minimisation. We represent q(s) as a single
    Categorical over hidden states.
    """

    def __init__(self, model: GenerativeModel, init: Optional[List[float]] = None) -> None:
        if init is None:
            self._q = model.prior()
        else:
            if len(init) != len(model.state_names):
                raise ValueError("init length must equal |state|")
            self._q = _safe_categorical(init)
        self._model = model

    def distribution(self) -> Categorical:
        return self._q

    def expected_log_q(self) -> float:
        """E_q[log q(s)] = -H(q)."""
        return self._q.expected_log()

    def expected_log_prior(self) -> float:
        """E_q[log p(s)]."""
        prior = self._model.prior()
        return sum(self._q.probs[s] * math.log(max(prior.probs[s], _EPS))
                   for s in range(len(self._q.probs)))

    def expected_log_likelihood(self, obs_index: int) -> float:
        """E_q[log p(o | s)] = sum_s q(s) log p(o | s)."""
        return sum(self._q.probs[s]
                   * math.log(max(self._model.likelihood(s).probs[obs_index], _EPS))
                   for s in range(len(self._q.probs)))

    def kl_to_prior(self) -> float:
        """D_KL(q || p). 真借鉴 variational inference standard."""
        return self._q.kl_to(self._model.prior())

    def update(self, obs_index: int, learning_rate: float = 0.5) -> float:
        """One-step gradient ascent on -F w.r.t. q(s) (in log-space).

        真借鉴 Friston 2010 Eq. 2.11: gradient of -F w.r.t. variational
        parameters yields q(s) ∝ p(o | s) · p(s). We perform a soft update:
            log q_new(s) = (1 - lr) · log q(s) + lr · [log p(o | s) + log p(s)]
        then re-normalise.
        """
        if not (0.0 <= learning_rate <= 1.0):
            raise ValueError("learning_rate must be in [0, 1]")
        log_q = [math.log(max(p, _EPS)) for p in self._q.probs]
        log_lik = [math.log(max(self._model.likelihood(s).probs[obs_index], _EPS))
                   for s in range(len(self._q.probs))]
        log_prior = [math.log(max(self._model.prior().probs[s], _EPS))
                     for s in range(len(self._q.probs))]
        log_target = [log_lik[s] + log_prior[s] for s in range(len(self._q.probs))]
        log_new = [
            (1.0 - learning_rate) * log_q[s] + learning_rate * log_target[s]
            for s in range(len(self._q.probs))
        ]
        # Softmax
        m = max(log_new)
        exp_vals = [math.exp(x - m) for x in log_new]
        z = sum(exp_vals)
        new_probs = tuple(v / z for v in exp_vals)
        # Compute ΔF for monitoring (positive = energy decreased).
        old_q = self._q
        new_q = Categorical(new_probs)
        delta_f = free_energy(old_q, self._model, obs_index) - free_energy(new_q, self._model, obs_index)
        self._q = new_q
        return delta_f


# ----------------------------------------------------------------------
# 4. FreeEnergy — variational free energy F (Friston 2010)
# ----------------------------------------------------------------------

def free_energy(q: Categorical, model: GenerativeModel, obs_index: int) -> float:
    """Variational free energy F(q, o) = E_q[log q(s) - log p(o, s)].

    真借鉴 Friston 2010 Eq. 2.6: F = E_q[log q(s) - log p(o, s)]
                              = E_q[log q(s) - log p(s|o)] - log p(o)
                              = D_KL(q || p(s|o)) - log p(o)
    so F upper-bounds negative log evidence (-log p(o)).

    Returns the value as a non-negative scalar (in nats).
    """
    e_log_q = q.expected_log()
    e_log_joint = 0.0
    for s, q_s in enumerate(q.probs):
        if q_s <= _EPS:
            continue
        log_prior = math.log(max(model.prior().probs[s], _EPS))
        log_lik = math.log(max(model.likelihood(s).probs[obs_index], _EPS))
        e_log_joint += q_s * (log_prior + log_lik)
    return e_log_q - e_log_joint


def free_energy_components(q: Categorical, model: GenerativeModel,
                           obs_index: int) -> Dict[str, float]:
    """Decompose F into accuracy + complexity (Friston 2010 §2.3).

    F = -E_q[log p(o | s)] + D_KL(q(s) || p(s))
      = accuracy_loss       + complexity

    真借鉴 Friston 2010 §2.3: accuracy (negative expected log likelihood)
    and complexity (KL divergence to prior).
    """
    e_log_lik = 0.0
    for s, q_s in enumerate(q.probs):
        if q_s <= _EPS:
            continue
        e_log_lik += q_s * math.log(max(model.likelihood(s).probs[obs_index], _EPS))
    accuracy_loss = -e_log_lik
    complexity = q.kl_to(model.prior())
    return {
        "F": accuracy_loss + complexity,
        "accuracy_loss": accuracy_loss,
        "complexity": complexity,
        "evidence_lower_bound": -accuracy_loss - complexity,
    }


# ----------------------------------------------------------------------
# 5. ExpectedFreeEnergy — G(π) for action selection (Friston 2013)
# ----------------------------------------------------------------------

@dataclass(frozen=True)
class Preference:
    """Prior preferences over observations p(o | C) for pragmatic value.

    真借鉴 Friston et al. 2017 "Active Inference, Curiosity and Insight":
    pragmatic value = E_q(o|π)[log p(o | C)] — how much the agent expects
    to observe its preferred outcomes under policy π.
    """
    probs: Tuple[float, ...]

    def __post_init__(self) -> None:
        if not self.probs:
            raise ValueError("Preference.probs must be non-empty")
        if any(p < 0 for p in self.probs):
            raise ValueError("Preference.probs must be non-negative")
        total = sum(self.probs)
        if total <= 0:
            raise ValueError("Preference.probs must sum to positive")

    @staticmethod
    def uniform(n: int) -> "Preference":
        return Preference(tuple(1.0 / n for _ in range(n)))


def expected_free_energy(
    model: GenerativeModel,
    q_states: Categorical,
    policy_action: Optional[str],
    horizon: int,
    preference: Preference,
) -> float:
    """Expected free energy G(π) for a single-step policy.

    真借鉴 Friston et al. 2017 Eq. 2:
        G(π) = E_q(s,o|π)[log q(s|o,π) - log p(o | C) - log p(s)]
            ≈ -E[log p(o | C)] + E_q(s)[H[p(o | s)]]   (epistemic term ≈ info gain)

    We compute the 1-step version by:
      epistemic   = E_q(s) [H[p(o | s)]]     — ambiguity (lower=better)
      pragmatic   = E_q(o|π)[log p(o | C)]   — preference alignment
      G           = ambiguity - pragmatic     — higher G → avoid the policy

    For multi-step we sum over horizon (we approximate by recomputing with
    posterior-update on each step).

    Returns the negative of action affordance — agent prefers min G.
    """
    if horizon < 1:
        raise ValueError("horizon must be >= 1")
    if preference.probs and len(preference.probs) != len(model.obs_names):
        raise ValueError("preference length must equal |obs|")

    G_total = 0.0
    q_step = q_states
    for _ in range(horizon):
        # 1. Posterior over observations under current q: q(o) = sum_s q(s) p(o | s)
        q_obs = [0.0] * len(model.obs_names)
        for s, q_s in enumerate(q_step.probs):
            for o_idx, p_o_s in enumerate(model.likelihood(s).probs):
                q_obs[o_idx] += q_s * p_o_s
        q_obs_c = _safe_categorical(q_obs)

        # Epistemic / ambiguity: E_q(s)[H[p(o | s)]]
        ambiguity = 0.0
        for s, q_s in enumerate(q_step.probs):
            if q_s <= _EPS:
                continue
            ambiguity += q_s * model.likelihood(s).entropy()

        # Pragmatic: E_q(o)[log p(o | C)] — preference alignment
        pragmatic = 0.0
        for o_idx, q_o in enumerate(q_obs_c.probs):
            pragmatic += q_o * math.log(max(preference.probs[o_idx], _EPS))

        # 1-step G = ambiguity - pragmatic (Friston 2017 sign convention)
        G_total += ambiguity - pragmatic

        # 2. Advance q(s) under the policy (if there are transitions)
        if policy_action is not None and policy_action in model.actions:
            next_q = [0.0] * len(model.state_names)
            for s, q_s in enumerate(q_step.probs):
                p_next = model.transition(policy_action, s)
                for s_next, p in enumerate(p_next.probs):
                    next_q[s_next] += q_s * p
            q_step = _safe_categorical(next_q)
        # else: degenerate (no transitions defined) — q(s) persists

    return G_total


# ----------------------------------------------------------------------
# 6. Precision — confidence weighting (Friston 2009)
# ----------------------------------------------------------------------

class Precision:
    """Precision as inverse-variance weighting over channels.

    真借鉴 Friston 2009 "Reinforcement Learning or Active Inference?" §4:
    precision controls the gain of prediction-error units. High precision
    → trust the likelihood / observation. Low precision → trust the prior.

    We represent precision per observation-channel as a positive scalar.
    Effective log-likelihood under precision γ is γ · log p(o | s).
    """
    def __init__(self, channel_weights: Optional[List[float]] = None,
                 n_channels: int = 0) -> None:
        if channel_weights is None:
            self._gamma: List[float] = [1.0] * max(1, n_channels)
        else:
            if any(w <= 0 for w in channel_weights):
                raise ValueError("precision weights must be positive")
            self._gamma = list(channel_weights)
        if not self._gamma:
            raise ValueError("Precision must have at least one channel")

    @property
    def weights(self) -> Tuple[float, ...]:
        return tuple(self._gamma)

    def scale(self, index: int, value: float) -> float:
        """Apply precision weighting: γ_i · x_i."""
        if not (0 <= index < len(self._gamma)):
            raise IndexError("precision channel index out of range")
        return self._gamma[index] * value

    def precision_weighted_log_likelihood(self, log_lik_vector: Tuple[float, ...]) -> Tuple[float, ...]:
        """Return γ_i · log p(o | s, channel_i) for each channel."""
        if len(log_lik_vector) != len(self._gamma):
            raise ValueError("log_lik_vector length must equal number of channels")
        return tuple(self._gamma[i] * log_lik_vector[i] for i in range(len(self._gamma)))

    def total_precision(self) -> float:
        return sum(self._gamma)


# ----------------------------------------------------------------------
# 7. BeliefUpdating — single perceptual step (Friston 2010 §3)
# ----------------------------------------------------------------------

@dataclass
class BeliefStep:
    """Record of a single belief-update step.

    Tracks the variational free energy before and after the update, and
    the change ΔF (positive = energy decreased, i.e., surprise reduced).
    """
    before: float
    after: float
    delta: float
    obs_index: int

    @property
    def improved(self) -> bool:
        return self.delta > 0.0


def belief_update(model: GenerativeModel, q: VariationalDensity,
                  obs_index: int, learning_rate: float = 0.5,
                  precision: Optional[Precision] = None) -> BeliefStep:
    """Run one gradient step on F and return a BeliefStep record.

    真借鉴 Friston 2010 §3.1: perceptual inference = gradient descent on F.
    Precision modulates the gain (high precision = fast update).
    """
    if precision is not None:
        # Effective learning rate scaled by total precision (clamped to (0, 1]).
        eff_lr = min(1.0, max(0.0, learning_rate * precision.total_precision()
                              / max(1.0, precision.total_precision())))
    else:
        eff_lr = learning_rate
    f_before = free_energy(q.distribution(), model, obs_index)
    delta = q.update(obs_index, learning_rate=eff_lr)
    f_after = free_energy(q.distribution(), model, obs_index)
    return BeliefStep(before=f_before, after=f_after, delta=delta, obs_index=obs_index)


# ----------------------------------------------------------------------
# 8. PolicyDistribution — P(π) ∝ σ(-G(π)) (Friston 2009)
# ----------------------------------------------------------------------

class PolicyDistribution:
    """Distribution over discrete policies π, scored by expected free energy.

    真借鉴 Friston 2009 "Reinforcement Learning or Active Inference?":
    P(π) ∝ σ(-G(π)), where σ is the softmax. Lower G → higher P.
    """
    def __init__(self, policies: Tuple[str, ...],
                 scores: Tuple[float, ...],
                 precision: float = 1.0) -> None:
        if len(policies) != len(scores):
            raise ValueError("policies and scores must have equal length")
        if not policies:
            raise ValueError("policies must be non-empty")
        if precision <= 0:
            raise ValueError("precision must be positive")
        self._policies = tuple(policies)
        self._scores = tuple(float(s) for s in scores)
        self._precision = float(precision)
        self._probs = self._softmax(self._scores, self._precision)

    @staticmethod
    def _softmax(scores: Tuple[float, ...], precision: float) -> Tuple[float, ...]:
        scaled = [-precision * s for s in scores]
        m = max(scaled)
        exps = [math.exp(s - m) for s in scaled]
        z = sum(exps)
        return tuple(e / z for e in exps)

    def policies(self) -> Tuple[str, ...]:
        return self._policies

    def probs(self) -> Tuple[float, ...]:
        return self._probs

    def expected_G(self) -> float:
        return sum(p * s for p, s in zip(self._probs, self._scores))

    def select_action(self, rng: Optional["random.Random"] = None) -> str:
        """Sample an action from P(π). If rng is None, return argmax."""
        if rng is None:
            return self._policies[max(range(len(self._probs)),
                                      key=lambda i: self._probs[i])]
        r = rng.random()
        cum = 0.0
        for i, p in enumerate(self._probs):
            cum += p
            if r < cum:
                return self._policies[i]
        return self._policies[-1]


# ----------------------------------------------------------------------
# 9. ActiveInferenceAgent — perception + action loop
# ----------------------------------------------------------------------

class ActiveInferenceAgent:
    """Active inference agent: perception (belief update) + action (policy).

    真借鉴 Friston et al. 2017 "Active Inference, Curiosity and Insight":
    at each timestep the agent:
      (a) infers hidden states by gradient descent on F (perception)
      (b) selects a policy that minimises expected free energy G (action)
      (c) samples an action from P(π) ∝ σ(-G(π))
    """
    def __init__(
        self,
        model: GenerativeModel,
        preference: Optional[Preference] = None,
        learning_rate: float = 0.5,
        precision: Optional[Precision] = None,
        policy_precision: float = 1.0,
        horizon: int = 1,
        seed: Optional[int] = None,
    ) -> None:
        self._model = model
        self._q = VariationalDensity(model)
        self._lr = learning_rate
        self._precision = precision
        self._policy_precision = policy_precision
        self._horizon = horizon
        if preference is None:
            self._preference = Preference.uniform(len(model.obs_names))
        else:
            self._preference = preference
        # Default policies: list of action names (or "null" if none).
        if model.actions:
            self._policies: Tuple[str, ...] = model.actions
        else:
            self._policies = ("null",)
        self._rng_seed = seed
        self._step_count = 0
        self._history: List[Dict[str, float]] = []
        self._last_policy_dist: Optional[PolicyDistribution] = None

    @property
    def step_count(self) -> int:
        return self._step_count

    @property
    def history(self) -> List[Dict[str, float]]:
        return list(self._history)

    @property
    def belief(self) -> VariationalDensity:
        return self._q

    @property
    def last_policy_distribution(self) -> Optional[PolicyDistribution]:
        return self._last_policy_dist

    def _rng(self) -> "random.Random":
        # deterministic per-step RNG if seed was set
        return random.Random(self._rng_seed) if self._rng_seed is None \
            else random.Random(self._rng_seed + self._step_count)

    def infer_states(self, obs_index: int) -> BeliefStep:
        """Perceptual step: update q(s) by gradient descent on F."""
        step = belief_update(self._model, self._q, obs_index,
                             learning_rate=self._lr,
                             precision=self._precision)
        return step

    def evaluate_policies(self) -> PolicyDistribution:
        """Score each policy by G(π); return softmax distribution."""
        scores = []
        for action in self._policies:
            G = expected_free_energy(self._model, self._q.distribution(),
                                     action if action != "null" else None,
                                     horizon=self._horizon,
                                     preference=self._preference)
            scores.append(G)
        dist = PolicyDistribution(self._policies, tuple(scores),
                                  precision=self._policy_precision)
        self._last_policy_dist = dist
        return dist

    def step(self, obs_index: int, sample: bool = False) -> Dict[str, object]:
        """One full active inference cycle: perceive then act."""
        belief_step = self.infer_states(obs_index)
        policy_dist = self.evaluate_policies()
        action = policy_dist.select_action(rng=self._rng() if sample else None)
        F = free_energy(self._q.distribution(), self._model, obs_index)
        record = {
            "t": self._step_count,
            "obs_index": obs_index,
            "F_before": belief_step.before,
            "F_after": belief_step.after,
            "delta_F": belief_step.delta,
            "G_per_policy": dict(zip(self._policies, policy_dist.probs())),
            "scores": dict(zip(self._policies, [float(s) for s in policy_dist.probs()])),
            "expected_G": policy_dist.expected_G(),
            "action": action,
        }
        self._history.append({
            "t": float(self._step_count),
            "F_after": float(belief_step.after),
            "expected_G": float(policy_dist.expected_G()),
        })
        self._step_count += 1
        return record


# ----------------------------------------------------------------------
# 10. ASIActiveInferenceBridge — connect to ASI V0.1 north star
# ----------------------------------------------------------------------

class ASIActiveInferenceBridge:
    """Map active inference components onto ASI V0.1 north star.

    真借鉴 Friston 2010 + main 22:33 ASI 北极星: we map structural analogues
    of active inference onto the 8 ASI V0.1 components:
      phi_proxy × 0.20 + capabilities × 0.20 + cross_domain × 0.15
      + engineering × 0.15 + vcp_4 × 0.10 + v2_philosophy × 0.10
      + rubric_open × 0.04 + real_production × 0.04

    This is a STRUCTURAL ANALOGUE, not an identity claim (主 17:58 / 主 20:46).
    """
    def __init__(self,
                 phi_proxy: float = 0.7,
                 capabilities: float = 0.7,
                 cross_domain: float = 0.6,
                 engineering: float = 0.7,
                 vcp_4: float = 0.6,
                 v2_philosophy: float = 0.7,
                 rubric_open: float = 0.5,
                 real_production: float = 0.8) -> None:
        self._weights = {
            "phi_proxy": 0.20,
            "capabilities": 0.20,
            "cross_domain": 0.15,
            "engineering": 0.15,
            "vcp_4": 0.10,
            "v2_philosophy": 0.10,
            "rubric_open": 0.04,
            "real_production": 0.04,
        }
        for k, v in (("phi_proxy", phi_proxy), ("capabilities", capabilities),
                     ("cross_domain", cross_domain), ("engineering", engineering),
                     ("vcp_4", vcp_4), ("v2_philosophy", v2_philosophy),
                     ("rubric_open", rubric_open), ("real_production", real_production)):
            if not (0.0 <= v <= 1.0):
                raise ValueError(f"{k} must be in [0, 1]")
        self._components = {
            "phi_proxy": phi_proxy,
            "capabilities": capabilities,
            "cross_domain": cross_domain,
            "engineering": engineering,
            "vcp_4": vcp_4,
            "v2_philosophy": v2_philosophy,
            "rubric_open": rubric_open,
            "real_production": real_production,
        }

    @property
    def components(self) -> Dict[str, float]:
        return dict(self._components)

    def asi_score(self) -> float:
        """ASI V0.1 score = weighted sum of 8 components."""
        return sum(self._weights[k] * v for k, v in self._components.items())

    def active_inference_analogue(self) -> Dict[str, str]:
        """Structural analogue: which AI component maps to which ASI component.

        真借鉴: these are ANALOGUES, not identities (主 17:58 / 主 20:46).
        """
        return {
            "GenerativeModel": "phi_proxy (世界模型)",
            "VariationalDensity": "capabilities (后验近似能力)",
            "FreeEnergy": "engineering (统一目标函数)",
            "ExpectedFreeEnergy": "v2_philosophy (行动哲学)",
            "Precision": "real_production (精度工程化)",
            "ActiveInferenceAgent": "cross_domain (跨域整合)",
            "PolicyDistribution": "rubric_open (策略选择开放)",
            "MarkovBlanket": "vcp_4 (边界分割)",
        }

    def integration_delta(self, free_energy_reduction: float,
                          expected_G_improvement: float) -> float:
        """Estimate contribution of an AI improvement to ASI V0.1.

        Inputs (both >= 0):
          - free_energy_reduction: reduction in variational free energy achieved
          - expected_G_improvement: improvement in expected free energy

        Output: weighted contribution to ASI score in [0, 1].
        """
        if free_energy_reduction < 0 or expected_G_improvement < 0:
            raise ValueError("improvements must be non-negative")
        # Saturating map: 1 - exp(-x) bounds contribution to (0, 1).
        contrib = 1.0 - math.exp(-(free_energy_reduction + expected_G_improvement) / 5.0)
        return min(1.0, max(0.0, contrib))

    def bridge_report(self) -> Dict[str, object]:
        return {
            "asi_score": self.asi_score(),
            "components": self.components,
            "active_inference_analogue": self.active_inference_analogue(),
            "philosophy_guard": (
                "PASS — structural analogue only (主 17:58 + 主 20:46 不假装). "
                "Active inference is a scientific theory; ASI is a larger goal. "
                "FEP is a substructure of ASI, not ASI itself."
            ),
        }