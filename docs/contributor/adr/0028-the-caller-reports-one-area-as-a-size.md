# ADR 0028: The caller reports one area as a size

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox/src/main.rs`, `crates/thunderbox-dashboard/examples/full_embed.rs` |
| Tests | No test holds the shape of the method. The compiler holds it. |
| Related | [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0021](0021-a-new-area-waits.md), [ADR 0023](0023-each-panel-asks-for-the-points-of-its-own-width.md), [ADR 0029](0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md) |

## Context

The runtime needs the width of the area for the point limit of each panel, and
the height for the panels that the renderer drops. The method carried the name
`set_panel_width` and then the name `set_panel_area_width`.
[ADR 0023](0023-each-panel-asks-for-the-points-of-its-own-width.md) records
those names.

## Decision

The caller reports one area, in one method and one value.
`DashboardRuntime::set_panel_area` takes a `ratatui_core::layout::Size`. The
caller must report the area that it gives to `DashboardView::render`.

The runtime never measures the terminal, and the caller always reports the
area. The runtime also holds no rule of a layout of its own. It reads the
private module `grid` of `thunderbox-dashboard`, and the renderer reads the
same module.

## Consequences

Both programs of this repository report the area with one line. That line
reads the same value that the program gives to `DashboardView::render`.

`Rect` is already part of the public interface of this crate, because
`DashboardView::render` takes one. `Size` therefore adds no new dependency for
a caller.

The name of the method states the thing that a caller reports, which is the
area that the caller gives to `DashboardView::render`. Every future number
about that area still fits that name, so this is the last name of this method.

The rule "the runtime never measures" keeps the promise that this crate never
owns the terminal. [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md)
states that promise.

## Designs that the project rejected

- A method of two `u16` values lets a caller pass the height first, and the
  compiler reports nothing. A body of 110 cells by 31 cells would then reach
  the runtime as 31 cells wide by 110 cells high. Each panel would then ask for
  the points of a width that it does not draw.
- A second method beside `set_panel_area_width` lets a caller report the width
  alone. That caller keeps the old behaviour, and the compiler reports nothing.
- A `Rect` carries an `x` and a `y` that the runtime must ignore, and a caller
  cannot see that. A pane that moves without a change of size would also start
  a set of queries.
