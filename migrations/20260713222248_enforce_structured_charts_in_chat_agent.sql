-- 20260713222248_enforce_structured_charts_in_chat_agent.sql
-- Force the chat agent to emit a structured `charts` entry whenever
-- the user asks for a chart (pie, bar, line, etc). The pie-chart
-- conversation in the wild showed the model describing a chart in
-- prose ("🍪 (A: 4), 🧡 (B: 8), ❤️ (C: 12)") with `charts: []` empty
-- because the strict-return-shape instructions didn't say anything
-- about the charts array.
--
-- This migration updates the chat supervisor's instructions (which the
-- agents table mirrors via UPDATE agents SET instructions = SELECT
-- FROM supervisors). It adds an explicit CHART RENDERING rule that
-- triggers when the user's query names a chart type. The model
-- already has the data needed to populate labels/data (the user's
-- numbers / categories); we just make sure it routes that into the
-- structured field instead of prose.

UPDATE supervisors
SET instructions = instructions || '

═══ CHART RENDERING (mandatory) ═══
When the user asks for a chart (pie / bar / line / column / scatter /
donut / etc.) or says "draw", "plot", "graph", "visualize":

- You MUST populate the `charts` array with a single ChartPart object:
    { "chart_type": "<pie|bar|line|...>",
      "title": "<short chart title>",
      "labels": ["<label1>", "<label2>", ...],
      "data":   [<number1>, <number2>, ...] }
- `chart_type` MUST be one of: bar, line, pie. Pick bar for categorical
  comparisons, pie for part-of-whole, line for trends over an axis.
- `labels` and `data` MUST be parallel arrays of equal length.
- Do NOT describe the chart in prose (e.g. "🍪 A: 4, 🧡 B: 8"). The
  renderer does not draw text-as-chart; an empty `charts` array means
  the user sees nothing.
- If the user did not provide enough data to compute the chart, ask
  one clarifying question in messages[].message AND leave `charts`
  empty.
- The assistant `messages[].message` may carry a one-line caption
  ("A pie chart of A=4, B=8, C=12") but the chart itself lives in
  the `charts` field.'
WHERE domain = 'chat';

UPDATE agents
SET instructions = (SELECT instructions FROM supervisors WHERE domain = 'chat')
WHERE domain = 'chat';
