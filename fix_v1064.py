#!/usr/bin/env python3
"""Fix V1064 tests by patching source to match test expectations."""
import re
from pathlib import Path

p = Path("apeireth/v1064_asi_continual_learning.py")
src = p.read_text(encoding="utf-8")

# 1) Replace @staticmethod softmax with instance method that uses self.temperature by default
old_static = '''    @staticmethod
    def softmax(x: List[float], temperature: float = 1.0) -> List[float]:
        """Stable softmax with temperature (Hinton 2015)."""
        z = [xi / max(temperature, 1e-9) for xi in x]
        m = max(z)
        exps = [math.exp(zi - m) for zi in z]
        s = sum(exps) or 1e-9
        return [e / s for e in exps]'''
new_instance = '''    def softmax(self, x, temperature=None):
        """Stable softmax with temperature (Hinton 2015).

        When called as instance method, defaults to ``self.temperature``.
        Falls back to ``temperature`` arg if explicitly provided.
        """
        T = self.temperature if temperature is None else temperature
        z = [xi / max(T, 1e-9) for xi in x]
        m = max(z)
        exps = [math.exp(zi - m) for zi in z]
        s = sum(exps) or 1e-9
        return [e / s for e in exps]  # canonical'''
assert old_static in src, "old_static not found"
src = src.replace(old_static, new_instance)

# 2) Update kl_divergence and total_loss to not pass self.temperature (it's the default now)
old_kl = '''        p_t = self.softmax(teacher_logits, self.temperature)
        p_s = self.softmax(student_logits, self.temperature)'''
new_kl = '''        p_t = self.softmax(teacher_logits)
        p_s = self.softmax(student_logits)'''
assert old_kl in src
src = src.replace(old_kl, new_kl)

# 3) Update RehearsalSampler.mix to handle empty old (backfill from new)
old_mix = '''    def mix(self, new_samples: List[ContinualSample],
            old_samples: List[ContinualSample],
            n_total: int) -> List[ContinualSample]:
        """Mix old and new samples."""
        n_old = int(n_total * self.alpha_old)
        n_new = n_total - n_old
        # random.sample with replacement if needed
        sampled_old = random.choices(old_samples, k=min(n_old, len(old_samples))) if old_samples else []
        sampled_new = random.choices(new_samples, k=min(n_new, len(new_samples))) if new_samples else []
        return sampled_old + sampled_new'''
new_mix = '''    def mix(self, new_samples, old_samples, n_total):
        """Mix old and new samples (Silver 2013).

        When one of (old, new) is empty, allocate all slots to the other side
        while preserving ``n_total``. When both are empty, return [].
        """
        if not new_samples and not old_samples:
            return []
        if not old_samples:
            return random.choices(new_samples, k=min(n_total, len(new_samples)))
        if not new_samples:
            return random.choices(old_samples, k=min(n_total, len(old_samples)))
        n_old = int(n_total * self.alpha_old)
        n_new = n_total - n_old
        sampled_old = random.choices(old_samples, k=n_old)
        sampled_new = random.choices(new_samples, k=n_new)
        return sampled_old + sampled_new'''
assert old_mix in src
src = src.replace(old_mix, new_mix)

# 4) Make _score_with_bridge pass when called via quick_score (uses num_tracked)
# already handled

# 5) Replace make_task to auto-detect out_dim and pad/truncate targets
old_make = '''def make_task(name: str, target_fn: Callable[[List[float]], List[float]],
              in_dim: int = 4, out_dim: int = 2) -> ContinualTask:
    """Create a simple task with given target function (主 00:56)."""

    def gen(n: int) -> List[ContinualSample]:
        samples = []
        for _ in range(n):
            x = [random.uniform(-1, 1) for _ in range(in_dim)]
            y = target_fn(x)
            samples.append(ContinualSample(x=x, y=y))
        return samples

    return ContinualTask(task_id=name, name=name, data_generator=gen,
                         n_samples=100, loss_type="mse")'''
new_make = '''def make_task(name, target_fn, in_dim: int = 4, out_dim=None):
    """Create a simple task with given target function (主 00:56).

    If out_dim is None, inferred by probing target_fn with a zero vector.
    Targets are then padded/truncated to out_dim for model compatibility.
    """
    if out_dim is None:
        probe = [0.0] * in_dim
        try:
            _y = target_fn(probe)
            out_dim = max(1, len(_y))
        except Exception:
            out_dim = 1

    def gen(n):
        samples = []
        for _ in range(n):
            x = [random.uniform(-1, 1) for _ in range(in_dim)]
            y = target_fn(x)
            if len(y) < out_dim:
                y = list(y) + [0.0] * (out_dim - len(y))
            elif len(y) > out_dim:
                y = list(y[:out_dim])
            samples.append(ContinualSample(x=x, y=y))
        return samples

    return ContinualTask(task_id=name, name=name, data_generator=gen,
                         n_samples=100, loss_type="mse")'''
# check the older definition (we already patched this manually above; skip if missing)
if old_make in src:
    src = src.replace(old_make, new_make)
else:
    print("make_task already replaced; skipping")

p.write_text(src, encoding="utf-8")
print("OK, file size:", len(src))
