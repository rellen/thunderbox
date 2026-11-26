# Architecture decision records

An architecture decision record (ADR) states one decision of this project. It
states the context, the decision, and the consequences of that decision. A
person who changes the code reads the record before the change. A person who
takes a new decision writes a new record.

ADR 0001 states why this project keeps these records. It also states the rules
of this folder.

## Status of a record

| status | meaning |
|---|---|
| Accepted | The project took the decision. The code follows it, unless the record states otherwise. |
| Proposed | The project has not taken a decision. The record states the question, the work, and the designs that the project examined. |
| Superseded | A later record replaced this decision. The record names the later record. |
| Rejected | The project examined the design and did not take it. The record states the fault that a reader measured in it. |

## Form of a record

Copy [the template](0000-template.md) for a new record. Give it the next free
number, and give it a name of lowercase words, joined by hyphens. Add a row to
the table below.

A record never changes its decision. A new decision takes a new record, and the
new record supersedes the old one. A record can gain a correction of a fact, a
new measurement, or a link to a later record.

## Records

The column `source` names the place that held the decision before this folder
existed. Each such record holds the text of that place, in the form of a record.
That text then left the place.

The file `docs/ARCHITECTURE.md` no longer exists. The commit
`4d8c223a0e7ea3bd62b85e7819be8c4ee10cb451` holds its last form. This command
writes that form:

```
git show 4d8c223a0e7ea3bd62b85e7819be8c4ee10cb451:docs/ARCHITECTURE.md
```

