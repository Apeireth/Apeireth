#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test Layer 2 (HOT) + Layer 4 (SMM)."""
from apeireth.meta_cognition import MetaMonitor, MetaReview, FailurePattern, META_COGNITION_VERSION
from apeireth.self_model import (
    SELF_MODEL_VERSION, SomaticMarkers, SelfObject, SelfModel, make_default_self_model
)

# Layer 2
print(f"MetaCognition version: {META_COGNITION_VERSION}")
mm = MetaMonitor()
trace = ["Step 1: setup OK", "Step 2: query failed - error fetching", "Step 3: try again, fail again", "Step 4: success"]
outcomes = [{"status": "ok"}, {"status": "fail"}, {"status": "fail"}, {"status": "ok"}]
review = mm.review("cycle_001", trace, outcomes)
print(f"  Review ID: {review.review_id}")
print(f"  Confidence: {review.confidence:.2f}")
print(f"  Failure patterns: {len(mm.failure_patterns)}")
print(f"  Failure summary: {mm.get_failure_summary()}")

# Layer 4
print(f"\nSelfModel version: {SELF_MODEL_VERSION}")
sm = make_default_self_model()
state = sm.query()
print(f"  self_id: {state['self_id']}")
print(f"  mood: {state['overall_mood']}")
print(f"  feel: {sm.feel()}")
print(f"  insights: {state['insights']}")
print(f"  predict research: {sm.predict_impact('research')}")
print(f"  predict reflect: {sm.predict_impact('reflect')}")

# 更新 somatic
sm.update_somatic(engagement=0.8, curiosity=0.9)
print(f"  after update: {sm.feel()}")

print("\nOK Layer 2 + 4 both work")
