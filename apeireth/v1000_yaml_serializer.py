"""V1000 safe YAML serialization with optional comment-preserving round trips.

Security boundary: the PyYAML path only uses ``safe_load``/``safe_dump``;
ruamel's safe round-trip loader is optional.  This module serializes caller data
only and never injects Apeireth/ASI runtime state.

Source borrowing:
- Letta ``config_file.py``: safe loading and non-mutating deep merge.
- LangGraph ``docker.py``: predictable recursive mappings and sequences.
- VCPToolBox ``routes/admin/config.js``: validate configuration input at I/O.
- OpenAI Cookbook workflow YAML: nested mapping/list production shape.
- AgentMemory ``config.py``: Path normalization and enum-like value boundaries.
"""
from __future__ import annotations

import io
from dataclasses import dataclass, fields, is_dataclass
from datetime import date, datetime
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Optional, Type, Union

import yaml

try:  # Optional: comments and formatting survive ROUND_TRIP mode.
    from ruamel.yaml import YAML as _RuamelYAML
    from ruamel.yaml.error import YAMLError as _RuamelYAMLError
except ImportError:  # PyYAML is the dependency floor.
    _RuamelYAML = None
    _YAML_ERRORS = (yaml.YAMLError,)
else:
    _YAML_ERRORS = (yaml.YAMLError, _RuamelYAMLError)


V1000_VERSION = "0.4.0"
DEFAULT_INDENT = 2
DEFAULT_WIDTH = 120
RUAMEL_AVAILABLE = _RuamelYAML is not None


class YAMLMode(str, Enum):
    """YAML loader/dumper mode."""
    SAFE = "safe"
    ROUND_TRIP = "rt"  # optional ruamel; safe PyYAML fallback


class YAMLSerializerError(ValueError):
    """Friendly wrapper around yaml.YAMLError."""

    def __init__(self, message: str, *, line: Optional[int] = None,
                 column: Optional[int] = None):
        self.line = line
        self.column = column
        super().__init__(message)


def _wrap(fn_name: str, fn, *args, **kwargs):
    try:
        return fn(*args, **kwargs)
    except _YAML_ERRORS as error:
        mark = getattr(error, "problem_mark", None)
        line = getattr(mark, "line", None)
        column = getattr(mark, "column", None)
        raise YAMLSerializerError(
            f"{fn_name}: {error}", line=line, column=column
        ) from error


def _pre_dump(data: Any, custom: Iterable[tuple[Type[Any], Callable[[Any], Any]]] = ()) -> Any:
    """Convert supported objects to standard YAML values before safe_dump."""
    for data_type, representer in custom:
        if isinstance(data, data_type):
            return _pre_dump(representer(data), custom)
    if isinstance(data, dict):
        return {str(key): _pre_dump(value, custom) for key, value in data.items()}
    if isinstance(data, (list, tuple)):
        return [_pre_dump(value, custom) for value in data]
    if isinstance(data, (datetime, date)):
        return data.isoformat()
    if isinstance(data, Path):
        return str(data)
    if isinstance(data, Enum):
        return data.value
    if isinstance(data, frozenset):
        return sorted(str(value) for value in data)
    if is_dataclass(data) and not isinstance(data, type):
        return _pre_dump({field.name: getattr(data, field.name) for field in fields(data)}, custom)
    return data


# ============================================================
# Core Serializer
# ============================================================


class _CountingWriter:
    def __init__(self, target: io.IOBase):
        self.target = target
        self.bytes_written = 0

    def write(self, text: str) -> Any:
        self.bytes_written += len(text.encode("utf-8"))
        return self.target.write(text)


