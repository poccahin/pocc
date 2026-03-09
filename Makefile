.PHONY: build-planetary spin-planetary-cluster check-structure

build-planetary: check-structure
	@echo "[Life++] build-planetary: scaffolding verified"

spin-planetary-cluster:
	@echo "[Life++] spin-planetary-cluster: simulation bootstrap placeholder"

check-structure:
	@test -d 1_kinetic_trust_root
	@test -d 2_ahin_nervous_system
	@test -d 2.5_pocc_collaboration_mesh
	@test -d 3_cai_cognitive_cortex
	@test -d 4_thermodynamic_ledger
	@test -d 5_planetary_omnisphere
	@test -d 6_planetary_defense
	@test -d 7_koala_os_frontend
