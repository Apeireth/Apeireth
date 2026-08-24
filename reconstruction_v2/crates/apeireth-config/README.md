# apeireth-config

> v2 port of v1.0-legacy/apeireth-config (complete API surface preserved).

ConfigEntry slice operations:
- validate / validate_all
- lookup
- merge (base + overlay)
- missing_required
- diff (added / removed / changed)
- key_is_valid
- merge_three_layers (default + file + override)
- parse_json_layer / to_json_layer

Organ Kani proofs in organ_kani_proofs.rs.
