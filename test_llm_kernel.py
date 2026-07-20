#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test LLM Kernel."""
import sys
sys.path.insert(0, '.')
from apeireth.llm_kernel import (
    LLM_KERNEL_VERSION, LLMConfig, LLMResponse,
    call_llm_minimax, call_llm_template, make_call_llm
)
print(f'LLM Kernel version: {LLM_KERNEL_VERSION}')

cfg = LLMConfig.minimax_default()
print(f'default config: provider={cfg.provider} model={cfg.model} base_url={cfg.base_url}')

resp = call_llm_minimax('Test ASI thinking')
print(f'minimax resp: provider={resp.provider} content_len={len(resp.content)}')

template_call = call_llm_template
print(f'template: {template_call("hello")[:80]}')

factory_call = make_call_llm('minimax')
print(f'factory minimax: {factory_call("hello")[:80]}')

print('OK Phase 21 LLM Kernel works')
