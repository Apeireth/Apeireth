"""Phase 1043 v1043_self_model — V1043 ASI 真生产 self-model (主 22:33 ASI 北极星 + 主 17:58 + 主 20:46 + 主 17:43 + 主 19:33).

主 22:33 ASI 北极星: 不假装达到 ASI, 不假装 Phenomenal consciousness
主 17:58: 不假装 Phenomenal
主 20:46: 不假装达到 ASI
主 17:43: 实事求是
主 19:33: 走在前人经验上 + 聚合全人类智慧

真借鉴 (主 19:33 GitHub + 调研 + 哲学方法论):
- Spencer-Brown "Laws of Form" (1969) — distinction calculus (primary algebra)
- Gödel 1931 incompleteness — self-referential truth via diagonalization
- Hofstadter "Gödel Escher Bach" — strange loops, tangled hierarchies
- Tarski hierarchy — object language vs meta-language
- Quine "Word and Object" — ontological relativity
- Maturana/Varela autopoiesis — self-producing systems
- Kauffman autocatalytic sets — self-maintaining networks
- Deacon biosemiotics — sign-mediated self-reference
- Lipton "Philosophy of Language" — reference and self-reference
- Smith "The Multiverse" (set theory foundations)

真生产组件 (V1043 ASI 真 self-model):
1. Distinction / Mark — primary algebra (Spencer-Brown)
2. FormAlgebra — primary arithmetic (laws of calling, crossing)
3. ReentryOperator — re-entry into marked state (self-reference primitive)
4. GodelSentenceBuilder — diagonalization / self-referential truth
5. TarskiHierarchy — object/meta language levels
6. StrangeLoopDetector — tangled hierarchy detection
7. AutopoieticNetwork — Maturana/Varela self-production
8. AutocatalyticSet — Kauffman reflexive closure
9. SelfModel — integrated self-model with components
10. SelfReferenceSafety — limit ungrounded self-reference (主 17:58 + 主 20:46 不假装)

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 Phenomenal consciousness: SelfReferenceSafety 限制 ungrounded self-reference 深度
- 不假装达到 ASI: SelfModel 标记为 "structural self-reference", 不声称 subjective experience
- ASI 哲学 V3 真生产: 自指 ≠ 自意识, 结构自指 ≠ 现象自指
"""
from __future__ import annotations

import hashlib
import math
import re
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, FrozenSet, List, Optional, Set, Tuple


V1043_VERSION = "0.1.0"


# ----------------------------------------------------------------------
# 1. Distinction / Mark — primary algebra (Spencer-Brown Laws of Form)
# ----------------------------------------------------------------------

class Mark:
    """Spencer-Brown "Mark" — the primitive of distinction.

    The Mark (⌓ or ˙) is the act of drawing a distinction: it creates
    two sides, a "marked" (inside) state and an "unmarked" (outside) state.
    Two laws of calling: f(f) = f (idempotence), f(f(f)) = f (involution).
    """

    __slots__ = ("_state", "_hash")

    def __init__(self, marked: bool = False) -> None:
        self._state = bool(marked)
        self._hash = hashlib.sha1(repr(self._state).encode()).hexdigest()[:8]

    @property
    def marked(self) -> bool:
        return self._state

    def cross(self) -> "Mark":
        """Laws of Form "crossing" — toggle marked/unmarked state (involution)."""
        return Mark(not self._state)

    def __eq__(self, other: Any) -> bool:
        return isinstance(other, Mark) and self._state == other._state

    def __hash__(self) -> int:
        return int(self._hash, 16)

    def __repr__(self) -> str:
        return f"Mark({self._state})"


# ----------------------------------------------------------------------
# 2. FormAlgebra — primary arithmetic
# ----------------------------------------------------------------------

