export const SCENARIOS = Object.freeze([
  {
    id: "navigate",
    description: "Load the deterministic fixture and read its title.",
    capability: "navigation",
  },
  {
    id: "snapshot",
    description: "Produce supported accessibility evidence for the fixture controls.",
    capability: "interactiveSnapshot",
  },
  {
    id: "fill",
    description: "Fill a labelled text field and read its value.",
    capability: "fill",
  },
  {
    id: "click",
    description: "Click a link and confirm same-context navigation.",
    capability: "click",
  },
  {
    id: "evaluate",
    description: "Evaluate JavaScript against the loaded document.",
    capability: "javascript",
  },
  {
    id: "screenshot",
    description: "Capture the current viewport as PNG.",
    capability: "screenshot",
  },
  {
    id: "agent-loop",
    description: "Run snapshot, click, then snapshot again.",
    capability: "agentLoop",
  },
  {
    id: "full-workflow",
    description: "Navigate, inspect, fill, check, select, read, click, and read.",
    capability: "fullWorkflow",
  },
]);

export const SCENARIO_IDS = Object.freeze(SCENARIOS.map(({ id }) => id));
