# Cert-Report Command

The `cert-report` command produces the Power-of-10 compliance evidence
package for an auditor. It aggregates rule pass/fail counts across every
file in the project, names every violation with its source location, and
emits machine-readable JSON for downstream tools — or human-readable text
for review.

fastC produces the evidence; the auditor produces the certification.

## Usage

```bash
fastc cert-report [OPTIONS] <INPUTS>...
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<INPUTS>...` | One or more `.fc` files or directories. Required. |

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--format <FORMAT>` | `json` | Output format: `json`, `text`, or `compact` |
| `-o, --output <PATH>` | `-` | Output file. `-` writes to stdout. |
| `--safety-level <LEVEL>` | `standard` | Rule strictness: `standard`, `critical`, or `relaxed` |
| `--project` | off | Aggregate every input into one project-wide report (vs. one report per file) |
| `--project-name <NAME>` | derived | Project name shown in the report header (project mode only) |
| `--fail-on-violation` | off | Exit 1 if any rule fails. The CI gate. |
| `-h, --help` |  | Print help |

## Output Formats

| Format | Audience |
|--------|----------|
| `json` | Auditor artifacts, agent tooling, machine pipelines |
| `text` | Humans reviewing a PR or local run |
| `compact` | Single-line JSON, one record per file. Easy to grep/jq. |

## JSON Schema

The JSON artifact is the canonical auditor deliverable. In project mode
(`--project`), the shape is:

```json
{
  "project": "fastc-rover",
  "safety_level": "standard",
  "files": [
    {
      "path": "src/control.fc",
      "rules": [
        {
          "id": "rule-1",
          "name": "Restrict to simple control flow",
          "passed": true,
          "violations": []
        },
        {
          "id": "rule-4",
          "name": "No function longer than 60 lines",
          "passed": false,
          "violations": [
            {
              "file": "src/control.fc",
              "span": { "start": 1421, "end": 1438 },
              "rule": "rule-4",
              "message": "function `dispatch` is 73 lines (limit: 60)"
            }
          ]
        }
      ]
    }
  ],
  "summary": {
    "total_rules": 10,
    "passed": 9,
    "failed": 1
  }
}
```

Without `--project`, the top-level object is an array of file reports
with the same `rules` / `violations` shape.

Every violation entry carries:

- `file` — absolute or project-relative source path
- `span` — byte offsets `{ start, end }` into the source file
- `rule` — the rule id (`rule-1` … `rule-10`)
- `message` — human-readable description suitable for a PR comment

## The Power-of-10 Rule Mapping

Each rule fastC checks corresponds to one of NASA/JPL's Power-of-10
safety-critical coding rules. Full descriptions, including which subset
runs at each safety level, live in
[../reference/power-of-10.md](../reference/power-of-10.md).

| ID | Rule |
|----|------|
| `rule-1` | Restrict to simple control flow constructs |
| `rule-2` | All loops must have a fixed upper bound |
| `rule-3` | No dynamic memory allocation after initialization |
| `rule-4` | No function longer than 60 lines |
| `rule-5` | At least two runtime assertions per function (modulated by safety level) |
| `rule-6` | Restrict the scope of data to the smallest possible |
| `rule-7` | Check the return value of every non-void function |
| `rule-8` | Limit preprocessor use to header guards and simple macros |
| `rule-9` | Restrict the use of pointers — one level of dereferencing, no function pointers |
| `rule-10` | Compile with all warnings enabled; treat warnings as errors |

The `--safety-level` flag tunes which rules run and how strictly:

| Level | Behaviour |
|-------|-----------|
| `relaxed` | P10 checks disabled. For exploratory code only. |
| `standard` | Default. All ten rules enforced; rule-5 requires one assertion per function. |
| `critical` | Strictest. Rule-5 requires two assertions per function; rule-4's 60-line limit is hard. |

## Examples

### Human Review

```bash
fastc cert-report src/*.fc --format=text
```

Walks every `.fc` file in `src/` and prints a per-file summary to stdout.
Use this in code review to see violations alongside source locations.

### Auditor Artifact

```bash
fastc cert-report src/ \
    --project \
    --project-name "fastc-rover-flight-control" \
    --format=json \
    --output=evidence/cert-report.json
```

Produces a single project-wide JSON report at the named path. This is
the file auditors ingest into their evidence tracker.

### CI Gate

```bash
fastc cert-report src/ \
    --project \
    --safety-level=critical \
    --fail-on-violation
```

Drop into the CI workflow alongside `fastc bench`. The build fails if
any rule fails at the `critical` safety level. JSON goes to stdout for
the runner to archive.

### Compact for Pipelines

```bash
fastc cert-report src/ --format=compact | jq 'select(.summary.failed > 0)'
```

`compact` emits one JSON record per line — ideal for streaming through
`jq`, ingesting into a log pipeline, or annotating a PR comment bot.

## Role in DO-178C / IEC 62304 / ISO 26262

`cert-report` is an evidence-production tool. It does not, and cannot,
issue certification. What it does:

- **Produces the artifact**: machine-readable JSON that an auditor can
  trace through a tool-qualification process.
- **Records the safety level**: the report carries the safety-level
  assumption it ran under, so the audit trail is self-describing.
- **Enables continuous evidence**: because the report runs on every PR,
  the evidence package is always current with the source tree.

What the auditor still owns:

- Tool qualification of the fastC compiler itself
- Mapping rule-by-rule outputs to the standard's objectives
- Independent review and sign-off

Cross-link: [../reference/certification.md](../reference/certification.md)
covers the full certification workflow, including which DO-178C
objectives the Power-of-10 ruleset covers and which still require
out-of-band evidence.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Report produced successfully; with `--fail-on-violation`, no rule failed |
| 1 | With `--fail-on-violation`, at least one rule failed |
| 2 | Input error (file not found, parse error in source) |

## See Also

- [Power-of-10 rules](../reference/power-of-10.md) — full description of
  every rule, with examples of pass and fail cases
- [Certification](../reference/certification.md) — the standards mapping
  and the role of fastC's evidence in DO-178C / IEC 62304 / ISO 26262
- [bench](bench.md) — the other CI gate, for compile-time performance
