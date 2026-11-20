# ADR 0001: Record each architecture decision

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | new |
| Code | `docs/contributor/adr` |
| Tests | No test reads a record. |
| Related | [ADR 0002](0002-write-every-document-in-simplified-technical-english.md) |

## Context

Before this folder existed, `docs/ARCHITECTURE.md` held the decisions of the
project. Section 12 of that file held each decision that the project took, in
one list. Section 13 held each decision that remained open. Section 12 alone
held more than 1200 lines.

An item of that list named another item by its title and by its place. The
text read "the item ... above", or "an earlier item of this section". A reader
then searched one long file for that item.

The description of a pull request often holds the reason that a design
failed. Section 6.2 of `CLAUDE.md` therefore asked a writer to copy that reason
into section 13 of `docs/ARCHITECTURE.md`. A person who clones this repository cannot read the
description of a pull request. A reason that stays in a description therefore
lives outside the project.

## Decision

The project records each architecture decision in a file of its own, in the
folder `docs/contributor/adr`. An architecture decision record (ADR) states one
decision. It states the context, the decision, and the consequences of that
decision.

The folder follows these rules:

1. One record holds one decision.
2. Each record copies the form of [the template](0000-template.md). The title
   line states the decision in one sentence. The table of fields names the
   status, the date, the source, the code, the tests, and the related records.
3. A record never changes its decision. A new decision takes a new record, and
   the new record supersedes the old record.
4. A record can gain a correction of a fact, a new measurement, or a link to a
   later record.
5. Each record holds one of four statuses: Accepted, Proposed, Superseded, or
   Rejected. [The index](README.md) states what each status means.
6. A record of the status Proposed keeps the section "The question that
   remains".
7. A new record takes the next free number, and a row in the table of
   [the index](README.md).
8. Every record follows [ADR 0002](0002-write-every-document-in-simplified-technical-english.md).
   Section 1 of [`CLAUDE.md`](../../../CLAUDE.md) governs each sentence of a
   record.
9. A record names another record with a link to the file of that record.

A design that the project examined and rejected takes a record of the status
Rejected, in the same pull request as the work. A question that the change
leaves open takes a record of the status Proposed. That record names three
things:

- the work that remains, and what it is worth;
- each design that the project rejected, with the fault that a reader measured
  in it;
- the question that the next design must answer.

A record of the status Accepted names each design that the project did not
take, in its section "Designs that the project rejected". The description of
the pull request can then point at the record.

## Consequences

A reader finds a decision by its number and by its title in
[the index](README.md). A link to a file replaces each reference to the place
of an item in a long list.

This rule replaces the earlier rule of section 6.2 of `CLAUDE.md`. Under that
earlier rule, a writer put a rejected design into section 13 of
`docs/ARCHITECTURE.md`. Section 6.2 now states the rule of this record.

The column `source` of the index names the place that held each decision
before this folder existed. The text of that place moved into the record.

A person who changes the code reads the records of the part that the change
alters, before the change. A change that alters a decision takes a new record,
and that record supersedes the old one.