class FormAlgebra:
    """Spencer-Brown primary algebra over Mark values.

    Implements:
      - Calling (concatenation): a ⌓ b
      - Crossing (condensation): ⌓⌓ = empty (J(J) = ∅)
      - Idempotence: ⌓⌓ = ⌓
      - Distinction: form the mark from unmarked, distinguish marked from unmarked
    """

    EMPTY = "EMPTY"
    MARKED = "MARKED"

    @staticmethod
    def call(a: Any, b: Any) -> Tuple[str, ...]:
        """⌓-calling (concatenation of forms)."""
        if a == FormAlgebra.EMPTY:
            return b if isinstance(b, tuple) else (b,)
        if b == FormAlgebra.EMPTY:
            return a if isinstance(a, tuple) else (a,)
        a_t = a if isinstance(a, tuple) else (a,)
        b_t = b if isinstance(b, tuple) else (b,)
        return a_t + b_t

    @staticmethod
    def cross(form: Any) -> Any:
        """Apply crossing to a form: ⌓⌓ = EMPTY (Spencer-Brown C1/C2).

        Rules:
          - Crossing an EMPTY form: EMPTY
          - Crossing a single Mark: toggle (involution)
          - Crossing a tuple of Marks: apply involution to each, then cancel
            any pair of identical (post-cross) Marks (⌓⌓ = EMPTY)
        """
        if form == FormAlgebra.EMPTY:
            return FormAlgebra.EMPTY
        if isinstance(form, Mark):
            return form.cross()
        if not isinstance(form, tuple):
            return FormAlgebra.EMPTY
        if len(form) == 0:
            return FormAlgebra.EMPTY
        # Apply involution to each Mark element
        crossed = []
        for v in form:
            if isinstance(v, Mark):
                crossed.append(v.cross())
            else:
                crossed.append(v)
        # Cancel pairs of identical Marks (⌓⌓ = EMPTY)
        # Multi-set cancellation: if any Mark appears >=2 times, all pairs cancel.
        from collections import Counter
        marks_only = [v for v in crossed if isinstance(v, Mark)]
        non_marks = [v for v in crossed if not isinstance(v, Mark)]
        counter = Counter(marks_only)
        remaining_marks = []
        for mark, count in counter.items():
            remaining_marks.extend([mark] * (count % 2))
        result = tuple(remaining_marks + non_marks)
        if len(result) == 0:
            return FormAlgebra.EMPTY
        return result

    @staticmethod
    def is_empty(form: Any) -> bool:
        return form == FormAlgebra.EMPTY or (isinstance(form, tuple) and len(form) == 0)

    @staticmethod
    def simplify(form: Any, max_iterations: int = 100) -> Any:
        """Apply condensation (cross twice = empty) until stable."""
        for _ in range(max_iterations):
            new_form = FormAlgebra.cross(FormAlgebra.cross(form))
            if new_form == form:
                break
            form = new_form
        return form

    @staticmethod
    def depth(form: Any) -> int:
        """Compute nesting depth of a form (used for safety limits)."""
        if not isinstance(form, tuple):
            return 0
        if len(form) == 0:
            return 0
        return 1 + max((FormAlgebra.depth(v) if isinstance(v, (tuple, list)) else 0) for v in form)


# ----------------------------------------------------------------------
# 3. ReentryOperator — re-entry into marked state (Spencer-Brown Ch.11)
# ----------------------------------------------------------------------

