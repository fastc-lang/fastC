# Certification & AI Integration

FastC provides built-in certification tooling designed for safety-critical development workflows and AI/Agent integration. This page covers generating compliance reports, integrating with CI/CD pipelines, and using FastC with AI coding assistants.

## Overview

The `cert-report` command generates machine-readable compliance reports that can be:

- Parsed by AI agents for automated code review
- Used in CI/CD pipelines for compliance gates
- Submitted as evidence for DO-178C and ISO 26262 certification audits
- Tracked over time for compliance metrics

## Quick Start

```bash
# Generate JSON report (default - best for AI agents)
fastc cert-report src/main.fc

# Generate human-readable text report
fastc cert-report src/main.fc --format text

# Generate compact JSON for CI/CD
fastc cert-report src/main.fc --format compact

# Fail if non-compliant (for CI/CD gates)
fastc cert-report src/main.fc --fail-on-violation
```

## Report Formats

### JSON Format (Default)

Best for AI agents and programmatic processing:

```bash
fastc cert-report src/main.fc --format json
```

Output structure:

```json
{
  "fastc_version": "0.1.0",
  "timestamp": "2026-02-19T10:30:00Z",
  "file": "src/main.fc",
  "safety_level": "Standard",
  "status": "compliant",
  "summary": {
    "rules_checked": 7,
    "rules_passed": 7,
    "rules_failed": 0,
    "total_violations": 0,
    "functions_analyzed": 5
  },
  "rules": [
    {
      "rule_number": 1,
      "name": "Simple Control Flow (no recursion)",
      "enabled": false,
      "passed": true,
      "violation_count": 0
    },
    // ... more rules
  ],
  "certification": {
    "standard": "NASA/JPL Power of 10 (Partial)",
    "applicable_rules": [
      "DO-178C Table A-5 - Code Standards (partial)",
      "ISO 26262-6:2018 Table 1 - Design principles (partial)"
    ],
    "tool_qualification": "TQL-5 (Advisory)",
    "analysis_method": "Static Analysis"
  }
}
```

### Compact JSON

Single-line JSON for parsing in shell scripts:

```bash
fastc cert-report src/main.fc --format compact
```

### Text Format

Human-readable report for auditors:

```bash
fastc cert-report src/main.fc --format text
```

```
╔══════════════════════════════════════════════════════════════╗
║              FASTC COMPLIANCE REPORT                         ║
╚══════════════════════════════════════════════════════════════╝

File:         src/main.fc
Safety Level: Standard
Status:       Compliant
...
```

## AI Agent Integration

### Prompting AI Assistants

When working with AI coding assistants, include the compliance report in your prompt:

```
I'm working on a safety-critical FastC project. Here's the current compliance report:

<report>
{paste JSON report here}
</report>

Please help me fix the Rule 2 violations (unbounded loops).
```

### Automated Review Workflow

AI agents can parse the JSON report to:

1. Identify specific violations by rule number
2. Access exact source locations (line, column, offset)
3. Read the suggested fixes in `help` and `note` fields
4. Track compliance status over time

Example Python script for AI agent integration:

```python
import json
import subprocess

# Generate compliance report
result = subprocess.run(
    ["fastc", "cert-report", "src/main.fc", "--format", "json"],
    capture_output=True,
    text=True
)
report = json.loads(result.stdout)

# Check compliance status
if report["status"] == "noncompliant":
    for rule in report["rules"]:
        if not rule["passed"]:
            for violation in rule.get("violations", []):
                print(f"Line {violation['location']['line']}: {violation['message']}")
                if violation.get("help"):
                    print(f"  Fix: {violation['help']}")
```

## CI/CD Integration

### GitHub Actions

```yaml
name: FastC Compliance Check

on: [push, pull_request]

jobs:
  compliance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install FastC
        run: cargo install --path crates/fastc

      - name: Check Compliance
        run: |
          fastc cert-report src/*.fc \
            --safety-level=critical \
            --format json \
            --output compliance-report.json \
            --fail-on-violation

      - name: Upload Report
        uses: actions/upload-artifact@v4
        with:
          name: compliance-report
          path: compliance-report.json
```

### GitLab CI

```yaml
compliance:
  stage: test
  script:
    - cargo install --path crates/fastc
    - fastc cert-report src/*.fc --safety-level=critical --fail-on-violation
  artifacts:
    reports:
      dotenv: compliance-report.json
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any
    stages {
        stage('Compliance') {
            steps {
                sh 'fastc cert-report src/*.fc --format json --output report.json'
                archiveArtifacts artifacts: 'report.json'
            }
        }
    }
    post {
        always {
            script {
                def report = readJSON file: 'report.json'
                if (report.status != 'compliant') {
                    error "Compliance check failed: ${report.summary.total_violations} violations"
                }
            }
        }
    }
}
```

