# ADR-030-STATIC-CONTROL-PLANE — Make the primary Cockpit a static self-contained projection

**Status:** Accepted


## Decision
Generate `cockpit.html` from local persisted projections without requiring a web server. Assets and data are embedded; no CDN or remote fetch is required.

## Commands
`build`, `open`, and optional `watch` that regenerates the file atomically.

## Consequences
Zero deployment burden, portable reports and local-first privacy. A server can be added later as an optional host for real-time collaboration, not as a prerequisite.
