# Tessera documentation

**Default language: English.** Portuguese alternatives: [`pt/`](pt/).

## Use the product

- [Device and Agent setup](guides/device-setup.md)
- [Google Calendar setup](guides/calendar-setup.md)
- [Where to change settings](guides/settings-surfaces.md)

## Contribute

| Doc | Audience |
|---|---|
| [Getting started](getting-started.md) | New clone — macOS + Linux toolchain |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | PR and finish-gate expectations |
| [../README.md](../README.md) | Project pitch + quick start |

## Guides

| Doc | Topic |
|---|---|
| [guides/architecture.md](guides/architecture.md) | Layers + widget contract |
| [guides/widget-authoring.md](guides/widget-authoring.md) | New widget step-by-step |
| [guides/simulator.md](guides/simulator.md) | Host harness (real sources, not a demo) |
| [guides/finish-gate.md](guides/finish-gate.md) | `test` → `e2e` → `flash` → `device` |
| [STRUCTURE.md](STRUCTURE.md) | Repository tree |

## ADRs and feature notes

- [`adr/`](adr/) — decisions for this repo
- [`superpowers/`](superpowers/) — dated feature specs/plans (working notes)

## Agents

Harness summary for coding agents: [`../AGENTS.md`](../AGENTS.md).

## Verification and maintenance

- [Product journey decisions](guides/product-consolidation.md)
- [Automated user journeys](guides/testing.md)
- [Repository asset policy](guides/repository-assets.md)

Dated files under `superpowers/` are design history, not a live task board. Confirm current behavior in code, tests and the guides above before treating an unchecked historical plan as missing work.