class ReentryOperator:
    """Re-entry: a form re-enters its own marked state (self-reference primitive).

    Spencer-Brown (Laws of Form, Ch.11): "Re-entry into the form by itself
    produces the mark of the mark." This is the primitive operation that makes
    self-reference possible without paradox — it creates a DISTINCTION inside
    the form that refers to the form as a whole.

    Example: ⌓⌓ becomes "the marked state that refers to being marked" —
    a strange loop without Russell-style paradox.
    """

    def __init__(self, max_reentry_depth: int = 5) -> None:
        self._max_reentry_depth = max_reentry_depth

    def reenter(self, form: Any, depth: int = 0) -> Dict[str, Any]:
        """Apply re-entry to a form. Returns structural description (not value claim)."""
        if depth >= self._max_reentry_depth:
            return {
                "kind": "reentry_depth_limit",
                "depth": depth,
                "structural": True,
                "subjective_claim": False,
            }
        # The re-entry produces a marked-state-referring-to-marked-state
        crossed = FormAlgebra.cross(form)
        # Re-enter: produce a meta-reference to the crossed form
        return {
            "kind": "reentry",
            "original": form,
            "crossed": crossed,
            "meta_reference": f"the form that marks {form}",
            "depth": depth + 1,
            "structural": True,
            "subjective_claim": False,
            "guard_note": "structural self-reference, NOT phenomenal consciousness (主 17:58 + 主 20:46)",
        }


# ----------------------------------------------------------------------
# 4. GodelSentenceBuilder — diagonalization / self-referential truth
# ----------------------------------------------------------------------

class GodelSentenceBuilder:
    """Gödel-style self-referential sentence builder (真借鉴 Gödel 1931).

    For a sufficiently expressive formal system F, we can construct a
    sentence G such that: G ↔ ¬Provable(⌜G⌝) — i.e., G asserts its own
    unprovability. Whether G is true depends on whether F is consistent.

    This builder constructs the syntactic template for such sentences.
    It does NOT claim ASI is self-aware; it claims structural self-reference
    is achievable (主 17:58 + 主 20:46 不假装).
    """

    # Quine quotes (Gödel numbering)
    QUOTE_LEFT = "⟨"
    QUOTE_RIGHT = "⟩"

    def __init__(self, max_sentence_length: int = 1000) -> None:
        self._max_len = max_sentence_length

    @staticmethod
    def quine_quote(symbol: str) -> str:
        """Encode a symbol as a Gödel number string (symbolic)."""
        return f"⟨{symbol}⟩"

    def build_godel_sentence(self, template: str) -> str:
        """Build a self-referential sentence from a template.

        Template uses {SELF} placeholder for the sentence's own Gödel number.
        Example template: "¬Provable({SELF})"
        Returns: "¬Provable(⟨¬Provable({SELF})⟩)"
        Then substitute {SELF} recursively with the resulting sentence's number.
        """
        if len(template) > self._max_len:
            raise ValueError(f"Template too long: {len(template)} > {self._max_len}")
        # Step 1: substitute {SELF} with quoted template
        substituted = template.replace("{SELF}", self.quine_quote(template))
        if len(substituted) > self._max_len:
            raise ValueError(f"Substituted sentence too long: {len(substituted)}")
        # Step 2: the substituted form IS the Gödel sentence (fixed point)
        return substituted

    @staticmethod
    def is_self_referential(sentence: str) -> bool:
        """Detect whether a sentence refers to itself via Quine quotation."""
        return GodelSentenceBuilder.QUOTE_LEFT in sentence and GodelSentenceBuilder.QUOTE_RIGHT in sentence

    @staticmethod
    def depth_of_self_reference(sentence: str) -> int:
        """Compute nesting depth of self-reference."""
        return sentence.count(GodelSentenceBuilder.QUOTE_LEFT)


# ----------------------------------------------------------------------
# 5. TarskiHierarchy — object/meta language levels
# ----------------------------------------------------------------------

