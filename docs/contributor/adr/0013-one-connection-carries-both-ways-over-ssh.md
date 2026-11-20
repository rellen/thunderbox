# ADR 0013: One connection carries both ways over SSH

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 11 |
| Code | No file holds this decision. The project has not built either way. |
| Tests | No test holds this decision. |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0006](0006-use-ratatui-crossterm-and-tokio.md), [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0059](0059-version-1-may-need-no-server.md) |

## Context

Two of the four ways to run thunderbox would use two processes:
"client and server" and "through SSH".
[ADR 0004](0004-design-for-the-way-inside-another-program-first.md) names the
four ways. The code holds neither of these two ways today. The two processes
must speak to each other.

## Decision

This record proposes the design below, and no code holds it.

One connection will carry two kinds of message. The same SSH channel will
carry a query and its answer in the "client and server" way. It will carry the
characters of the terminal in the SSH way. Both ways will get their password
check and their encryption from SSH. The project will therefore write one
connection layer, and not two.

The project will put a length in front of each message, and it will encode the
message with `postcard`. `postcard` is a crate that writes a Rust value as a
short sequence of bytes.

For the SSH way, the project will follow `russh/examples/ratatui_app.rs`:

- That example writes to an object that implements `io::Write`. That object
  collects the bytes and sends them with `handle.data()`.
- The example sets `Viewport::Fixed` from `pty_request` and from
  `window_change_request`.
- The program will read the keys with `termwiz::input::InputParser`.
- The program will keep one host key in a file.
- The program will answer `shell_request`.
- The program will set `inactivity_timeout`.
- The program will limit how many people can connect at one time.

## Consequences

The project has built nothing of this record. No crate of the workspace holds
`russh`, `termwiz`, or `postcard`.

A pattern of the `legend` field belongs to the side that draws, so it will
never cross the boundary between the two processes.
[ADR 0012](0012-the-five-duties-of-a-data-source.md) states that rule.

## Designs that the project rejected

gRPC is a protocol that calls a function in another process, and `tonic` is a
Rust crate for it. gRPC through `tonic` is more machinery than a link between
two programs needs.

## The question that remains

The work that remains is the connection layer and the SSH server. That work
would give thunderbox the two ways that use two processes.
[ADR 0059](0059-version-1-may-need-no-server.md) asks whether version 1 needs
the "client and server" way at all.

The next design must answer this question: does one connection layer, with a
length in front of each message and `postcard` inside it, serve both ways?
