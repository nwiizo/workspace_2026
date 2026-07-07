# Agentic Technology Map

Date: 2026-07-06
Scope: Codex, Claude Code, MCP, A2A, and the local tool/workspace implications.

This is a dated synthesis. Use it to align local repos and agent workflows; use
the linked official sources for exact command flags and protocol details.

## One-Screen Map

| Layer | Use it for | Local repos | Current action |
|---|---|---|---|
| Codex subagents | Parallel bounded exploration, review, test/log triage, and worker tasks | `ccswarm`, `dotfiles`, `claudelytics` | Model provider capabilities; do not assume every provider has the same subagent surface |
| Claude Code subagents | Project/user/plugin-scoped specialist agents and session-only CLI-defined agents | `dotfiles`, project `.claude/agents/`, `hatena-blog-pull` | Normalize reusable agents in `dotfiles`; keep project agents local only when domain-specific |
| Telemetry streams | Progress events, usage, item-level traces, stuck-run diagnosis | `ccswarm`, `claudelytics` | Keep separate from schema-constrained final output |
| Schema-constrained output | Machine-readable verdicts, reports, routing decisions | `ccswarm`, evaluation tools | Add explicit capability probes before depending on it |
| MCP | Tool and external-context access | `tfmcp`, `remote-mcp-devkit`, `dotfiles` | Treat Remote MCP + OAuth as infrastructure, not one-off connector glue |
| A2A | Communication between independent agent services | `ccswarm`, `workspace_2026` experiments | Use for remote agents; do not confuse it with MCP tool calling |
| Agent config infrastructure | Skills, rules, custom agents, hooks, MCP config | `dotfiles` | Keep shared config in `.agents/`; audit symlinks before publishing |

## Codex

Current shape:

- Codex supports subagent workflows: the parent run can spawn specialized agents
  in parallel and collect their results.
- Codex only spawns subagents when explicitly asked to do so.
- Custom agents can be configured separately from the main session.
- `codex exec` is the non-interactive path for scripts and CI.
- `codex exec --json` is an event stream; `--output-schema` is for a structured
  final answer.
- `codex app-server` is a local JSON-RPC interface for rich clients, approvals,
  history, threads/turns, and streamed events.

Local implications:

- `ccswarm` should keep ccswarm-orchestrated fan-out as the deterministic
  baseline and add provider-native delegation only behind capability checks.
- `claudelytics` should treat Codex JSONL as telemetry, not as report schema.
- `dotfiles` should keep Codex custom agents in the shared `.agents` source
  tree when they are reusable.

Primary sources:

- https://developers.openai.com/codex/subagents
- https://developers.openai.com/codex/noninteractive
- https://developers.openai.com/codex/app-server
- https://developers.openai.com/codex/cli/reference
- https://developers.openai.com/codex/sdk

## Claude Code

Current shape:

- Claude Code supports project, user, plugin, managed, and CLI-defined
  subagents.
- Project subagents live under `.claude/agents/`; user-level subagents live
  under `~/.claude/agents/`; plugins can provide agents.
- CLI-defined subagents are useful for one session and quick automation tests.
- `claude -p` supports output formats including JSON and stream JSON; structured
  output is exposed through JSON schema options in current docs.
- Hooks, status lines, settings schemas, and SDKs make Claude Code configuration
  part of the development environment.

Local implications:

- Keep reusable Claude subagents and skills centralized in `dotfiles/.agents/`.
- Keep project-specific agents local only when they depend on repo context
  such as a manuscript voice, BJJ taxonomy, or a translation policy.
- In `ccswarm`, do not document Claude flags from memory; keep README/CLAUDE
  aligned with the provider code and current CLI docs.

Primary sources:

- https://docs.anthropic.com/en/docs/claude-code/sub-agents
- https://docs.anthropic.com/en/docs/claude-code/cli-reference
- https://docs.anthropic.com/en/docs/claude-code/hooks
- https://docs.anthropic.com/en/docs/claude-code/sdk

## MCP

Current shape:

- MCP is the tool/context protocol: hosts connect models to external tools,
  resources, prompts, and data.
- The current spec page identifies MCP as the authoritative protocol for
  integrating LLM applications with external data sources and tools.
- HTTP authorization is part of the transport-level story; the 2025-11-25
  authorization page requires OAuth protected-resource metadata for restricted
  MCP servers.

Local implications:

- `tfmcp` is the Terraform MCP server line. Keep dangerous Terraform operations
  gated by explicit environment variables and document Claude Desktop/Codex
  integration separately from core tool behavior.
- `remote-mcp-devkit` is the conformance harness line. It should stay focused on
  Remote MCP + OAuth 2.1 validation: protected-resource metadata, 401
  challenge, authorization metadata, token exchange, and fake-client dance.
- `dotfiles` should document which MCP servers are personal defaults, which are
  required, and which are project-specific.

Primary sources:

- https://modelcontextprotocol.io/specification/2025-11-25
- https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization
- https://modelcontextprotocol.io/specification/2025-11-25/changelog

## A2A

Current shape:

- A2A is the agent-to-agent protocol for independent agent systems, including
  systems built with different frameworks or vendors.
- Its specification focuses on capability discovery, interaction modality
  negotiation, and collaborative task management.
- Google codelab material positions A2A as complementary to MCP: MCP for tools,
  A2A for agents.

Local implications:

- In `ccswarm`, A2A should mean remote agent execution and task/message
  interoperability, not local CLI subprocess execution.
- Do not claim automatic discovery unless the code actually reads Agent Cards
  from `.well-known/agent-card.json`, or from another endpoint explicitly
  configured by that implementation.
- For design docs, keep A2A objects (Agent Card, Message, Task, Artifact)
  distinct from MCP tools/resources/prompts.

Primary sources:

- https://github.com/a2aproject/A2A/blob/main/docs/specification.md
- https://github.com/a2aproject/A2A/blob/main/README.md
- https://codelabs.developers.google.com/intro-a2a-purchasing-concierge

Note: the A2A GitHub links above are live upstream references, not
commit-pinned snapshot URLs.

## Design Rules For Local Repos

1. Capability-probe before using provider-native features.
2. Keep telemetry streams and schema-constrained reports separate.
3. Use subagents for bounded parallel work; give write-heavy agents explicit
   ownership.
4. Use MCP for tools and data; use A2A for remote agents.
5. Keep reusable agent assets in `dotfiles`, with project-local overrides only
   for domain-specific behavior.
6. Treat generated state (`target*`, `__pycache__`, local task queues, session
   output) as cleanup debt unless a repo explicitly tracks it.

## Repository Alignment

| Repository | Status | Alignment | Repo-local evidence |
|---|---|---|---|
| `ccswarm` | active implementation | Provider capability model, telemetry heartbeats, A2A/local execution separation, schema-constrained verdicts | `crates/ccswarm/src/session/a2a.rs`, `crates/ccswarm/src/session/bridge.rs`, `docs/ARCHITECTURE.md` |
| `dotfiles` | canonical config | Source of reusable `.agents`, `.claude`, `.codex`, MCP, and editor-agent configuration | `.agents/README.md`, `.claude/README.md`, `.codex/README.md`, `AGENTS.md` |
| `claudelytics` | active tool | Usage analytics across Claude/Codex streams and model pricing/report views | `src/parser.rs`, `src/codex_usage.rs`, `src/pricing.rs`, `src/reports.rs` |
| `tfmcp` | active MCP tool | Terraform MCP server and Claude Desktop/Codex integration surface | `Cargo.toml`, `README.md`, `src/main.rs` |
| `remote-mcp-devkit` | likely canonical | Remote MCP + OAuth 2.1 conformance harness | `README.md`, `docs/development-spec.md`, `src/client_dance.rs` |
| `workspace_2026/tools/remote-mcp-devkit` | to reconcile | Workspace-local mirror or experiment around the same harness | untracked workspace copy under `tools/` |
| `hatena-blog-pull` | domain-specific agents | Reusable writing-review agent patterns, separate from technical tooling | repo-local agent/rule docs |
| AI-agent translation repos | knowledge intake | Agent and MCP pattern intake; validate book imports independently | per-book `CODEX_TRANSLATION_BRIEF.md`, `content/`, `scripts/` |

## Open Questions

- Which `ccswarm` worktrees are still active, and which are stale agent
  scratch?
- Should `remote-mcp-devkit` live as a root repo, under `workspace_2026/tools/`,
  or both with one acting as canonical?
- Which project-local `.agents` and `.claude/agents` should graduate into
  `dotfiles/.agents/`?
- Should repeated translation scripts become a shared package or remain copied
  per book for isolation?
- Which local task state is intentionally tracked (`vibe-ticket`) versus local
  runtime output?
