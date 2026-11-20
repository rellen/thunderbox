# ADR 0059: Version 1 may need no server

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 2 |
| Code | No crate of the workspace is `thunderbox-server`. |
| Tests | No test holds this decision. |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md) |

## Context

[ADR 0004](0004-design-for-the-way-inside-another-program-first.md) names four
ways to run thunderbox. Two of them use two processes: "client and server" and
"through SSH". `thunderbox-server` is the name of the planned server of the
first of those two ways. The project has not built it. The project has not
built the SSH way either, and
[ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md) states the plan
for it.

## Decision

The project has not taken a decision. The SSH way, together with data sources
that run inside the process of thunderbox, may cover every need.

## Consequences

Under the rule of
[ADR 0004](0004-design-for-the-way-inside-another-program-first.md), the
project builds the way "client and server" only when a need appears that the
SSH way cannot meet. No work on `thunderbox-server` starts before such a need
appears.

## The question that remains

The work that remains is a search for a need that the SSH way cannot meet. A
version 1 with no such need holds no server.

The next design answers one question. Does version 1 need `thunderbox-server`?