## Project-Wide Reports

For multi-file projects, use the `--project` flag:

```bash
fastc cert-report src/*.fc --project --project-name="FlightController"
```

This generates an aggregated report:

```json
{
  "fastc_version": "0.1.0",
  "timestamp": "2026-02-19T10:30:00Z",
  "project_name": "FlightController",
  "safety_level": "Standard",
  "status": "compliant",
  "summary": {
    "files_analyzed": 10,
    "files_compliant": 10,
    "files_non_compliant": 0,
    "total_violations": 0,
    "total_functions": 47
  },
  "files": [
    // Individual file reports
  ]
}
```

## Safety Levels

| Level | Command | Use Case |
|-------|---------|----------|
| Standard | `--safety-level=standard` | General development (default) |
| Critical | `--safety-level=critical` | Safety-critical systems (all P10 rules) |
| Relaxed | `--safety-level=relaxed` | Prototyping (no P10 checks) |

```bash
# For safety-critical applications (aerospace, medical)
fastc cert-report src/*.fc --safety-level=critical

# For prototyping (skip compliance checks)
fastc cert-report src/*.fc --safety-level=relaxed
```

## Certification Standards

FastC compliance reports reference these certification standards:

### DO-178C (Aerospace)

- **Section 6.3.4** - Source Code
- **Table A-5** - Code Standards

### ISO 26262 (Automotive)

- **Part 6:2018 Table 1** - Design principles at software unit level

### MISRA C

FastC's Power of 10 rules overlap significantly with MISRA C:2012 guidelines.

## Tool Qualification

The compliance report includes tool qualification information:

```json
{
  "certification": {
    "tool_qualification": "TQL-5 (Advisory)",
    "analysis_method": "Static Analysis"
  }
}
```

**TQL-5** indicates the tool provides advisory information but does not make final compliance decisions. For full certification, combine FastC reports with:

- Manual code review
- Unit testing coverage
- Dynamic analysis (sanitizers, fuzzing)
- Formal verification (where applicable)

## Best Practices

### For AI-Assisted Development

1. **Generate reports before AI review** - Include compliance status in prompts
2. **Request specific rule fixes** - Use rule numbers to focus AI attention
3. **Verify AI fixes** - Re-run compliance check after AI modifications
4. **Track compliance trends** - Store reports for historical analysis

### For CI/CD Pipelines

1. **Use `--fail-on-violation`** - Prevent non-compliant code from merging
2. **Archive reports** - Keep compliance evidence for audits
3. **Use compact format** - Faster parsing in automated scripts
4. **Set appropriate safety level** - Critical for safety-critical, Standard for general

### For Certification Audits

1. **Generate project reports** - Use `--project` for full coverage
2. **Use text format** - Human-readable for auditor review
3. **Document safety level** - Explain why Standard vs Critical
4. **Combine with test reports** - Show coverage alongside compliance

## v1.0 Evidence Artifacts

In addition to `cert-report`, a v1.0 build emits three machine-readable artifacts alongside the generated C. An auditor with these three files can verify Power-of-10 compliance, capability-flow, and contract discharge *without re-running the compiler*. This is the cert-side surface FastC ships.

### `discharge.json` — per-build contract discharge report

Every `@requires` / `@ensures` clause in the program produces an obligation. Each obligation is resolved by one of three tiers — syntactic (always on), SMT (when built with `--prove`), or runtime trap — and the result is written to `discharge.json` at the build output root.

**Schema:**

```json
{
  "proven": 47,
  "runtime": 12,
  "unknown": 0,
  "obligations": [
    {
      "function": "checked_div",
      "clause": "requires",
      "index": 0,
      "status": "proven",
      "tier": "syntactic"
    },
    {
      "function": "checked_div",
      "clause": "ensures",
      "index": 0,
      "status": "proven",
      "tier": "smt"
    },
    {
      "function": "binary_search",
      "clause": "call_site",
      "index": 2,
      "status": "runtime"
    }
  ]
}
```

Field semantics:

