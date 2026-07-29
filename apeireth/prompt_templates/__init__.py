"""V1122 prompt_templates 包 — 跨模块 prompt 模板加载器 (主 19:33 走在前人经验上).

真借鉴 (主 19:33):
- V1011 PromptTemplate dataclass (零依赖, {var} 占位)
- Jinja2 {{ var }} 但简化为 str.format_map 风格避免硬依赖

只调不重 (主 23:44 干到底): 该 __init__ 仅做模板发现 + 渲染, 不发明 DSL.
"""
from .loader import (
    list_templates,
    load_template,
    render_template,
    TEMPLATE_DIR,
    V1122_TPL_VERSION,
)

__all__ = [
    "list_templates",
    "load_template",
    "render_template",
    "TEMPLATE_DIR",
    "V1122_TPL_VERSION",
]