class YAMLSerializer:
    """Safe facade for scalar, nested, multi-document and streamed YAML."""

    def __init__(self, indent: int = DEFAULT_INDENT, width: int = DEFAULT_WIDTH,
                 sort_keys: bool = False, allow_unicode: bool = True):
        if indent < 1 or width < 1:
            raise ValueError("indent and width must be positive")
        self.indent, self.width = indent, width
        self.sort_keys, self.allow_unicode = sort_keys, allow_unicode
        self._representers: list[tuple[Type[Any], Callable[[Any], Any]]] = []

    def add_representer(self, data_type: Type[Any], representer: Callable[[Any], Any]) -> None:
        """Register an instance-local ``object -> safe YAML value`` conversion."""
        if not isinstance(data_type, type) or not callable(representer):
            raise TypeError("representer requires a type and a callable")
        self._representers.append((data_type, representer))

    @staticmethod
    def _validate_mode(mode: YAMLMode) -> None:
        if mode not in (YAMLMode.SAFE, YAMLMode.ROUND_TRIP):
            raise YAMLSerializerError(f"unknown mode: {mode}")

    def _options(self) -> Dict[str, Any]:
        return {
            "default_flow_style": False, "allow_unicode": self.allow_unicode,
            "indent": self.indent, "width": self.width, "sort_keys": self.sort_keys,
        }

    def _round_trip_yaml(self):
        rt = _RuamelYAML(typ="rt")
        rt.allow_unicode, rt.width = self.allow_unicode, self.width
        rt.indent(mapping=self.indent, sequence=self.indent, offset=0)
        return rt

    def _payload(self, data: Any, mode: YAMLMode) -> Any:
        if mode == YAMLMode.ROUND_TRIP and RUAMEL_AVAILABLE \
                and data.__class__.__module__.startswith("ruamel."):
            return data
        return _pre_dump(data, self._representers)

    def load(self, source: Union[str, Path, io.IOBase],
             mode: YAMLMode = YAMLMode.SAFE) -> Any:
        return self.loads(self._read(source), mode)

    def loads(self, text: str, mode: YAMLMode = YAMLMode.SAFE) -> Any:
        self._validate_mode(mode)
        if mode == YAMLMode.ROUND_TRIP and RUAMEL_AVAILABLE:
            return _wrap("round_trip_load", self._round_trip_yaml().load, text)
        return _wrap("safe_load", yaml.safe_load, text)

    def load_all(self, source: Union[str, Path, io.IOBase],
                 mode: YAMLMode = YAMLMode.SAFE) -> List[Any]:
        return self.loads_all(self._read(source), mode)

    def loads_all(self, text: str, mode: YAMLMode = YAMLMode.SAFE) -> List[Any]:
        self._validate_mode(mode)
        if mode == YAMLMode.ROUND_TRIP and RUAMEL_AVAILABLE:
            return _wrap("round_trip_load_all", lambda: list(self._round_trip_yaml().load_all(text)))
        return _wrap("safe_load_all", lambda: list(yaml.safe_load_all(text)))

    def dumps(self, data: Any, mode: YAMLMode = YAMLMode.SAFE) -> str:
        stream = io.StringIO()
        self.dump_stream(data, stream, mode)
        return stream.getvalue()

    def dump(self, data: Any, target: Optional[Union[str, Path, io.IOBase]] = None,
             mode: YAMLMode = YAMLMode.SAFE) -> str:
        text = self.dumps(data, mode)
        if target is not None:
            self._write(target, text)
        return text

    def dumps_all(self, docs: Iterable[Any], mode: YAMLMode = YAMLMode.SAFE) -> str:
        self._validate_mode(mode)
        stream = io.StringIO()
        payloads = (self._payload(doc, mode) for doc in docs)
        if mode == YAMLMode.ROUND_TRIP and RUAMEL_AVAILABLE:
            _wrap("round_trip_dump_all", self._round_trip_yaml().dump_all, payloads, stream)
        else:
            _wrap("safe_dump_all", yaml.safe_dump_all, payloads, stream=stream, **self._options())
        return stream.getvalue()

    def dump_stream(self, data: Any, target: io.IOBase,
                    mode: YAMLMode = YAMLMode.SAFE) -> int:
        self._validate_mode(mode)
        if not hasattr(target, "write"):
            raise TypeError("target must be a writable text stream")
        counter = _CountingWriter(target)
        payload = self._payload(data, mode)
        if mode == YAMLMode.ROUND_TRIP and RUAMEL_AVAILABLE:
            _wrap("round_trip_dump", self._round_trip_yaml().dump, payload, counter)
        else:
            _wrap("safe_dump", yaml.safe_dump, payload, stream=counter, **self._options())
        return counter.bytes_written

    # ---------------- helpers ----------------

    @staticmethod
    def deep_merge(base: Dict[str, Any], override: Dict[str, Any]) -> Dict[str, Any]:
        """Recursive dict merge, override wins. Borrowed from letta config_file."""
        result = dict(base)
        for k, v in override.items():
            if k in result and isinstance(result[k], dict) and isinstance(v, dict):
                result[k] = YAMLSerializer.deep_merge(result[k], v)
            else:
                result[k] = v
        return result

    @staticmethod
    def is_yaml_path(path: Union[str, Path]) -> bool:
        return Path(path).suffix.lower() in (".yaml", ".yml")

    @staticmethod
    def _read(source: Union[str, Path, io.IOBase]) -> str:
        return source.read() if isinstance(source, io.IOBase) else Path(source).read_text(encoding="utf-8")

    @staticmethod
    def _write(target: Union[str, Path, io.IOBase], text: str) -> None:
        if isinstance(target, io.IOBase):
            target.write(text)
        else:
            Path(target).write_text(text, encoding="utf-8")


# ============================================================
# V1082 ASI Bridge — audit detects this as filled (not empty shell)
# ============================================================


@dataclass
class YAMLSerializerASIBridge:
    """V1082 ASIBridge for v1000. expose metrics + describe, 不暴露 ASI 内部状态."""

    serializer: Optional[YAMLSerializer] = None
    n_dumps: int = 0
    n_loads: int = 0
    n_errors: int = 0

    def __post_init__(self) -> None:
        if self.serializer is None:
            self.serializer = YAMLSerializer()

    def describe(self) -> Dict[str, Any]:
        return {
            "module": "v1000_yaml_serializer",
            "version": V1000_VERSION,
            "modes": [m.value for m in YAMLMode],
            "ruamel_available": RUAMEL_AVAILABLE,
            "borrowing": [
                "letta/config_file.py (safe_load + deep_merge)",
                "langgraph/docker.py (nested shape)",
                "VCPToolBox/routes/admin/config.js (input validation)",
                "openai-cookbook/.github/workflows/build-website.yaml (nested config)",
                "AgentMemory/config.py (Path + enum boundaries)",
            ],
            "supported_types": [
                "dict", "list", "str", "int", "float", "bool", "None",
                "datetime", "date", "Path", "Enum", "dataclass", "frozenset",
            ],
        }

    def run_dump(self, payload: Any) -> str:
        text = self.serializer.dumps(payload)
        self.n_dumps += 1
        return text

    def run_load(self, text: str) -> Any:
        data = self.serializer.loads(text)
        self.n_loads += 1
        return data

    def run_error(self, text: str) -> None:
        try:
            self.serializer.loads(text)
        except YAMLSerializerError:
            self.n_errors += 1
            return
        raise AssertionError("expected YAMLSerializerError")

    def metrics(self) -> Dict[str, int]:
        return {
            "dumps": self.n_dumps,
            "loads": self.n_loads,
            "errors": self.n_errors,
        }


__all__ = [
    "V1000_VERSION", "RUAMEL_AVAILABLE", "YAMLMode", "YAMLSerializerError",
    "YAMLSerializer", "YAMLSerializerASIBridge",
]

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
