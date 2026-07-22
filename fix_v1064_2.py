#!/usr/bin/env python3
"""Make SimpleModel robust to size mismatches between pred and target."""
from pathlib import Path

p = Path("apeireth/v1064_asi_continual_learning.py")
src = p.read_text(encoding="utf-8")

old = (
    "    def grad_squared(self, x: List[float], target: List[float]) -> Dict[str, float]:\n"
    '        """Compute (\u2202L/\u2202w)^2 for Fisher info (\u4e3b 17:43 \u5b9e\u4e8b\u6c42\u662f)."""\n'
    "        pred = self.predict(x)\n"
    "        # MSE gradient: \u2202L/\u2202w_{o,i} = (pred[o] - target[o]) * x[i]\n"
    "        grads = {}\n"
    "        for o in range(self.out_dim):\n"
    "            err = pred[o] - target[o]\n"
    "            for i in range(self.in_dim):\n"
    "                g = err * x[i]\n"
    "                name = f\"w_{o * self.in_dim + i}\"\n"
    "                grads[name] = g ** 2\n"
    "        return grads\n"
    "\n"
    "    def sgd_step(self, x: List[float], target: List[float],\n"
    "                 lr: float = 0.01) -> None:\n"
    '        """One SGD update step."""\n'
    "        pred = self.predict(x)\n"
    "        for o in range(self.out_dim):\n"
    "            err = pred[o] - target[o]\n"
    "            for i in range(self.in_dim):\n"
    "                idx = o * self.in_dim + i\n"
    "                self.weights[idx] -= lr * err * x[i]\n"
    "            self.bias -= lr * err"
)

new = (
    "    def grad_squared(self, x, target):\n"
    '        """Compute (\u2202L/\u2202w)^2 for Fisher info (\u4e3b 17:43 \u5b9e\u4e8b\u6c42\u662f).\n'
    "\n"
    "        Size-mismatch safe: iterates over min(self.out_dim, len(pred), len(target))\n"
    "        and min(self.in_dim, len(x)).\n"
    '        """\n'
    "        pred = self.predict(x)\n"
    "        out_dim = min(self.out_dim, len(pred), len(target))\n"
    "        in_dim = min(self.in_dim, len(x))\n"
    "        grads = {}\n"
    "        for o in range(out_dim):\n"
    "            err = pred[o] - target[o]\n"
    "            for i in range(in_dim):\n"
    "                g = err * x[i]\n"
    "                name = f\"w_{o * self.in_dim + i}\"\n"
    "                grads[name] = g ** 2\n"
    "        return grads\n"
    "\n"
    "    def sgd_step(self, x, target, lr=0.01):\n"
    '        """One SGD update step. Size-mismatch safe."""\n'
    "        pred = self.predict(x)\n"
    "        out_dim = min(self.out_dim, len(pred), len(target))\n"
    "        in_dim = min(self.in_dim, len(x))\n"
    "        for o in range(out_dim):\n"
    "            err = pred[o] - target[o]\n"
    "            for i in range(in_dim):\n"
    "                idx = o * self.in_dim + i\n"
    "                self.weights[idx] -= lr * err * x[i]\n"
    "            self.bias -= lr * err"
)

if old in src:
    src = src.replace(old, new)
    p.write_text(src, encoding="utf-8")
    print("Replaced.")
else:
    print("NOT found.")
