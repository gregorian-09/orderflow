# Orderflow Documentation

Orderflow is a developer library for building deterministic market-data,
analytics, signal, order-management, and execution systems. It is not merely
a collection of indicators and it is not a broker terminal. It gives an
application explicit boundaries for observing a market, deciding what to do,
submitting an order, recording what happened, and recovering when the process
or venue becomes uncertain.

The documentation is written as a connected knowledge system. It starts with
the domain model, follows data through the market-data and execution planes,
then descends into exact APIs, language bindings, persistence formats, and
operational procedures. Start with [How to Read the Documentation](./handbook/00-how-to-read.md)
if you are new to the project.

The rendered portal is configured in the repository root
[`mkdocs.yml`](https://github.com/gregorian-09/orderflow/blob/main/mkdocs.yml)
and is suitable for local preview and Read the Docs hosting.

The documentation is intended for both:

- Non-coders who need to understand orderflow concepts and trading workflow.
- Engineers and contributors who need exact API and architecture details.

The layout follows production-oriented documentation principles:

- a reader learning what the system means;
- an engineer implementing a correct integration;
- an operator recovering and supervising a live deployment;
- a contributor changing a public contract without breaking users.

Each important subject is explained in four connected layers: meaning,
contract, worked use, and design limits. The generated reference indexes then
make every public declaration searchable without pretending that an inventory
alone teaches the system.

## Start Here

- [How to Read the Documentation](./handbook/00-how-to-read.md)
- [What Orderflow Is](./handbook/01-orderflow-primer.md)
- [Domain Foundations](./foundations/README.md)
- [Release 0.5.0 Notes](./ops/release-0.5.0.md)
- [Release 0.4.0 Notes](./ops/release-0.4.0.md)
- [Release 0.3.0 Notes](./ops/release-0.3.0.md)
- [Handbook Home](./handbook/README.md)
- [Binding Docs](./bindings/README.md)
- [API Docs (legacy entry)](./api/README.md)
- [Ops Docs](./ops/README.md)
- [Security Docs](./security/README.md)

## Recommended Reading Order

1. [How to Read the Documentation](./handbook/00-how-to-read.md)
2. [What Orderflow Is](./handbook/01-orderflow-primer.md)
3. [Domain Foundations](./foundations/README.md)
4. [System Architecture](./handbook/04-architecture.md)
5. [Real Trade Workflow](./handbook/03-trade-workflow.md)
6. [Building an Orderflow Strategy](./handbook/02-strategy-design.md)
7. [OMS Architecture](./handbook/09-oms-architecture.md)
8. [OMS Cookbook](./handbook/10-oms-cookbook.md)
9. [Low-Latency Design](./handbook/11-low-latency-design.md)
10. [Persistence and Replay](./persistence/README.md)
11. [Provider Adapter Authoring](./handbook/12-provider-adapter-authoring.md)
12. [Recovery and Operations](./handbook/13-recovery-and-operations.md)
13. [Complete API Reference](./handbook/05-api-reference.md)
14. [Language Bindings](./bindings/end-to-end.md)
15. [Contributor Guide](./handbook/06-contributor-guide.md)