| number | title | status | source |
|---|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record each architecture decision | Accepted | new |
| [0002](0002-write-every-document-in-simplified-technical-english.md) | Write every document in Simplified Technical English | Accepted | `CLAUDE.md` section 1 |
| [0003](0003-agree-on-the-answer-and-not-on-the-question.md) | Agree on the form of the answer, and not on the form of the question | Accepted | `docs/ARCHITECTURE.md` section 2 |
| [0004](0004-design-for-the-way-inside-another-program-first.md) | Design for the way inside another program first | Accepted | `docs/ARCHITECTURE.md` section 1 |
| [0005](0005-one-trait-is-the-seam-of-every-data-source.md) | One trait is the seam of every data source | Accepted | `docs/ARCHITECTURE.md` section 3 |
| [0006](0006-use-ratatui-crossterm-and-tokio.md) | Use ratatui, crossterm, and tokio, and no other library of that kind | Accepted | `docs/ARCHITECTURE.md` section 4 |
| [0007](0007-the-settings-of-a-panel-win-over-the-column.md) | The settings of a panel win over the configuration of a column | Accepted | `docs/ARCHITECTURE.md` section 5 |
| [0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md) | Draw each chart through a wrapper of the project | Accepted, not built | `docs/ARCHITECTURE.md` section 5, and section 13 item 3 |
| [0009](0009-the-data-source-reduces-the-points.md) | The data source reduces the points | Accepted | `docs/ARCHITECTURE.md` section 6 |
| [0010](0010-the-program-that-runs-alone-is-a-dashboard-over-sysinfo.md) | The program that runs alone is a dashboard over sysinfo | Accepted | `docs/ARCHITECTURE.md` section 7 |
| [0011](0011-add-a-data-source-as-code-behind-a-feature.md) | Add a data source as code behind a feature | Accepted | `docs/ARCHITECTURE.md` section 10 |
| [0012](0012-the-five-duties-of-a-data-source.md) | The five duties of a data source | Accepted | `docs/ARCHITECTURE.md` section 10 |
| [0013](0013-one-connection-carries-both-ways-over-ssh.md) | One connection carries both ways over SSH | Proposed | `docs/ARCHITECTURE.md` section 11 |
| [0014](0014-the-file-format-of-a-dashboard-is-toml.md) | The file format of a dashboard is TOML | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0015](0015-both-ways-into-another-program-are-in-scope.md) | Both ways into another program are in scope | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) | A program that holds a dashboard holds four rules | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0017](0017-the-dashboard-applies-the-pattern-of-a-legend.md) | The dashboard applies the pattern of a legend | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0018](0018-a-refresh-keeps-the-answer-of-each-panel.md) | A refresh keeps the answer of each panel | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0019](0019-the-runtime-holds-one-query-to-a-limit.md) | The runtime holds one query to a limit | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0020](0020-a-task-that-ends-with-no-answer-gives-an-error.md) | A task that ends with no answer gives an error | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0021](0021-a-new-area-waits.md) | A new area waits, and the next set of queries carries it | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md) | An area that changes no point limit starts no set of queries | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0023](0023-each-panel-asks-for-the-points-of-its-own-width.md) | Each panel asks for the points of its own width | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) | A panel that the renderer drops asks for nothing | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0025](0025-the-rows-of-the-grid-belong-to-the-dashboard.md) | The rows of the grid belong to the dashboard | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0026](0026-panel-state-gains-no-fourth-variant.md) | `PanelState` gains no fourth variant | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0027](0027-a-dropped-panel-keeps-its-answer.md) | A panel that the renderer drops keeps its answer | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0028](0028-the-caller-reports-one-area-as-a-size.md) | The caller reports one area as a size | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0029](0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md) | The runtime asks about every panel until an area arrives | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md) | A set of queries holds one job for each panel | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0031](0031-a-snapshot-waits-in-a-third-case.md) | `Snapshot::is_loading` gains a third case | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0032](0032-each-rule-of-the-path-of-an-area-holds-a-test.md) | Each rule of the path of an area holds a test | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0033](0033-no-text-writes-the-password-of-an-address.md) | No text writes the password of an address | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0034](0034-a-dashboard-reads-more-than-one-data-source.md) | A dashboard reads more than one data source | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0035](0035-the-program-binds-each-key-of-the-time-range.md) | The program binds each key of the time range | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md) | Each label of the axis of time states its distance from the present moment | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0037](0037-the-refresh-continues-while-the-window-stands-still.md) | The refresh continues while the window stands still | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0038](0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md) | A sample of sysinfo carries the moment of the machine | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0039](0039-a-stack-of-charts-can-share-one-width.md) | A stack of charts can share one width | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0040](0040-a-reference-to-a-variable-reaches-the-query-alone.md) | A reference to a variable reaches the query alone | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0041](0041-validate-reports-the-fault-of-a-variable.md) | `Dashboard::validate` reports the fault of a variable | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md) | The runtime asks for the choices of a variable one time | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0043](0043-the-program-binds-each-key-of-a-variable.md) | The program binds each key of a variable | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0044](0044-the-row-of-variables-holds-short-labels.md) | The row of the variables holds short labels | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) | The panels of one set of queries run at the same time | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md) | A list in the form of `top` is a kind of panel of its own | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0047](0047-the-process-list-of-sysinfo-keeps-no-past.md) | The list of processes of sysinfo keeps no past | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0048](0048-the-process-list-reads-the-machine-through-its-own-system.md) | The list of processes reads the machine through a `System` of its own | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0049](0049-the-number-of-a-process-is-text.md) | The number of a process is text | Superseded by [0083](0083-a-column-can-hold-identifiers.md) | `docs/ARCHITECTURE.md` section 12 |
| [0050](0050-a-row-panel-holds-panels-by-their-place.md) | A row panel holds panels by their place on the grid | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0051](0051-a-row-panel-takes-one-line.md) | A row panel takes one line, and a collapse gives its rows to the other panels | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0052](0052-the-runtime-owns-the-collapse.md) | The runtime owns the collapse, and each snapshot carries it | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0053](0053-a-panel-that-appears-costs-a-set-of-queries.md) | A panel that appears costs a set of queries | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0054](0054-the-program-binds-enter-and-space.md) | The program binds `Enter` and `Space` | Accepted | `docs/ARCHITECTURE.md` section 12 |
| [0055](0055-build-the-package-with-a-nix-flake.md) | Build the package with a Nix flake | Accepted | `CLAUDE.md` section 4.4 |
| [0056](0056-the-check-phase-of-the-package-reads-a-ca-file.md) | The check phase of the package reads a CA file | Accepted | `CLAUDE.md` section 4.4 |
| [0057](0057-license-under-mit-or-apache.md) | License the project under MIT or Apache 2.0 | Accepted | `docs/ARCHITECTURE.md` section 13 item 5 |
| [0058](0058-the-public-interface-holds-ratatui-core.md) | The public interface holds `ratatui-core` | Proposed | `docs/ARCHITECTURE.md` section 13 item 1 |
| [0059](0059-version-1-may-need-no-server.md) | Version 1 may need no server | Proposed | `docs/ARCHITECTURE.md` section 13 item 2 |
| [0060](0060-a-height-that-uncovers-a-panel-asks-every-panel.md) | A height that uncovers a panel asks every panel | Proposed | `docs/ARCHITECTURE.md` section 13 item 4 |
| [0061](0061-a-duration-that-does-not-fit-ends-the-program.md) | A duration that does not fit ends the program | Superseded by [0073](0073-a-duration-that-does-not-fit-returns-an-error.md) | `docs/ARCHITECTURE.md` section 13 item 6 |
| [0062](0062-downsamples-promises-a-step-that-no-code-takes.md) | `SourceDescriptor::downsamples` promises a step that no code takes | Proposed | `docs/ARCHITECTURE.md` section 13 item 7 |
| [0063](0063-a-narrow-top-panel-drops-the-sort-column.md) | A narrow top panel drops the column that gives the order | Proposed | `docs/ARCHITECTURE.md` section 13 item 8 |
| [0064](0064-plain-http-needs-no-ca-file.md) | An address of plain HTTP needs no CA file | Proposed | `docs/ARCHITECTURE.md` section 13 item 9 |
| [0065](0065-no-panel-cuts-a-number.md) | No panel cuts a number | Accepted | new |
| [0066](0066-the-documentation-is-one-mdbook-site.md) | The documentation is one mdBook site, and the workflow `checks` publishes it | Accepted | new |
| [0067](0067-five-other-designs-for-the-site-failed.md) | Five other designs for the documentation site failed | Rejected | new |
| [0068](0068-the-site-shows-one-version.md) | The site shows one version | Proposed | new |
| [0069](0069-a-chart-cuts-its-line-at-each-hole.md) | A chart cuts its line at each hole | Accepted | new |
| [0070](0070-each-panel-draws-inside-the-intersection-of-its-area-and-the-buffer.md) | Each panel draws inside the intersection of its area and the buffer | Accepted | `docs/contributor/open-findings.md` item 2.6 |
| [0071](0071-the-program-draws-only-on-a-terminal.md) | The program draws only on a terminal | Accepted | `docs/contributor/open-findings.md` items 4.7 and 7.2 |
| [0072](0072-the-program-reads-its-command-line-from-left-to-right.md) | The program reads its command line from left to right | Accepted | `docs/contributor/open-findings.md` item 4.6 |
| [0073](0073-a-duration-that-does-not-fit-returns-an-error.md) | A duration that does not fit returns an error | Accepted | [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md), and `docs/contributor/open-findings.md` items 4.1 and 4.3 |
| [0074](0074-validate-checks-the-time-range.md) | `Dashboard::validate` checks the time range | Accepted | `docs/contributor/open-findings.md` item 4.3 |
| [0075](0075-no-path-of-the-library-panics-on-a-time-value.md) | No path of the library panics on a time value | Accepted | `docs/contributor/open-findings.md` items 4.1 and 4.3 |
| [0076](0076-a-key-that-would-pass-the-limit-leaves-the-range-as-it-was.md) | A key that would pass the limit leaves the range as it was | Accepted | `docs/contributor/open-findings.md` item 4.3 |
| [0077](0077-the-text-form-of-an-offset-keeps-a-unit-that-a-person-can-read.md) | The text form of an offset keeps a unit that a person can read | Accepted | new |
| [0078](0078-a-duration-that-saturates-hides-the-mistake.md) | A duration that saturates hides the mistake | Rejected | [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) |
| [0079](0079-a-key-that-clamps-an-end-changes-the-width.md) | A key that clamps an end changes the width | Rejected | new |
| [0080](0080-a-dashboard-states-a-refresh-above-0-and-a-known-unit.md) | A dashboard states a refresh above 0 and a known unit | Accepted | `docs/contributor/open-findings.md` items 4.4 and 5.3 |
| [0081](0081-the-runtime-returns-an-error-for-a-frame-of-two-lengths.md) | The runtime returns an error for a frame of two lengths | Accepted | `docs/contributor/open-findings.md` item 3.3 |
| [0082](0082-a-filled-chart-draws-each-line-over-every-fill.md) | A filled chart draws each line over every fill | Accepted | GitHub issue #15 (code review finding 48) |
| [0083](0083-a-column-can-hold-identifiers.md) | A column can hold identifiers | Accepted | [ADR 0049](0049-the-number-of-a-process-is-text.md) |
| [0084](0084-each-column-of-a-table-takes-an-equal-share-of-the-panel.md) | Each column of a table takes an equal share of the panel | Accepted | new |
| [0085](0085-a-panel-can-keep-a-column-of-numbers-at-the-left.md) | A panel can keep a column of numbers at the left | Accepted | new |
