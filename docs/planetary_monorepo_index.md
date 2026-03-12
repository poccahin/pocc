# 🌌 `poccahin/pocc` 全栈工程索引（Planetary Core Blueprint）

本文档将 Life++ Planetary Core 的目标化目录结构统一归档，作为本地初始化与后续重构的参考蓝图。

## 📂 目标目录树

```text
pocc/
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   └── lprfc_template.md
│   └── workflows/
│       └── planetary_build.yml
├── 0_kinetic_trust_root/
├── 1_ahin_nervous_system/
├── 2_pocc_collaboration_mesh/
├── 3_thermodynamic_ledger/
├── 4_ap2_universal_gateway/
├── 5_planetary_defense/
├── 6_koala_os_omnisphere/
├── 7_openclaw_skills/
├── docs/
├── tests/
├── Makefile
└── README.md
```

## 🧭 现有仓库与目标蓝图的路径映射

| 目标路径 | 当前实现路径 |
| --- | --- |
| `0_kinetic_trust_root` | `1_kinetic_trust_root` |
| `1_ahin_nervous_system` | `2_ahin_nervous_system` |
| `2_pocc_collaboration_mesh` | `2.5_pocc_collaboration_mesh` |
| `3_thermodynamic_ledger` | `programs/lifeplus_core` + `4_thermodynamic_ledger` |
| `4_ap2_universal_gateway` | `gateway/ap2_universal_router` |
| `5_planetary_defense` | `6_planetary_defense` |
| `6_koala_os_omnisphere` | `7_koala_os_frontend` |
| `7_openclaw_skills` | `openclaw-skills` |

## 🛠️ 统一构建入口

根目录 `Makefile` 已采用统一目标：

- `make all`
- `make boot-l0`
- `make compile-l1-l4`
- `make train-l2.5`
- `make build-l7`
- `make ignite-testnet`


## 📚 关联文档

- `docs/lifeplusplus_planetary_engineering_blueprint_zh.md`：L0-L4 分层、CTx v0 结构、MVP→Production 路线与安全失效模式。
