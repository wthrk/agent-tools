# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0](https://github.com/wthrk/agent-tools/compare/v1.0.0...v1.1.0) (2026-02-04)


### Features

* add jj support for update and rebase commands ([#28](https://github.com/wthrk/agent-tools/issues/28)) ([b6dc55a](https://github.com/wthrk/agent-tools/commit/b6dc55aad6751cd37d819ea8b4ad3b3afcaddb3a))


### Bug Fixes

* address Copilot review feedback on reviewing skill ([#31](https://github.com/wthrk/agent-tools/issues/31)) ([ea546e9](https://github.com/wthrk/agent-tools/commit/ea546e93c1e0519bc3f0e3217368059fdb6c5768))
* improve skill auto-trigger descriptions ([#32](https://github.com/wthrk/agent-tools/issues/32)) ([547f536](https://github.com/wthrk/agent-tools/commit/547f536697065a68d90e1963318913b860f34811))
* use jj diff instead of jj diff --stat for clean check ([#33](https://github.com/wthrk/agent-tools/issues/33)) ([a2049e3](https://github.com/wthrk/agent-tools/commit/a2049e34b132eb05b9cfcd0095d76ef4f879a48c))

## [1.0.0](https://github.com/wthrk/agent-tools/compare/v0.1.0...v1.0.0) (2026-02-03)


### ⚠ BREAKING CHANGES

* skill-test and skill-test-core crates have been removed.

### Features

* add build command and skill new command ([#14](https://github.com/wthrk/agent-tools/issues/14)) ([f5def2a](https://github.com/wthrk/agent-tools/commit/f5def2a039f5f895f11d2fb452bd0995e870cb4f))
* add global CLAUDE.md and hooks management ([#23](https://github.com/wthrk/agent-tools/issues/23)) ([1b36dee](https://github.com/wthrk/agent-tools/commit/1b36deebfa6409185b01cb022ee8a465ae3b54c8))
* add managing-agents-md skill ([#22](https://github.com/wthrk/agent-tools/issues/22)) ([e609c77](https://github.com/wthrk/agent-tools/commit/e609c7785e8835299754543f74a0877f716adbb9))
* add responding-copilot-reviews skill ([#25](https://github.com/wthrk/agent-tools/issues/25)) ([bc15551](https://github.com/wthrk/agent-tools/commit/bc155515e2e02f6606a2bfac94f1ee825b27e2a7))
* add skill management commands and creating-skills skill ([#16](https://github.com/wthrk/agent-tools/issues/16)) ([ff8bb2f](https://github.com/wthrk/agent-tools/commit/ff8bb2f5af45c31033bcb1f2896a7b668775c743))
* add skill-test and skill-tools CLI ([23709a3](https://github.com/wthrk/agent-tools/commit/23709a32404a1b593ec89e97475ac0477ad1e445))
* **jj-skill:** add concurrent execution documentation ([#19](https://github.com/wthrk/agent-tools/issues/19)) ([e15fc7e](https://github.com/wthrk/agent-tools/commit/e15fc7ef546ea7a4da4c8fe60bc8242c178df103))
* **skill-test:** add skill-test CLI and test fixtures ([5fc0c0a](https://github.com/wthrk/agent-tools/commit/5fc0c0a8f1e36ffd9f9652fe63cec0678cda6aa8))
* **skill-test:** add skill-test-core library ([0076a1c](https://github.com/wthrk/agent-tools/commit/0076a1c94ad880e3b5742587300350a83a04efc8))
* **skill-test:** Phase 7-8 output unification and improvements ([a372370](https://github.com/wthrk/agent-tools/commit/a372370e1ae3ee34b58b500d68b91425667aae4f))
* **skill-tools:** add skill-tools CLI for managing Claude Code skills ([c08fcc5](https://github.com/wthrk/agent-tools/commit/c08fcc5e096c1c0107a3447d45f9ea091d0fc073))
* **skills:** add searching-skills definition ([3a3cd46](https://github.com/wthrk/agent-tools/commit/3a3cd4658c3ffe688eb263a6fdd627808d52a2a5))


### Bug Fixes

* **ci:** correct release-plz config field name ([#4](https://github.com/wthrk/agent-tools/issues/4)) ([f4695c5](https://github.com/wthrk/agent-tools/commit/f4695c58abd5d4dafe4fe54aa5d8df36b89b36c2))
* disable cargo publish in release-plz ([#18](https://github.com/wthrk/agent-tools/issues/18)) ([53af14e](https://github.com/wthrk/agent-tools/commit/53af14eb9012854db666f55d53dfe899b2f47873))
* improve skill-tools installation and error messages ([a2a4bfc](https://github.com/wthrk/agent-tools/commit/a2a4bfc37f5fd11aa428d01eda61542c39cc6391))
* remove publish = false from Cargo.toml for release-plz compatibility ([#13](https://github.com/wthrk/agent-tools/issues/13)) ([41f4086](https://github.com/wthrk/agent-tools/commit/41f4086002431efafe7e37277f5ce042c8ea81e0))


### Code Refactoring

* remove skill-test functionality ([#27](https://github.com/wthrk/agent-tools/issues/27)) ([fca3620](https://github.com/wthrk/agent-tools/commit/fca36209afcffada356e1c6a37137f3e87966808))

## [Unreleased]

## [0.1.0](https://github.com/wthrk/agent-tools/releases/tag/v0.1.0) - 2026-02-01

### Other

- *(deps)* bump colored from 2.2.0 to 3.1.1 ([#9](https://github.com/wthrk/agent-tools/pull/9))
- *(deps)* bump directories from 5.0.1 to 6.0.0 ([#10](https://github.com/wthrk/agent-tools/pull/10))
- add release automation with release-plz
- replace cargo-make with xtask
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
