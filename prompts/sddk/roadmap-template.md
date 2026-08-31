# SDDK Milestone Template

SDDK roadmaps are represented by milestone nodes under `{vault}/milestones/`.
Resolve `{vault}` with `sddk knowledge path`; never create or update a roadmap
inside an adopted workspace.

Read `{vault}/templates/milestone.md` and include:

- stable milestone ID and title;
- status, dates, owner, and stale-after date;
- goals, non-goals, success criteria, and risks;
- linked requirements, ADRs, cycles, and incidences using wikilinks;
- append-only bi-temporal changelog.

The active serialization state is `{vault}/milestones/_active.md`. Release
updates the milestone and cycle nodes, then releases that lock. Existing
product roadmaps are read-only evidence and are never SDDK authority.

F3 tuning and metrics are operational state under the project XDG/state
directories. Engram may mirror reusable jurisprudence only when enabled by the
knowledge profile.