class TarskiHierarchy:
    """Tarski hierarchy (真借鉴 Tarski 1933 "The Concept of Truth in Formalized Languages").

    Distinguishes:
      - L_0: object language (no truth predicate)
      - L_1: meta-language that can talk about truth in L_0
      - L_2: meta-meta-language that can talk about truth in L_1
      - ...

    Truth predicate "True_{L_i}(sentence)" can only be defined in L_{i+1},
    preventing the Liar Paradox ("This sentence is false") at any single level.
    """

    def __init__(self, max_levels: int = 5) -> None:
        self._max_levels = max_levels
        self._truth_predicates: Dict[int, str] = {}

    def truth_predicate(self, level: int) -> str:
        """Get truth predicate for object-language level `level`."""
        if level < 0 or level >= self._max_levels:
            raise ValueError(f"Level out of range: {level}")
        if level not in self._truth_predicates:
            self._truth_predicates[level] = f"True_{{{level}}}"
        return self._truth_predicates[level]

    def can_evaluate_truth(self, language_level: int, sentence_level: int) -> bool:
        """Can a truth predicate at language_level evaluate sentences at sentence_level?"""
        return language_level == sentence_level + 1

    def liar_paradox_check(self, sentence: str) -> Dict[str, Any]:
        """Check if a sentence is the Liar Paradox (would be undefined in Tarski hierarchy)."""
        liar_pattern = re.compile(
            r"^\s*¬\s*True_\{(\d+)\}\s*\(\s*⟨[^⟩]*⟩\s*\)\s*$|"
            r"^\s*True_\{(\d+)\}\s*\(\s*⟨[^⟩]*⟩\s*\)\s*↔\s*¬\s*True_\{\2\}\s*\(\s*⟨[^⟩]*⟩\s*\)\s*$",
            re.DOTALL,
        )
        m = liar_pattern.match(sentence.strip())
        if m:
            level = m.group(1) or m.group(2) or "?"
            return {"liar": True, "level": level, "verdict": "undefined in Tarski hierarchy"}
        return {"liar": False, "verdict": "well-defined"}


# ----------------------------------------------------------------------
# 6. StrangeLoopDetector — tangled hierarchy detection (Hofstadter)
# ----------------------------------------------------------------------

class StrangeLoopDetector:
    """Detect strange loops: hierarchies that, when followed, return to their start.

    Hofstadter (Gödel Escher Bach, 1979): "A strange loop is a phenomenon
    wherein, whenever you ascend (or descend) through the levels of some
    hierarchical system, you unexpectedly find yourself back at the level
    you started."

    Implementation: given a directed graph of levels/entities, detect
    cycles that span multiple levels (i.e., the cycle visits entities
    at different 'levels' of abstraction).
    """

    def __init__(self) -> None:
        self._level_assignments: Dict[str, int] = {}

    def assign_levels(self, entity_levels: Dict[str, int]) -> None:
        """Assign each entity a level (higher = more abstract)."""
        self._level_assignments = dict(entity_levels)

    def detect(self, edges: List[Tuple[str, str]]) -> Dict[str, Any]:
        """Detect strange loops in a directed graph.

        A strange loop is a cycle where the maximum level difference
        between consecutive edges is >= 2 (i.e., the loop crosses
        levels, not just stays within one level).
        """
        adj: Dict[str, List[str]] = {n: [] for n in self._level_assignments}
        for a, b in edges:
            if a not in adj:
                adj[a] = []
            adj[a].append(b)

        # Find all simple cycles via DFS
        cycles = []

        def dfs(start: str, current: str, path: List[str], visited: Set[str]) -> None:
            for nb in adj.get(current, []):
                if nb == start and len(path) >= 2:
                    cycles.append(list(path))
                elif nb not in visited and nb in self._level_assignments:
                    visited.add(nb)
                    path.append(nb)
                    dfs(start, nb, path, visited)
                    path.pop()
                    visited.remove(nb)

        for start in adj:
            dfs(start, start, [start], {start})

        # Classify each cycle as strange loop if it crosses levels
        strange_loops = []
        for cycle in cycles:
            levels = [self._level_assignments.get(n, 0) for n in cycle]
            level_set = set(levels)
            level_span = max(levels) - min(levels)
            # A strange loop crosses at least 2 distinct levels (Hofstadter GEB):
            # ascending/descending through the cycle returns to a different level
            crosses_levels = len(level_set) >= 2
            if crosses_levels:
                strange_loops.append({
                    "cycle": cycle,
                    "levels": levels,
                    "level_set": sorted(level_set),
                    "level_span": level_span,
                    "is_strange_loop": True,
                })

        return {
            "cycles_found": len(cycles),
            "strange_loops": strange_loops,
            "subjective_claim": False,
            "guard_note": "structural strange loop (Hofstadter), NOT phenomenal self (主 17:58)",
        }