- `function` — the FastC function the obligation belongs to
- `clause` — one of `requires`, `ensures`, or `call_site` (a discharge generated at the caller of a function whose pre-condition could not be proved statically)
- `index` — position of the clause within its function (or within the call site)
- `status` — one of `proven` (statically discharged), `runtime` (lowered to a runtime trap), or `unknown` (the SMT solver returned `unknown`; build fails in `--prove` mode)
- `tier` — present when `status == "proven"`; one of `syntactic` or `smt`

The discharge engine is body-aware: for straight-line returns of the form `return e;` the post-condition is checked directly against `e`, allowing many `@ensures` clauses to discharge at tier-1 without invoking the SMT solver. Call-site discharge fires for direct calls, method dispatch, and bound `fn`-pointers — every place a callee's pre-condition is visible to the caller.

See `crates/fastc/src/discharge/mod.rs` for the encoder. The artifact is regenerated on every build, with an on-disk cache keyed by obligation content so unchanged clauses are not re-solved. See [Contracts](../language/contracts.md) for the language-level surface.

### `caps.json` — per-build capability graph

Every function that accepts a capability handle via a `ref(...)` parameter is recorded in `caps.json`. This is the cap-flow record an auditor uses to verify that I/O, allocation, and external access are gated through the expected capability boundaries.

**Schema:**

```json
{
  "schema": "fastc.caps.v1",
  "functions": [
    {
      "name": "fetch_status",
      "module": "http",
      "caps": ["CapNet", "CapAlloc"]
    },
    {
      "name": "read_config",
      "module": "main",
      "caps": ["CapFsRead"]
    }
  ]
}
```

Field semantics:

- `schema` — pinned to `"fastc.caps.v1"`; consumers should reject unknown versions
- `functions[].name` — the FastC function name (post-mangling-free; this is the source-level identifier)
- `functions[].module` — owning module path
- `functions[].caps` — the list of `Cap*` types the function accepts as parameters (e.g. `CapAlloc`, `CapFsRead`, `CapNet`, `CapTime`, `CapRand`, `CapStdout`)

Because capabilities can only enter a function through a parameter — there is no global capability table — the union of `caps` across the graph is the complete I/O / allocation / external-access surface of the program. A function with empty `caps` is provably hermetic at this level.

See `crates/fastc/src/caps_summary.rs` for the emitter. See [Capabilities](../language/capabilities.md) for the language surface.

### Unified diagnostic envelope

Every diagnostic the compiler emits — parse error, type error, P10 violation, capability violation, contract violation, discharge failure, annotation violation — serializes through one shape. Certification pipelines that need to ingest all diagnostics uniformly can rely on this single shape; there is no per-rule format to special-case.

**Schema:**

```json
{
  "kind": "p10",
  "rule_id": "p10.rule_4",
  "severity": "error",
  "span": {
    "file": "src/control.fc",
    "start": 1284,
    "end": 1297
  },
  "message": "function 'compute_trajectory' exceeds 60-line limit (74 lines)",
  "hint": "decompose into focused helpers per Rule 4"
}
```

Field semantics:

- `kind` — coarse category: `parse`, `type`, `resolve`, `p10`, `capability`, `contract`, `discharge`, `annotation`
- `rule_id` — stable identifier (e.g. `p10.rule_4`, `cap.unauthorized`, `contract.requires_failed`, `discharge.unknown`)
- `severity` — `error`, `warning`, or `info`
- `span` — source location with `file` (path as the compiler saw it; basename under `--reproducible`), `start`, `end` byte offsets
- `message` — primary diagnostic text
- `hint` — optional remediation guidance

Emit this envelope by running any subcommand with `--diagnostics-format=json`. See `crates/fastc/src/diag/json.rs` for the canonical encoder.

### Putting the three together

For a DO-178C or ISO 26262 evidence package, ship the four files together:

| File | Verifies |
|------|----------|
| `cert-report.json` | Power-of-10 rule compliance |
| `discharge.json` | Contract clause discharge (proven vs runtime vs unknown) |
| `caps.json` | I/O / allocation / external-access surface (cap-flow) |
| Diagnostic stream (envelope) | All warnings and errors during the build, in one shape |

An auditor can then re-verify the program's safety surface without needing a FastC compiler — every property visible in these files is grounded in static analysis at build time.

## See Also

- [Power of 10 Rules](power-of-10.md) - Complete rule documentation
- [Safety Guarantees](safety.md) - Memory safety features
- [Contracts](../language/contracts.md) - `@requires` / `@ensures` and discharge tiers
- [Capabilities](../language/capabilities.md) - Cap types and `ref(...)` plumbing
- [CLI Reference](../cli/index.md) - All commands and options
