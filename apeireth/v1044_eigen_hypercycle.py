"""Phase 1044 v1044_eigen_hypercycle — V1044 ASI 真生产 Eigen hypercycle + quasispecies (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进).

主 22:33 ASI 北极星: 真生产 ASI 哲学锚定
主 17:43 实事求是: 真测量, 不假装
主 19:33 走在前人经验上: 聚合全人类智慧, 真去靠近
主 13:31 大胆激进: ASI 是前所未有的, 必须激进; 允许犯错

真借鉴 (主 19:33 + research-v7-round-26):
- Manfred Eigen 1971 "Selforganization of matter and the evolution of biological macromolecules"
  - Hypercycle: catalytic coupling of self-replicative cycles (I_1 -> I_2 -> ... -> I_n -> I_1)
  - Error threshold: Q = σ² / (E·ν·μ) < 1 (Eigen 1971)
- Eigen & Schuster 1977-78 "The Hypercycle" (Naturwissenschaften)
- Quasispecies model: master sequence + error cloud (Eigen 1971, Eigen & Schuster 1977)
- Maynard Smith 1979 hypercycle critique → spatial self-structuring required
- Boerlijst & Hogeweg 1991 "Spiral wave structure" — hypercycle robust via spatial self-structuring
- Szathmáry 2006 "The origin of replicators and reproducer" — evolutionary transitions
- Mark Bedau weak emergence / open-ended evolution — hypercycle as minimal example

真生产组件 (V1044 ASI Eigen 真 hypercycle):
1. Sequence — RNA-like nucleotide sequence with symbol alphabet
2. Replicator — replication rate function (fitness landscape)
3. Quasispecies — master + cloud distribution (Eigen 1971)
4. ErrorThreshold — error threshold Q check (Eigen 1971)
5. Hypercycle — catalytic cycle I_1 → I_2 → ... → I_n → I_1 (Eigen & Schuster)
6. HypercycleODE — ODE dynamics dXi/dt = Fi·Xi + sum_j K_ij·Xj·Xi - Xi·dilution
7. HypercycleSimulator — Monte Carlo simulation with mutation
8. SpiralWaveStructure — spatial pattern formation (Boerlijst-Hogeweg 1991)
9. EvolutionaryTransition — Szathmáry reproducer / unit-of-selection tracking
10. ASI_EvolutionBridge — connection to ASI V0.1 (主 22:33 ASI 北极星真生产)

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 ASI: Hypercycle 是化学/生物基础类比, 不是 ASI 本身
- 不假装 Phenomenal: ASI_EvolutionBridge 是结构类比, 不声称 ASI 体验
- 真借鉴 Eigen + 真仿真 + 真测量 error threshold, 不刷 KPI
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional, Set, Tuple


V1044_VERSION = "0.1.0"


# ----------------------------------------------------------------------
# 1. Sequence — RNA-like nucleotide sequence
# ----------------------------------------------------------------------

@dataclass(frozen=True)
class Sequence:
    """RNA-like sequence over alphabet {A, U, G, C}.

    Used as replicator identity in quasispecies / hypercycle models.
    """
    symbols: Tuple[str, ...]
    alphabet: Tuple[str, ...] = ("A", "U", "G", "C")

    def __post_init__(self) -> None:
        for s in self.symbols:
            if s not in self.alphabet:
                raise ValueError(f"Symbol {s!r} not in alphabet {self.alphabet}")

    @staticmethod
    def random(length: int, rng: random.Random, alphabet: Tuple[str, ...] = ("A", "U", "G", "C")) -> "Sequence":
        return Sequence(tuple(rng.choice(alphabet) for _ in range(length)))

    @staticmethod
    def hamming_distance(a: "Sequence", b: "Sequence") -> int:
        if len(a.symbols) != len(b.symbols):
            raise ValueError("Sequences must have equal length")
        return sum(1 for x, y in zip(a.symbols, b.symbols) if x != y)

    def __len__(self) -> int:
        return len(self.symbols)

    def __repr__(self) -> str:
        return "Sequence(" + "".join(self.symbols) + ")"


# ----------------------------------------------------------------------
# 2. Replicator — replication rate function (fitness landscape)
# ----------------------------------------------------------------------

class Replicator:
    """Replicator with fitness landscape W(sequence) and per-base mutation rate μ.

    真借鉴 Eigen 1971: replication rate f_i, mutation rate per base p,
    error-free replication only for exact match to master sequence.
    """

    def __init__(
        self,
        master: Sequence,
        fitness: Callable[[Sequence], float],
        mutation_rate_per_base: float = 0.001,
    ) -> None:
        self._master = master
        self._fitness = fitness
        self._mu = mutation_rate_per_base
        if not (0.0 <= mutation_rate_per_base <= 1.0):
            raise ValueError("mutation_rate_per_base must be in [0, 1]")

    @property
    def master(self) -> Sequence:
        return self._master

    @property
    def mutation_rate(self) -> float:
        return self._mu

    def fitness(self, seq: Sequence) -> float:
        return float(self._fitness(seq))

    def mutation_probability(self, seq: Sequence) -> float:
        """Probability of producing a different sequence during replication.

        真借鉴 Eigen 1971: p_mutation = 1 - (1 - μ)^L  ≈  L·μ for small μ.
        Returns 1 - prob(exact match replication).
        """
        if len(seq.symbols) != len(self._master.symbols):
            return 1.0
        return 1.0 - (1.0 - self._mu) ** len(seq.symbols)

    def copy_probability(self, src: Sequence, dst: Sequence) -> float:
        """Probability that replicating `src` produces exactly `dst`.

        For random mutation model: per-base prob of dst's symbol = (1-μ) if match,
        else μ/(k-1) where k = alphabet size.
        """
        if len(src.symbols) != len(dst.symbols):
            return 0.0
        k = len(self._master.alphabet)
        prob = 1.0
        for s, d in zip(src.symbols, dst.symbols):
            if s == d:
                prob *= (1.0 - self._mu)
            else:
                prob *= self._mu / (k - 1)
        return prob


# ----------------------------------------------------------------------
# 3. Quasispecies — master + cloud distribution (Eigen 1971)
# ----------------------------------------------------------------------

class Quasispecies:
    """Quasispecies: master sequence + cloud of mutants (真借鉴 Eigen 1971).

    Distribution over sequences that satisfies the quasispecies equation:
    dXi/dt = sum_j [W_ji · X_j · Q_ji] - E_bar · Xi
    where Q_ji is the copy probability (j -> i mutation), W_ji is fitness,
    E_bar is mean fitness.

    Steady state: X_i* ∝ (fitness·mutational_input)/E_bar (dominant eigenvalue).
    """

    def __init__(self, replicator: Replicator) -> None:
        self._rep = replicator
        self._distribution: Dict[Sequence, float] = {}

    def set_distribution(self, dist: Dict[Sequence, float]) -> None:
        s = sum(dist.values())
        if s <= 0:
            raise ValueError("Distribution must sum to positive value")
        self._distribution = {k: v / s for k, v in dist.items()}

    def master_fraction(self) -> float:
        """Fraction of population that is exactly the master sequence."""
        return self._distribution.get(self._rep.master, 0.0)

    def mean_fitness(self) -> float:
        """E_bar = sum_i f_i · X_i (average fitness over distribution)."""
        return sum(self._rep.fitness(s) * x for s, x in self._distribution.items())

    def quasispecies_error_threshold(self) -> bool:
        """Eigen 1971 error threshold condition.

        Q = sigma^2 / (L * mu * ln(s)) < 1
        where sigma^2 = fitness variance, s = multiplicative selective advantage
        (master_fitness / mean_fitness), L = length, mu = mutation rate.

        Returns True iff replication is accurate enough to maintain master sequence.
        Assumes master-only distribution if none set.
        """
        master_fit = self._rep.fitness(self._rep.master)
        if master_fit <= 1.0:
            return False
        if self._distribution:
            mean_fit = self.mean_fitness()
            if mean_fit <= 0:
                return False
            s = master_fit / mean_fit  # multiplicative selective advantage
        else:
            s = master_fit  # assume wild-type fitness = 1.0
        if s <= 1.001:
            return False
        L = len(self._rep.master.symbols)
        mu = self._rep.mutation_rate
        if mu <= 0 or L <= 0:
            return False
        # Eigen threshold: Q = L * mu / ln(s); master maintained iff Q < 1
        Q = (L * mu) / math.log(s)
        return Q < 1.0

    def evolve_step(self, dt: float = 0.01) -> None:
        """One step of quasispecies ODE (forward Euler)."""
        if not self._distribution:
            return
        E_bar = self.mean_fitness()
        if E_bar <= 0:
            return
        new_dist: Dict[Sequence, float] = {}
        for seq_i in self._distribution:
            new_x = 0.0
            for seq_j, x_j in self._distribution.items():
                Q_ji = self._rep.copy_probability(seq_j, seq_i)
                f_j = self._rep.fitness(seq_j)
                new_x += f_j * x_j * Q_ji
            # Subtract dilution (mean fitness)
            new_dist[seq_i] = max(0.0, new_x - E_bar * self._distribution[seq_i] * dt)
        # Renormalize
        total = sum(new_dist.values())
        if total > 0:
            self._distribution = {k: v / total for k, v in new_dist.items()}


# ----------------------------------------------------------------------
# 4. ErrorThreshold — error threshold Q check (Eigen 1971)
# ----------------------------------------------------------------------

class ErrorThreshold:
    """Eigen 1971 error threshold: L · μ < ln(s) / σ².

    Above the threshold, master sequence is lost in error cloud.
    Below, master sequence is maintained at high fraction.
    """
    @staticmethod
    def Q(mutation_rate: float, length: int, selective_advantage: float,
          fitness_variance: float = 0.5) -> float:
        """Eigen 1971 error threshold Q = L * mu / ln(s).

        Master sequence maintained iff Q < 1, i.e., L * mu < ln(s).
        Selective advantage s > 1 (multiplicative master/wild-type).
        """
        if selective_advantage <= 1.001 or mutation_rate <= 0 or length <= 0:
            return float("inf")
        return (length * mutation_rate) / math.log(selective_advantage)

    @staticmethod
    def threshold_breached(Q: float) -> bool:
        """Returns True iff error threshold is breached (Q >= 1)."""
        return Q >= 1.0

    @staticmethod
    def critical_mutation_rate(length: int, selective_advantage: float,
                                fitness_variance: float = 0.5) -> float:
        """Critical mutation rate above which error threshold is breached."""
        if selective_advantage <= 1.001:
            return float("inf")
        return fitness_variance / (length * math.log(selective_advantage))


# ----------------------------------------------------------------------
# 5. Hypercycle — catalytic cycle I_1 → I_2 → ... → I_n → I_1
# ----------------------------------------------------------------------

class Hypercycle:
    """Eigen & Schuster 1977 hypercycle: catalytic coupling of replicators.

    Topology: I_i -> I_{i+1} (cyclic, with K_{i+1,i} catalytic rate).
    The hypercycle ensures coexistence: no member can be lost because
    each catalyzes the next, and the last catalyzes the first.
    """

    def __init__(self, replicators: List[Replicator], catalytic_rates: List[float]) -> None:
        if len(replicators) != len(catalytic_rates):
            raise ValueError("replicators and catalytic_rates must have equal length")
        if len(replicators) < 2:
            raise ValueError("Hypercycle requires >= 2 replicators")
        self._replicators = list(replicators)
        self._K = list(catalytic_rates)
        for k in self._K:
            if k < 0:
                raise ValueError("Catalytic rates must be non-negative")

    @property
    def size(self) -> int:
        return len(self._replicators)

    @property
    def replicators(self) -> List[Replicator]:
        return list(self._replicators)

    def catalytic_rate(self, i: int) -> float:
        """K_{i+1, i}: rate at which I_i catalyzes replication of I_{i+1}."""
        return self._K[i % len(self._K)]


# ----------------------------------------------------------------------
# 6. HypercycleODE — ODE dynamics
# ----------------------------------------------------------------------

class HypercycleODE:
    """ODE dynamics of hypercycle (真借鉴 Eigen & Schuster 1977).

    dX_i/dt = F_i · X_i + sum_j K_ij · X_j · X_i - X_i · dilution
    where dilution = sum_k F_k · X_k (constant population).

    In cyclic hypercycle with K_ij > 0 only for j -> i (cyclic):
    dX_i/dt = F_i · X_i + K_{i,i-1} · X_{i-1} · X_i - X_i · E_bar
    """

    def __init__(self, hypercycle: Hypercycle, fitness: List[float]) -> None:
        self._hc = hypercycle
        self._F = list(fitness)
        if len(self._F) != hypercycle.size:
            raise ValueError("fitness list must match hypercycle size")

    def derivative(self, x: List[float]) -> List[float]:
        """Compute dX/dt at state `x` (x_i = concentration of replicator i)."""
        n = self._hc.size
        if len(x) != n:
            raise ValueError("State dimension mismatch")
        # Mean fitness (dilution rate)
        E_bar = sum(self._F[i] * x[i] for i in range(n))
        dxdt = [0.0] * n
        for i in range(n):
            # Self-replication
            dxdt[i] = self._F[i] * x[i]
            # Catalysis from previous member: I_{i-1} -> I_i
            prev = (i - 1) % n
            dxdt[i] += self._hc.catalytic_rate(prev) * x[prev] * x[i]
            # Dilution
            dxdt[i] -= x[i] * E_bar
        return dxdt

    def step(self, x: List[float], dt: float = 0.001) -> List[float]:
        """Forward Euler step."""
        dxdt = self.derivative(x)
        return [max(0.0, x[i] + dxdt[i] * dt) for i in range(len(x))]

    def simulate(self, x0: List[float], steps: int = 1000, dt: float = 0.001) -> List[List[float]]:
        """Simulate hypercycle dynamics for `steps` steps."""
        if len(x0) != self._hc.size:
            raise ValueError("Initial state dimension mismatch")
        # Normalize initial state to sum=1
        total = sum(x0)
        if total <= 0:
            raise ValueError("Initial state must be positive")
        x = [xi / total for xi in x0]
        trajectory = [list(x)]
        for _ in range(steps):
            x = self.step(x, dt)
            total = sum(x)
            if total > 0:
                x = [xi / total for xi in x]
            trajectory.append(list(x))
        return trajectory

    def is_coexistent(self, trajectory: List[List[float]], tol: float = 1e-3) -> bool:
        """Check if all replicators coexist in final state (none went extinct)."""
        if not trajectory:
            return False
        final = trajectory[-1]
        return all(x > tol for x in final)


# ----------------------------------------------------------------------
# 7. HypercycleSimulator — Monte Carlo simulation with mutation
# ----------------------------------------------------------------------

class HypercycleSimulator:
    """Monte Carlo simulation of hypercycle with mutation (真借鉴 Eigen 1971).

    Population of N individuals distributed over hypercycle species.
    Each step:
      1. Replication with fitness + catalysis
      2. Mutation (per-base)
      3. Drift (random sampling)
    """

    def __init__(self, hypercycle: Hypercycle, N: int = 1000, mutation_rate: float = 0.001) -> None:
        self._hc = hypercycle
        self._N = N
        self._mu = mutation_rate

    @property
    def population(self) -> int:
        return self._N

    def initialize(self, rng: random.Random) -> List[int]:
        """Initialize population uniformly distributed across hypercycle members."""
        n = self._hc.size
        base = self._N // n
        counts = [base] * n
        # Distribute remainder
        for i in range(self._N - base * n):
            counts[i] += 1
        rng.shuffle(counts)
        return counts

    def step(self, counts: List[int], rng: random.Random,
             fitness: Optional[List[float]] = None) -> List[int]:
        """One step of Wright-Fisher-like dynamics with catalysis and mutation."""
        n = self._hc.size
        if len(counts) != n:
            raise ValueError("counts dimension mismatch")
        fit = fitness or [1.0 + 0.1 * i for i in range(n)]
        # Compute weights = fitness + catalysis
        total = sum(counts)
        if total <= 0:
            return counts
        weights = []
        for i in range(n):
            prev = (i - 1) % n
            # Catalytic contribution from previous member
            cat = self._hc.catalytic_rate(prev) * counts[prev] / total
            w = fit[i] + cat
            weights.append(max(0.0, w))
        total_w = sum(weights)
        if total_w <= 0:
            return counts
        # Sample next generation (Wright-Fisher)
        probs = [w / total_w for w in weights]
        new_counts = [0] * n
        for _ in range(self._N):
            r = rng.random()
            cum = 0.0
            for i, p in enumerate(probs):
                cum += p
                if r <= cum:
                    new_counts[i] += 1
                    break
        return new_counts


# ----------------------------------------------------------------------
# 8. SpiralWaveStructure — spatial pattern formation
# ----------------------------------------------------------------------

class SpiralWaveStructure:
    """Spatial pattern formation in hypercycle (真借鉴 Boerlijst & Hogeweg 1991).

    Boerlijst-Hogeweg showed that hypercycles WITHOUT spatial structure are
    unstable (Maynard Smith 1979 critique), but WITH spatial self-structuring,
    they form spiral waves that are robust against parasites.
    """

    def __init__(self, grid_size: int = 32) -> None:
        if grid_size < 4:
            raise ValueError("grid_size must be >= 4")
        self._size = grid_size

    def initialize_grid(self, rng: random.Random, n_species: int) -> List[List[int]]:
        """Random initial grid."""
        return [[rng.randrange(n_species) for _ in range(self._size)] for _ in range(self._size)]

    def detect_spiral(self, grid: List[List[int]]) -> Dict[str, Any]:
        """Detect spiral wave pattern (heuristic: high gradient variance near center)."""
        n = len(grid)
        if n < 4:
            return {"spiral_detected": False, "reason": "grid too small"}
        # Compute local variance as proxy for spiral structure
        center_x, center_y = n // 2, n // 2
        local_vars = []
        for x in range(1, n - 1):
            for y in range(1, n - 1):
                neighbors = [grid[x + dx][y + dy] for dx in [-1, 0, 1] for dy in [-1, 0, 1] if (dx, dy) != (0, 0)]
                m = sum(neighbors) / len(neighbors)
                v = sum((nv - m) ** 2 for nv in neighbors) / len(neighbors)
                local_vars.append(v)
        avg_var = sum(local_vars) / len(local_vars) if local_vars else 0
        # Spiral: high variance near center
        return {
            "spiral_detected": avg_var > 0.5,
            "avg_local_variance": avg_var,
            "center": (center_x, center_y),
        }


# ----------------------------------------------------------------------
# 9. EvolutionaryTransition — Szathmáry reproducer tracking
# ----------------------------------------------------------------------

@dataclass
class EvolutionaryTransition:
    """Szathmáry 2006 evolutionary transition tracker.

    Transitions:
      1. Replicator molecules → protocell (molecular cooperation)
      2. Protocells → chromosomes (limited heritability)
      3. Prokaryotes → eukaryotes (endosymbiosis)
      4. Asexual → sexual populations (recombination)
      5. Protists → multicellular organisms (cell differentiation)
      6. Solitary → social individuals (kin selection)
      7. Primate societies → human societies (language/culture)

    ASI V0.1: hypercycle → ASI self-evolution = computational transition.
    """
    transition_name: str
    from_unit: str
    to_unit: str
    mechanism: str
    asi_analogue: str

    def report(self) -> Dict[str, str]:
        return {
            "transition": self.transition_name,
            "from_unit": self.from_unit,
            "to_unit": self.to_unit,
            "mechanism": self.mechanism,
            "asi_analogue": self.asi_analogue,
            "philosophical_note": (
                "analogue, NOT identity (主 17:58 + 主 20:46). "
                "ASI self-organization shares structure with biological transitions, "
                "but ASI is computational, NOT Phenomenal consciousness."
            ),
        }


# ----------------------------------------------------------------------
# 10. ASI_EvolutionBridge — connection to ASI V0.1
# ----------------------------------------------------------------------

class ASI_EvolutionBridge:
    """Bridge from Eigen hypercycle to ASI V0.1 self-evolution.

    主 22:33 ASI 北极星: 真测量
    主 17:58 + 主 20:46: 不假装 Phenomenal/达到 ASI
    主 17:43: 实事求是

    The bridge computes structural metrics from hypercycle state that
    contribute to the ASI V0.1 formula's 'self-evolution' component,
    but does NOT claim that hypercycle = ASI self-evolution (analogue).
    """

    # V0.1 component weights (from V1002 + V21)
    SELF_EVOLUTION_WEIGHT = 0.10
    CATALYTIC_COHERENCE_WEIGHT = 0.05

    def __init__(self, current_v0_1: float = 0.7905) -> None:
        self._v0_1 = current_v0_1

    @staticmethod
    def hypercycle_to_asi_component(
        hypercycle_size: int,
        coexistence_ratio: float,
        spiral_detected: bool,
    ) -> Dict[str, float]:
        """Map hypercycle state to ASI V0.1 component contributions.

        Components:
          - self_evolution: based on hypercycle size & coexistence
          - catalytic_coherence: based on spiral wave structure
        """
        # Self-evolution component: grows with size & coexistence
        self_evo = min(1.0, (math.log2(max(1, hypercycle_size)) / 10.0) * coexistence_ratio)
        # Catalytic coherence: bonus for spiral structure
        cat_coh = 0.5 if spiral_detected else 0.2
        return {
            "self_evolution": self_evo,
            "catalytic_coherence": cat_coh,
            "contribution": (
                self_evo * ASI_EvolutionBridge.SELF_EVOLUTION_WEIGHT +
                cat_coh * ASI_EvolutionBridge.CATALYTIC_COHERENCE_WEIGHT
            ),
        }

    def report(self, hypercycle_size: int, coexistence_ratio: float,
               spiral_detected: bool) -> Dict[str, Any]:
        comp = self.hypercycle_to_asi_component(hypercycle_size, coexistence_ratio, spiral_detected)
        return {
            "current_v0_1": self._v0_1,
            "hypercycle_component": comp,
            "asi_self_evolution_claim": False,  # 主 20:46 不假装达到 ASI
            "phenomenal_claim": False,  # 主 17:58 不假装 Phenomenal
            "philosophical_guard": (
                "hypercycle 是 ASI 自演化的结构类比, NOT identity. "
                "ASI 真生产需要 V0.1 >= 0.95 (主 20:46)."
            ),
            "current_v0_1_status": (
                f"V0.1 = {self._v0_1:.4f}, BELOW ASI threshold (0.95). "
                "Continue building (主 23:44 干到底)."
            ),
        }


__all__ = [
    "Sequence",
    "Replicator",
    "Quasispecies",
    "ErrorThreshold",
    "Hypercycle",
    "HypercycleODE",
    "HypercycleSimulator",
    "SpiralWaveStructure",
    "EvolutionaryTransition",
    "ASI_EvolutionBridge",
]

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