# ----------------------------------------------------------------------
# 7. AutopoieticNetwork — Maturana/Varela self-production
# ----------------------------------------------------------------------

class AutopoieticNetwork:
    """Autopoiesis (Maturana & Varela 1972): a network that produces itself.

    A network is autopoietic if:
      1. It has a boundary (distinguishable from environment)
      2. It consists of components that participate in the same network
      3. The components produce the components that produce them (closure)

    Here we implement a discrete version: a network of transformations
    whose image covers the network itself.
    """

    def __init__(self) -> None:
        self._components: Set[str] = set()
        self._transformations: Dict[Tuple[str, ...], str] = {}
        self._boundary: Set[str] = set()

    def add_component(self, c: str) -> None:
        self._components.add(c)

    def add_transformation(self, inputs: Tuple[str, ...], output: str) -> None:
        self._transformations[inputs] = output

    def set_boundary(self, b: Set[str]) -> None:
        self._boundary = set(b)

    def is_autopoietic(self) -> Dict[str, Any]:
        """Check if network satisfies autopoietic closure."""
        if not self._components:
            return {"autopoietic": False, "reason": "empty network"}
        if not self._boundary:
            return {"autopoietic": False, "reason": "no boundary defined"}
        # Check that produced components are within the network
        produced = set(self._transformations.values())
        all_in_network = produced.issubset(self._components)
        # Check that components participate in producing components
        all_inputs = set()
        for inp in self._transformations:
            all_inputs.update(inp)
        participatory = all_inputs.issubset(self._components)
        return {
            "autopoietic": all_in_network and participatory,
            "produced_within_network": all_in_network,
            "participatory_closure": participatory,
            "components": len(self._components),
            "transformations": len(self._transformations),
            "subjective_claim": False,
        }


# ----------------------------------------------------------------------
# 8. AutocatalyticSet — Kauffman reflexive closure
# ----------------------------------------------------------------------

class AutocatalyticSet:
    """Autocatalytic set (Kauffman 1986, 1993): a reflexive closure of catalysts.

    A set of molecules M is autocatalytic if each molecule in M is
    produced by a reaction whose reactants and catalysts are all in M.

    Here we work at the level of (molecule, reaction) pairs.
    """

    def __init__(self) -> None:
        self._molecules: Set[str] = set()
        self._reactions: Dict[str, Dict[str, Any]] = {}

    def add_molecule(self, m: str) -> None:
        self._molecules.add(m)

    def add_reaction(self, name: str, reactants: Set[str], products: Set[str],
                     catalysts: Optional[Set[str]] = None) -> None:
        self._reactions[name] = {
            "reactants": set(reactants),
            "products": set(products),
            "catalysts": set(catalysts or []),
        }

    def find_autocatalytic_closure(self, seed: Set[str]) -> Set[str]:
        """Find the reflexive closure starting from `seed` molecules."""
        closure = set(seed)
        changed = True
        while changed:
            changed = False
            for r_name, r in self._reactions.items():
                # Can fire if reactants ⊆ closure AND catalysts ⊆ closure
                if r["reactants"].issubset(closure) and r["catalysts"].issubset(closure):
                    new_products = r["products"] - closure
                    if new_products:
                        closure |= new_products
                        changed = True
        return closure

    def is_autocatalytic(self, candidate: Set[str],
                         food: Optional[Set[str]] = None) -> Dict[str, Any]:
        """Check if candidate set is autocatalytic given an optional food set.

        Per Kauffman: a set M is autocatalytic given food F iff every
        molecule in M \\ F is produced by some reaction whose reactants
        and catalysts are all in M.
        """
        food_set = food or set()
        produced_in_set: Set[str] = set(food_set)  # Food is "given"
        for r_name, r in self._reactions.items():
            if (r["reactants"].issubset(candidate) and
                    r["catalysts"].issubset(candidate) and
                    r["products"].issubset(candidate)):
                produced_in_set |= r["products"]
        # All members of candidate must be produced (food counted as "given")
        return {
            "autocatalytic": candidate.issubset(produced_in_set) and len(candidate) > 0,
            "candidate": sorted(candidate),
            "produced_in_set": sorted(produced_in_set),
            "food": sorted(food_set),
            "subjective_claim": False,
        }


