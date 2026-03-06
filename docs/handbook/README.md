# Orderflow Handbook

This handbook is the primary documentation for the project.

It is written for three audiences:

- Traders and analysts who need plain-language orderflow concepts.
- API users integrating C, Python, Java, or Rust.
- Contributors extending adapters, runtime logic, and bindings.

## Document Map

1. [What Orderflow Is](./01-orderflow-primer.md)  
   Conceptual model, footprint chart construction, and key terms.
2. [Building an Orderflow Strategy](./02-strategy-design.md)  
   How to turn concepts into repeatable, testable rules.
3. [Real Trade Workflow](./03-trade-workflow.md)  
   End-to-end flow from analysis to execution and review.
4. [Architecture](./04-architecture.md)  
   Components, data flow, UML-style diagrams, and module boundaries.
5. [API Reference](./05-api-reference.md)  
   Full Rust/C/Python/Java API coverage with payload contracts.
6. [Contributor Guide](./06-contributor-guide.md)  
   Build/test/extend instructions and implementation notes.
7. [References](./07-references.md)  
   Standards, platform docs, market microstructure references, and risk disclosures.

## Scope and Guardrails

- This software provides data processing, analytics, and signal infrastructure.
- It does not provide financial advice.
- Strategy examples are educational and must be validated with risk controls before live usage.

## Static Diagram Exports

For platforms that do not render Mermaid, static exports are available in:

- `docs/handbook/assets/diagrams/svg/`
- `docs/handbook/assets/diagrams/png/`
- Mermaid sources used for export: `docs/handbook/assets/diagrams/src/`
