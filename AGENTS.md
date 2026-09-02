Read `README.md`, then `goal.md`. The coverage table in `README.md` is the work list.

Before starting work, run `npx frog list` to review known repository friction.

- Log repository papercuts and friction with `npx frog log` when they occur.
- Include tooling, documentation, API, test, and convention friction.
- Do not log global, system, or internal friction.

Before changing architecture, behavior, or domain logic, use `lat search` or
`lat locate` to read the relevant sections in `lat.md/`. Update the graph when
those concepts change, and run `lat check` before finishing.