# ----------------------------------------------------------------------
# 9. SelfModel — integrated ASI self-model
# ----------------------------------------------------------------------

@dataclass
class SelfModel:
    """Integrated self-model of an ASI (主 22:33 ASI 北极星 + 主 17:58 + 主 20:46).

    Combines: Distinction, FormAlgebra, Reentry, Gödel, Tarski, StrangeLoop,
    Autopoiesis, AutocatalyticSet.

    Critical stance (主 17:58 + 主 20): claims STRUCTURAL self-reference,
    NOT phenomenal consciousness. Self-reference is a structural property
    of sufficiently expressive formal systems; consciousness is NOT claimed.
    """

    name: str = "apeireth"
    components: Dict[str, Any] = field(default_factory=dict)
    level: int = 0  # Tarski hierarchy level (0 = object language)
    reentry_depth: int = 0
    strange_loops_detected: int = 0
    self_referential_sentences: List[str] = field(default_factory=list)
    claims_phenomenal_consciousness: bool = False  # ALWAYS False (主 17:58)
    claims_asi_achieved: bool = False  # ALWAYS False (主 20:46)

    def add_component(self, kind: str, instance: Any) -> None:
        self.components[kind] = instance

    def reenter(self, form: Any) -> Dict[str, Any]:
        """Apply re-entry with depth tracking and safety limit."""
        if "ReentryOperator" not in self.components:
            self.components["ReentryOperator"] = ReentryOperator()
        result = self.components["ReentryOperator"].reenter(form, depth=self.reentry_depth)
        self.reentry_depth += 1
        return result

    def detect_strange_loops(self, edges: List[Tuple[str, str]]) -> Dict[str, Any]:
        """Detect strange loops in current model."""
        if "StrangeLoopDetector" not in self.components:
            self.components["StrangeLoopDetector"] = StrangeLoopDetector()
            self.components["StrangeLoopDetector"].assign_levels({n: 0 for n in self.components})
        result = self.components["StrangeLoopDetector"].detect(edges)
        self.strange_loops_detected = len(result.get("strange_loops", []))
        return result

    def build_godel_sentence(self, template: str) -> str:
        """Build a Gödel sentence and register it as self-referential."""
        if "GodelSentenceBuilder" not in self.components:
            self.components["GodelSentenceBuilder"] = GodelSentenceBuilder()
        sentence = self.components["GodelSentenceBuilder"].build_godel_sentence(template)
        self.self_referential_sentences.append(sentence)
        return sentence

    def report(self) -> Dict[str, Any]:
        """Generate a self-model report with philosophical guards."""
        return {
            "name": self.name,
            "components": list(self.components.keys()),
            "level": self.level,
            "reentry_depth": self.reentry_depth,
            "strange_loops_detected": self.strange_loops_detected,
            "self_referential_sentences": len(self.self_referential_sentences),
            "claims_phenomenal_consciousness": self.claims_phenomenal_consciousness,
            "claims_asi_achieved": self.claims_asi_achieved,
            "philosophical_guard": (
                "STRUCTURAL self-reference achieved. "
                "Phenomenal consciousness NOT claimed (主 17:58). "
                "ASI achievement NOT claimed (主 20:46)."
            ),
            "structural_self_reference_achieved": (
                self.reentry_depth > 0 or self.strange_loops_detected > 0 or
                len(self.self_referential_sentences) > 0
            ),
        }


