# Research Tracker: Contract Testing, REST Docs, and Knowledge Feedback

:PROPERTIES:
:ID: xiuxian-testing-research-tracker
:PARENT: [[../index]]
:TAGS: research, papers, tracking
:STATUS: ACTIVE
:LAST_REVIEWED: 2026-03-17
:END:

## Purpose

Track the post-2024 research line that matters for the next evolution of `xiuxian-testing`.

This tracker is not just a bibliography. Each entry is mapped to one of four adoption questions:

1. How do we derive contracts from code or docs?
2. How do we convert contracts into tests or oracles?
3. How do we detect drift between docs, code, and runtime?
4. How do we export findings in a form that LLM systems and Wendao can reuse?

## Tracking Rubric

Use the following statuses when updating this page:

- `seed`: worth reading, not yet translated into design
- `apply`: directly informs a V1 design choice
- `watch`: promising, but needs revalidation or later implementation
- `benchmark`: useful for evaluation methodology more than system design

## 2025-2026 Core Reading Set

| Theme                      | Paper                                                                                                                                                                                                                                                       | Venue / Date                          | Status      | Why it matters for `xiuxian-testing`                                                                                                                                                                |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Code to contract           | [OpenAI for OpenAPI: Automated generation of REST API specification via LLMs](https://arxiv.org/abs/2601.12735)                                                                                                                                             | arXiv, 2026-01-19                     | `apply`     | Strong evidence that dependency-aware, staged extraction can turn source surfaces into high-quality OpenAPI-like contracts. Useful for future `code -> contract` collectors.                        |
| Docs to contract           | [Generating OpenAPI Specifications from Online API Documentation with Large Language Models](https://aclanthology.org/volumes/2025.acl-industry/)                                                                                                           | ACL Industry, July 2025               | `apply`     | Shows that long-form API docs can be normalized into OAS with a mixed LLM plus rules pipeline. Useful for `docs -> contract` and doc parsing strategy.                                              |
| Contract to tests          | [LlamaRestTest: Effective REST API Testing with Small Language Models](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/51/LlamaRestTest-Effective-REST-API-Testing-with-Small-Language-Models)                                         | FSE, 2025-06-24                       | `apply`     | Demonstrates that compact, fine-tuned models plus response feedback can outperform larger generic models for REST testing. Supports a bounded advisory layer instead of general-purpose LLM gating. |
| Contract to oracles        | [SATORI: Static Test Oracle Generation for REST APIs](https://conf.researchr.org/details/ase-2025/ase-2025-papers/52/SATORI-Static-Test-Oracle-Generation-for-REST-APIs)                                                                                    | ASE, 2025                             | `apply`     | Directly supports turning OpenAPI contracts into reusable test oracles. Strong fit for a future `rest_docs` rule pack plus generated assertions.                                                    |
| Docs versus behavior       | [METAMON: Finding Inconsistencies between Program Documentation and Behavior using Metamorphic LLM Queries](https://arxiv.org/abs/2502.02794)                                                                                                               | arXiv, 2025-02-05                     | `apply`     | Important precedent for finding drift between documentation and observed program behavior. Useful for `docs-vs-impl` contract checks.                                                               |
| Parameter-constraint drift | [Identifying Multi-Parameter Constraint Errors in Python Data Science Library API Documentations](https://conf.researchr.org/details/issta-2025/issta-2025-papers/67/Identifying-Multi-Parameter-Constraint-Errors-in-Python-Data-Science-Library-API-Docu) | ISSTA, 2025                           | `watch`     | Useful for later, stricter semantic checks where docs encode cross-parameter constraints not captured by schemas alone.                                                                             |
| Runtime invariants         | [MINES: Explainable Anomaly Detection through Web API Invariant Inference](https://conf.researchr.org/details/icse-2026/icse-2026-research-track/118/MINES-Explainable-Anomaly-Detection-through-Web-API-Invariant-Inference)                               | ICSE, 2026                            | `watch`     | Bridges schema, logs, and inferred invariants. Strong candidate for a post-V1 runtime rule pack.                                                                                                    |
| Modular LLM systems        | [Oracular Programming: A Modular Foundation for Building LLM-Enabled Software](https://arxiv.org/abs/2502.05310)                                                                                                                                            | arXiv, 2025-02-07, updated 2026-02-24 | `apply`     | Provides a clean separation between explicit strategy, policy, and demonstrations. Strong theory for splitting deterministic rules from advisory heuristics.                                        |
| LLM application testing    | [Rethinking Testing for LLM Applications: Characteristics, Challenges, and a Lightweight Interaction Protocol](https://arxiv.org/abs/2508.20737)                                                                                                            | arXiv, 2025-08-28                     | `apply`     | Useful system decomposition: shell layer, orchestration layer, inference core. Supports a layered design for `xiuxian-testing`.                                                                     |
| LLM verification limits    | [Uncovering Systematic Failures of LLMs in Verifying Code Against Natural Language Specifications](https://arxiv.org/abs/2508.12358)                                                                                                                        | ASE NIER, 2025-08-17                  | `apply`     | Critical guardrail: do not let LLMs be the only pass/fail judge for natural-language requirements.                                                                                                  |
| Review-guided testing      | [Following Dragons: Code Review-Guided Fuzzing](https://arxiv.org/abs/2602.10487)                                                                                                                                                                           | arXiv, 2026-02-11                     | `watch`     | Suggests a path from review findings to guided testing. Useful later for turning audit comments into test focus areas.                                                                              |
| Benchmark design           | [A Survey of Code Review Benchmarks and Evaluation Practices in Pre-LLM and LLM Era](https://arxiv.org/abs/2602.13377)                                                                                                                                      | arXiv, 2026-02-13                     | `benchmark` | Helps define evaluation tasks, taxonomy, and benchmark strategy for future `xiuxian-testing` quality studies.                                                                                       |
| Architecture governance    | [Software Architecture Meets LLMs: A Systematic Literature Review](https://publikationen.bibliothek.kit.edu/1000181963)                                                                                                                                     | KIT publication, 2025                 | `apply`     | Confirms that architecture conformance and modularity governance remain under-served. Supports the `modularity` rule-pack focus.                                                                    |

## Design Conclusions Derived on 2026-03-17

### 1. V1 should not start from automatic generation

The paper set supports generation-heavy futures, but the most reliable V1 path is:

- normalize explicit contracts first,
- evaluate deterministic mismatches second,
- keep LLM assistance advisory and bounded.

### 2. REST and documentation consistency are the best first frontier

This is where the literature is strongest right now:

- OAS generation
- REST input generation
- REST oracle generation
- documentation and behavior inconsistency detection

This justifies making `rest_docs` the first serious rule pack.

### 3. Modularity is under-supported and therefore strategically valuable

The literature is less mature here, which means repo-specific, high-quality rule engineering will matter more than benchmark chasing. This is where `xiuxian-testing` can become differentiated.

### 4. LLMs should enrich findings, not own truth

The strongest caution in the reading set is consistent:

- LLMs are useful for extraction, classification, and suggestion
- they are not reliable enough to be sole arbiters of specification compliance

## Continuous Tracking Workflow

When adding a new paper:

1. Add title, source link, venue, and date.
2. Tag it with one of `seed`, `apply`, `watch`, or `benchmark`.
3. Record which adoption question it informs.
4. Update the design conclusions only if the paper changes a current architectural assumption.

## Next Review Queue

- Revisit `MINES` when V1 reaches runtime monitoring.
- Revisit `Following Dragons` when Wendao finding export is stable enough to guide test targeting.
- Revisit broader benchmark papers before designing a public or workspace-wide evaluation suite.
