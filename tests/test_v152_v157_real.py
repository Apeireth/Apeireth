"""V152-V157 真生产 tests (主 22:27 不空壳 + 主 22:30)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest

import apeireth.v152_opencog_atomspace as v152_mod
import apeireth.v153_aera_autocatalytic as v153_mod
import apeireth.v154_nars_revision as v154_mod
import apeireth.v155_dgm_sakana as v155_mod
import apeireth.v156_world_model_full as v156_mod
import apeireth.v157_ppo_full as v157_mod

V152OpenCogAtomSpace = v152_mod.V152OpenCogAtomSpace
AtomType = v152_mod.AtomType
TruthValue = v152_mod.TruthValue
AttentionValue = v152_mod.AttentionValue
V153AERAAutocatalytic = v153_mod.V153AERAAutocatalytic
AERAComponent = v153_mod.AERAComponent
AERAProcess = v153_mod.AERAProcess
AERAState = v153_mod.AERAState
V154NARSRevision = v154_mod.V154NARSRevision
NARSBelief = v154_mod.NARSBelief
nars_revision_rule = v154_mod.nars_revision_rule
V155DGMSakana = v155_mod.V155DGMSakana
DGMArchiveAgent = v155_mod.DGMArchiveAgent
dgm_ucb1 = v155_mod.dgm_ucb1
V156WorldModelFull = v156_mod.V156WorldModelFull
WMState = v156_mod.WMState
DreamerPrediction = v156_mod.DreamerPrediction
V157PPOFull = v157_mod.V157PPOFull
PPOBuffer = v157_mod.PPOBuffer
compute_gae = v157_mod.compute_gae
ppo_clip_loss = v157_mod.ppo_clip_loss


class TestV152V157Batch:
    """V152-V157 真生产 batch tests (主 22:27 不空壳 + 主 22:30 一次推完)."""

    # V152 OpenCog Hyperon AtomSpace
    def test_v152_add_concept(self):
        s = V152OpenCogAtomSpace(); aid = s.add_concept("Apeireth")
        assert aid in s.atoms
        assert s.atoms[aid].atom_type == AtomType.CONCEPT

    def test_v152_add_predicate(self):
        s = V152OpenCogAtomSpace(); aid = s.add_predicate("test")
        assert s.atoms[aid].atom_type == AtomType.PREDICATE

    def test_v152_add_link_inheritance(self):
        s = V152OpenCogAtomSpace()
        a1 = s.add_concept("parent"); a2 = s.add_concept("child")
        lid = s.add_link(AtomType.LINK_INHERITANCE, [a1, a2])
        assert s.atoms[lid].atom_type == AtomType.LINK_INHERITANCE

    def test_v152_spawn_attention(self):
        s = V152OpenCogAtomSpace(); aid = s.add_concept("test")
        result = s.spawn_attention(aid, 50.0)
        assert result is True
        assert s.atoms[aid].av.sti >= 50.0

    def test_v152_decay(self):
        s = V152OpenCogAtomSpace(); aid = s.add_concept("test")
        s.spawn_attention(aid, 50.0)
        before = s.atoms[aid].av.sti
        s.decay_attention(decay_rate=0.1)
        assert s.atoms[aid].av.sti < before

    def test_v152_pattern_match(self):
        s = V152OpenCogAtomSpace(); s.add_concept("a"); s.add_concept("b")
        s.add_concept("c", 1.0, 0.5)
        matches = s.pattern_match(AtomType.CONCEPT, min_confidence=0.8)
        assert len(matches) >= 2

    def test_v152_stats(self):
        s = V152OpenCogAtomSpace(); stats = s.stats()
        assert "n_atoms" in stats
        assert stats["version"] == v152_mod.V152_VERSION

    # V153 AERA Autocatalytic
    def test_v153_add_component(self):
        s = V153AERAAutocatalytic(); cid = s.add_component("test", produces=["x"])
        assert s.components[cid].produces == ["x"]

    def test_v153_add_process(self):
        s = V153AERAAutocatalytic(); pid = s.add_process("p", inputs=["a"], outputs=["b"])
        assert s.processes[pid].outputs == ["b"]

    def test_v153_state_autopoietic(self):
        s = V153AERAAutocatalytic()
        c1 = s.add_component("perception", produces=["reasoning_input"], requires=["sensory_data"])
        c2 = s.add_component("reasoning", produces=["action_plan"], requires=["reasoning_input"])
        c3 = s.add_component("action", produces=["sensory_data"], requires=["action_plan"])
        sid = s.create_state([c1, c2, c3], [])
        assert s.states[sid].is_autopoietic is True

    def test_v153_state_endogenous(self):
        s = V153AERAAutocatalytic()
        c1 = s.add_component("c1"); p1 = s.add_process("p1")
        sid = s.create_state([c1], [p1])
        assert s.states[sid].is_endogenous is True

    def test_v153_stats(self):
        s = V153AERAAutocatalytic(); s.add_component("c")
        stats = s.stats()
        assert stats["n_components"] == 1

    # V154 NARS Revision
    def test_v154_add_belief(self):
        s = V154NARSRevision(); bid = s.add_belief("test belief")
        assert s.beliefs[bid].content == "test belief"

    def test_v154_revise(self):
        s = V154NARSRevision(); bid = s.add_belief("test")
        old_tv = s.beliefs[bid].tv
        s.revise_belief(bid, (1.0, 0.9))
        assert s.beliefs[bid].tv != old_tv

    def test_v154_revision_rule(self):
        tv = nars_revision_rule([(1.0, 0.5), (1.0, 0.5)])
        assert tv[0] == 1.0
        assert 0 < tv[1] <= 1.0

    def test_v154_experience_grounded(self):
        s = V154NARSRevision()
        bid = s.add_belief("ASI 北极星")
        for _ in range(5):
            s.revise_belief(bid, (1.0, 0.9))
        decide = s.experience_grounded_decide("ASI", evidence_count=3)
        assert decide is True

    def test_v154_stats(self):
        s = V154NARSRevision(); stats = s.stats()
        assert "n_beliefs" in stats
        assert stats["version"] == v154_mod.V154_VERSION

    # V155 DGM Sakana
    def test_v155_add_to_archive(self):
        s = V155DGMSakana(); s.add_to_archive("a1", code_repr="code1", fitness=0.5)
        assert "a1" in s.archive

    def test_v155_ucb1_zero_visits(self):
        score = dgm_ucb1(0.5, 0, 10)
        assert score == float("inf")

    def test_v155_select_parent(self):
        s = V155DGMSakana()
        s.add_to_archive("p1", fitness=0.5)
        s.add_to_archive("p2", fitness=0.7)
        parent = s.select_parent_ucb1(["p1", "p2"])
        assert parent in ["p1", "p2"]

    def test_v155_record_evaluation(self):
        s = V155DGMSakana()
        s.add_to_archive("p1", fitness=0.5)
        s.record_evaluation("c1", "p1", 0.8)
        assert s.archive["p1"].fitness == 0.8

    def test_v155_stats(self):
        s = V155DGMSakana(); s.add_to_archive("a")
        stats = s.stats()
        assert stats["n_agents"] == 1

    # V156 World Model Full
    def test_v156_encode(self):
        wm = V156WorldModelFull(latent_dim=8, hidden_dim=16)
        sid = wm.encode_observation("obs1")
        assert sid in wm.states
        assert len(wm.states[sid].latent_z) == 8

    def test_v156_dream_step(self):
        wm = V156WorldModelFull(latent_dim=4, hidden_dim=8)
        sid = wm.encode_observation("obs1")
        pid = wm.dream_step(sid, action="a1", reward=0.5)
        assert pid in [p.pred_id for p in wm.predictions]

    def test_v156_imagine_rollout(self):
        wm = V156WorldModelFull(latent_dim=4, hidden_dim=8)
        sid = wm.encode_observation("obs1")
        rollout = wm.imagine_rollout(sid, n_steps=3, actions=["a1", "a2", "a3"])
        assert len(rollout) == 3

    def test_v156_stats(self):
        wm = V156WorldModelFull(); stats = wm.stats()
        assert "latent_dim" in stats

    # V157 PPO Full
    def test_v157_ppo_clip_loss(self):
        loss, ratio = ppo_clip_loss(-1.0, -0.5, 1.0, eps=0.2)
        assert ratio > 0
        assert loss < 0

    def test_v157_compute_gae(self):
        advs = compute_gae([1.0, 0.5, 0.0], [0.5, 0.6, 0.7], [False, False, True])
        assert len(advs) == 3

    def test_v157_add_rollout(self):
        ppo = V157PPOFull(); ppo.add_rollout("obs", "act", 1.0, 0.5, -1.0)
        assert ppo.n_rollout_steps() == 1

    def test_v157_ppo_update(self):
        ppo = V157PPOFull()
        loss, ratio = ppo.ppo_update_step(-1.0, -0.5, 1.0)
        assert ppo.n_ppo_steps() == 1

    def test_v157_clear(self):
        ppo = V157PPOFull()
        ppo.add_rollout("obs", "act", 1.0, 0.5, -1.0)
        ppo.clear_buffer()
        assert ppo.n_rollout_steps() == 0

    def test_v157_stats(self):
        ppo = V157PPOFull(); stats = ppo.stats()
        assert "gamma" in stats
        assert stats["version"] == v157_mod.V157_VERSION