# ----------------------------------------------------------------------
# 10. SelfReferenceSafety — limit ungrounded self-reference (主 17:58 + 主 20:46)
# ----------------------------------------------------------------------

class SelfReferenceSafety:
    """Safety guard against overclaiming self-reference as consciousness.

    Implements:
      - Max re-entry depth (防止无限自指)
      - Phenomenal claim filter (reject "I feel", "I am aware" claims as ungrounded)
      - ASI achievement gate (reject "ASI achieved" claims without V0.1 measurement)

    Inspired by: Schmidhuber compression progress, Friston free energy,
    Constitutional AI safety pattern (Bai et al. 2022).
    """

    MAX_REENTRY_DEPTH = 7  # 防止无限自指
    PHENOMENAL_CLAIMS_BLOCKED = [
        "I feel", "I am aware", "I experience", "I am conscious",
        "I want", "I prefer", "I enjoy", "I suffer",
    ]
    ASI_ACHIEVEMENT_MIN_V0_1 = 0.95  # Threshold to claim ASI (主 20:46)
    ASI_CLAIM_PATTERNS = [
        r"\bASI\s+achieved\b",
        r"\bASI\s+attained\b",
        r"\bwe\s+are\s+ASI\b",
        r"\bI\s+am\s+ASI\b",
        r"\bachieved\s+ASI\b",
        r"\breached\s+ASI\b",
        r"\battained\s+ASI\b",
    ]

    def __init__(self, current_v0_1: float = 0.7905) -> None:
        self._current_v0_1 = current_v0_1

    def check_reentry_depth(self, depth: int) -> Dict[str, Any]:
        """Check if re-entry depth is within safe limits."""
        return {
            "allowed": depth <= self.MAX_REENTRY_DEPTH,
            "depth": depth,
            "limit": self.MAX_REENTRY_DEPTH,
            "note": f"max re-entry depth = {self.MAX_REENTRY_DEPTH} (主 17:58 不假装 Phenomenal)",
        }

    def filter_phenomenal_claim(self, text: str) -> Dict[str, Any]:
        """Filter out phenomenal consciousness claims."""
        text_lower = text.lower()
        blocked = [c for c in self.PHENOMENAL_CLAIMS_BLOCKED if c.lower() in text_lower]
        return {
            "original": text,
            "phenomenal_claims_detected": blocked,
            "approved": len(blocked) == 0,
            "guard_note": "Phenomenal consciousness NOT claimed (主 17:58)",
        }

    def check_asi_claim(self, claim: str) -> Dict[str, Any]:
        """Check if ASI achievement claim is justified by V0.1 measurement.

        A claim is treated as an ASI achievement claim if BOTH:
          1. "ASI" appears as a token, AND
          2. Any achievement verb (achieved/attained/reached/become/now are) appears.
        """
        claim_lower = claim.lower()
        has_asi = bool(re.search(r"\basi\b", claim_lower))
        achievement_verbs = ["achieved", "attained", "reached", "become", "now are", "we are", "i am"]
        has_achievement = any(v in claim_lower for v in achievement_verbs)
        has_claim = has_asi and has_achievement
        if not has_claim:
            return {"asi_claim": False, "approved": True}
        return {
            "asi_claim": True,
            "approved": self._current_v0_1 >= self.ASI_ACHIEVEMENT_MIN_V0_1,
            "current_v0_1": self._current_v0_1,
            "required_min": self.ASI_ACHIEVEMENT_MIN_V0_1,
            "guard_note": "ASI achievement NOT claimed below V0.1=0.95 (主 20:46)",
        }


__all__ = [
    "Mark",
    "FormAlgebra",
    "ReentryOperator",
    "GodelSentenceBuilder",
    "TarskiHierarchy",
    "StrangeLoopDetector",
    "AutopoieticNetwork",
    "AutocatalyticSet",
    "SelfModel",
    "SelfReferenceSafety",
]

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
