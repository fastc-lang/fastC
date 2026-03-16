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

## See Also

- [Power of 10 Rules](power-of-10.md) - Complete rule documentation
- [Safety Guarantees](safety.md) - Memory safety features
- [CLI Reference](../cli/index.md) - All commands and options
