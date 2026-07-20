#!/usr/bin/env python3
"""Phase 34 + 35 test — Maturana 自创生 + Bertalanffy GST."""
import sys
sys.path.insert(0, '.')

from apeireth.autopoiesis import AUTOPOIESIS_VERSION, AutopoieticComponent, AutopoieticSystem
from apeireth.systems_theory import GST_VERSION, GST_PRINCIPLES, SystemsTheoryLibrary
print(f'Phase 34 Autopoiesis: {AUTOPOIESIS_VERSION}')
print(f'Phase 35 GST: {GST_VERSION}')

# Autopoiesis test
apo = AutopoieticSystem('apeireth_central')
b = apo.add_component('boundary', '中央 AI 边界 (主人 12:27 边界)')
m1 = apo.add_component('producer', 'persona:调度者', produces=[b.comp_id])
m2 = apo.add_component('producer', 'skill:deliberate', produces=[m1.comp_id, b.comp_id])
apo.register_production(m1.comp_id, b.comp_id)
apo.register_production(m2.comp_id, m1.comp_id)
apo.register_production(m2.comp_id, b.comp_id)
print(f'  Autopoiesis: {apo.stats()}')

# GST test
gst = SystemsTheoryLibrary()
gst.apply_principle('wholeness', '中央 AI 整体 > 4 archetype 之总和', evidence='Bateson Mind=生态')
gst.apply_principle('isomorphy', '跨域调研真生产 = 系统论同构', evidence='AnySearch 8 跨域')
gst.apply_principle('self_organization', '涌现 自组织', evidence='Phase 25 NicheConstructor')
print(f'  GST: {gst.stats()}')
q = 'wholeness'
hits = gst.search(q)
print(f'  search wholeness: {len(hits)} hit')
print('OK Phase 34+35 work')
