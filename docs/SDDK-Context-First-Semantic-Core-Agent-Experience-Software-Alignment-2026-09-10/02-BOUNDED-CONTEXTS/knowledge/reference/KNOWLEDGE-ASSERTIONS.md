# Knowledge Assertions

Assertions are the granular progressive memory of what is known about software.

Examples:

```text
file:planner.rs responsibility = workflow-planning [INFERRED]
module:domain dependency->storage = true [OBSERVED]
context:decision owner-of Decision = true [DECLARED]
tradeoff:DEC-81 assumes graph_nodes < 2M [DECIDED]
```

They are append/supersede, not mutable rows. Projection queries materialize the current view.
