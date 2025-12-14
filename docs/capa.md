# TDT CAPA Entity (Corrective and Preventive Action)

This document describes the CAPA entity type in TDT (Tessera Engineering Toolkit).

## Overview

CAPAs address root causes of quality issues and prevent recurrence. Corrective actions fix existing problems; preventive actions stop potential problems. CAPAs track investigation, actions, and effectiveness verification.

## Entity Type

- **Prefix**: `CAPA`
- **File extension**: `.tdt.yaml`
- **Directory**: `manufacturing/capas/`

## Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (CAPA-[26-char ULID]) |
| `title` | string | Short descriptive title (1-200 chars) |
| `status` | enum | `draft`, `review`, `approved`, `released`, `obsolete` |
| `created` | datetime | Creation timestamp (ISO 8601) |
| `author` | string | Author name |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `capa_number` | string | Company CAPA number (e.g., "CAPA-2024-0015") |
| `capa_type` | enum | `corrective`, `preventive` |
| `source` | Source | What triggered the CAPA |
| `problem_statement` | string | Description of the problem |
| `root_cause_analysis` | RootCauseAnalysis | RCA details |
| `actions` | array[ActionItem] | Action items |
| `effectiveness` | Effectiveness | Verification of effectiveness |
| `closure` | Closure | Closure details |
| `timeline` | Timeline | Key dates |
| `capa_status` | enum | CAPA workflow status |
| `tags` | array[string] | Tags for filtering |
| `entity_revision` | integer | Entity revision number (default: 1) |

### CAPA Types

| Type | Description |
|------|-------------|
| `corrective` | Fix an existing problem and prevent recurrence |
| `preventive` | Prevent a potential problem before it occurs |

### Source Types

| Type | Description |
|------|-------------|
| `ncr` | Non-conformance report |
| `audit` | Internal or external audit finding |
| `customer_complaint` | Customer-reported issue |
| `trend_analysis` | Statistical trend identified |
| `risk` | Risk assessment finding |

### RCA Methods

| Method | Description |
|--------|-------------|
| `five_why` | 5 Whys analysis |
| `fishbone` | Ishikawa/fishbone diagram |
| `fault_tree` | Fault tree analysis |
| `eight_d` | 8D problem solving |

### CAPA Status

| Status | Description |
|--------|-------------|
| `initiation` | CAPA opened, gathering information |
| `investigation` | Root cause analysis in progress |
| `implementation` | Actions being implemented |
| `verification` | Verifying effectiveness |
| `closed` | CAPA verified and closed |

### Action Status

| Status | Description |
|--------|-------------|
| `open` | Not started |
| `in_progress` | Work in progress |
| `completed` | Action completed, awaiting verification |
| `verified` | Verified effective |

### Source Object

| Field | Type | Description |
|-------|------|-------------|
| `type` | enum | `ncr`, `audit`, `customer_complaint`, `trend_analysis`, `risk` |
| `reference` | string | Reference ID (e.g., NCR ID) |

### RootCauseAnalysis Object

| Field | Type | Description |
|-------|------|-------------|
| `method` | enum | `five_why`, `fishbone`, `fault_tree`, `eight_d` |
| `root_cause` | string | Identified root cause |
| `contributing_factors` | array[string] | Contributing factors |

### ActionItem Object

| Field | Type | Description |
|-------|------|-------------|
| `action_number` | integer | Action item number |
| `description` | string | Action description |
| `action_type` | enum | `corrective`, `preventive` |
| `owner` | string | Person responsible |
| `due_date` | date | Target completion date |
| `completed_date` | date | Actual completion date |
| `status` | enum | `open`, `in_progress`, `completed`, `verified` |
| `evidence` | string | Evidence of completion |

### Effectiveness Object

| Field | Type | Description |
|-------|------|-------------|
| `verified` | boolean | Whether effectiveness was verified |
| `verified_date` | date | Date of verification |
| `result` | enum | `effective`, `partially_effective`, `ineffective` |
| `evidence` | string | Evidence of effectiveness |

### Closure Object

| Field | Type | Description |
|-------|------|-------------|
| `closed` | boolean | Whether CAPA is closed |
| `closed_date` | date | Date closed |
| `closed_by` | string | Person who closed |

### Timeline Object

| Field | Type | Description |
|-------|------|-------------|
| `initiated_date` | date | Date CAPA was initiated |
| `target_date` | date | Target completion date |

### Links

| Field | Type | Description |
|-------|------|-------------|
| `links.ncrs` | array[EntityId] | Source NCRs |
| `links.risks` | array[EntityId] | Related risks |
| `links.processes_modified` | array[EntityId] | Processes updated |
| `links.controls_added` | array[EntityId] | New controls added |

## Example

```yaml
id: CAPA-01KC5B6P6PSHZ6TMCSDJQQ6HG3
title: "Tool Wear Detection Improvement"
capa_number: "CAPA-2024-0015"

capa_type: corrective

source:
  type: ncr
  reference: NCR-01KC5B6E1RKCPKGACCH569FX5R

problem_statement: |
  Multiple NCRs for bore diameter out of tolerance due to undetected
  tool wear. Three incidents in January 2024 resulting in 15 scrapped
  parts and $2,500 in rework costs.

root_cause_analysis:
  method: five_why
  root_cause: |
    Lack of systematic tool life monitoring and no automatic
    compensation for tool wear in CNC program.
  contributing_factors:
    - "No tool life tracking in CNC controller"
    - "Operators rely on visual inspection"
    - "Insufficient in-process inspection frequency"
    - "No SPC charting for bore diameter"

actions:
  - action_number: 1
    description: "Implement tool life management in CNC controller"
    action_type: corrective
    owner: "Manufacturing Engineering"
    due_date: 2024-02-15
    completed_date: 2024-02-10
    status: completed
    evidence: "CNC program updated, tool life set to 50 parts"

  - action_number: 2
    description: "Add in-process SPC for bore diameter"
    action_type: corrective
    owner: "Quality Engineering"
    due_date: 2024-02-28
    completed_date: 2024-02-25
    status: completed
    evidence: "CTRL-01KC5B5M87QMYVJT048X27TJ5S created"

  - action_number: 3
    description: "Train operators on tool wear indicators"
    action_type: preventive
    owner: "Production Supervisor"
    due_date: 2024-03-15
    completed_date: 2024-03-10
    status: verified
    evidence: "Training records on file, 12 operators trained"

  - action_number: 4
    description: "Update work instruction with tool inspection step"
    action_type: preventive
    owner: "Manufacturing Engineering"
    due_date: 2024-03-15
    completed_date: 2024-03-12
    status: completed
    evidence: "WORK-01KC5B5XKGWKFTTA9YWTGJB9GE updated"

effectiveness:
  verified: true
  verified_date: 2024-04-15
  result: effective
  evidence: |
    No bore diameter NCRs in 60 days since implementation.
    SPC charts show Cpk improved from 0.8 to 1.45.
    Tool life data confirms predictable wear pattern.

closure:
  closed: true
  closed_date: 2024-04-20
  closed_by: "Quality Manager"

timeline:
  initiated_date: 2024-01-25
  target_date: 2024-03-31

capa_status: closed

tags: [tool-wear, spc, machining, bore]
status: approved

links:
  ncrs:
    - NCR-01KC5B6E1RKCPKGACCH569FX5R
  risks: []
  processes_modified:
    - PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
  controls_added:
    - CTRL-01KC5B5M87QMYVJT048X27TJ5S

created: 2024-01-25T09:00:00Z
author: Quality Manager
entity_revision: 3
```

## CLI Commands

### Create a new CAPA

```bash
# Create with default template
tdt capa new

# Create with title and type
tdt capa new --title "Tool Wear Fix" --type corrective

# Create linked to an NCR
tdt capa new --title "Address Bore NCR" --type corrective --ncr NCR@1

# Interactive wizard
tdt capa new -i

# Create and immediately edit
tdt capa new --title "New CAPA" --edit
```

### List CAPAs

```bash
# List all CAPAs
tdt capa list

# Filter by type
tdt capa list --type corrective
tdt capa list --type preventive

# Filter by CAPA status
tdt capa list --capa-status open
tdt capa list --capa-status implementation
tdt capa list --capa-status closed

# Show overdue CAPAs
tdt capa list --overdue

# Search in title/description
tdt capa list --search "tool"

# Output formats
tdt capa list -f json
tdt capa list -f csv
tdt capa list -f md
```

### Show CAPA details

```bash
# Show by ID
tdt capa show CAPA-01KC5

# Show using short ID
tdt capa show CAPA@1

# Output as JSON
tdt capa show CAPA@1 -f json
```

### Edit a CAPA

```bash
# Open in editor
tdt capa edit CAPA-01KC5

# Using short ID
tdt capa edit CAPA@1
```

### Verify CAPA effectiveness

```bash
# Record effectiveness verification result
tdt capa verify CAPA@1 --result effective

# Verify as partially effective
tdt capa verify CAPA@1 --result partial

# Verify as ineffective (requires further action)
tdt capa verify CAPA@1 --result ineffective

# Include evidence
tdt capa verify CAPA@1 --result effective --evidence "No recurrence in 60 days, Cpk improved"

# Skip confirmation prompt
tdt capa verify CAPA@1 --result effective -y
```

**Example Output:**

```
Verifying CAPA Effectiveness
──────────────────────────────────────────────────
CAPA: CAPA@2 "Tool Wear Detection Improvement"
Current Status: Implementation

Recording verification result: Effective
Evidence: No bore diameter NCRs in 60 days

Note: CAPA will be closed automatically (result = effective)

Continue? [y/N] y

✓ CAPA@2 verified as Effective
  Evidence: No bore diameter NCRs in 60 days
  Status: Closed
```

**Note:** If the verification result is `effective`, the CAPA status is automatically set to `closed`. For `partial` or `ineffective` results, the status is set to `verification` for further action.

## CAPA Workflow

```
┌────────────┐     ┌───────────────┐     ┌────────────────┐     ┌──────────────┐     ┌────────┐
│ INITIATION │────▶│ INVESTIGATION │────▶│ IMPLEMENTATION │────▶│ VERIFICATION │────▶│ CLOSED │
└────────────┘     └───────────────┘     └────────────────┘     └──────────────┘     └────────┘
      │                   │                      │                      │
      │                   │                      │                      │
      ▼                   ▼                      ▼                      ▼
   Open CAPA,        Root cause           Execute actions,        Verify
   define problem    analysis             track completion        effectiveness
```

## Root Cause Analysis Methods

### 5 Whys

Simple technique asking "why" repeatedly:

```
Problem: Bore diameter out of tolerance
Why 1: Tool was worn
Why 2: Tool life exceeded
Why 3: No tool life tracking
Why 4: Feature not enabled in CNC
Why 5: Not included in setup procedure
Root Cause: Setup procedure missing tool life configuration
```

### Fishbone (Ishikawa)

Categorize causes by:
- **Man** - People, training, skill
- **Machine** - Equipment, maintenance
- **Method** - Procedures, processes
- **Material** - Raw materials, consumables
- **Measurement** - Inspection, calibration
- **Environment** - Conditions, contamination

### 8D Problem Solving

1. **D1** - Team formation
2. **D2** - Problem description
3. **D3** - Containment actions
4. **D4** - Root cause analysis
5. **D5** - Corrective actions
6. **D6** - Implementation
7. **D7** - Preventive actions
8. **D8** - Congratulate the team

## Best Practices

### Effective CAPAs

1. **Clear problem statement** - Define scope and impact
2. **True root cause** - Dig deep, don't stop at symptoms
3. **Measurable actions** - SMART goals
4. **Assigned ownership** - Single point of accountability
5. **Verification period** - Allow time to prove effectiveness

### Common Pitfalls

| Pitfall | Better Approach |
|---------|-----------------|
| Fixing symptoms | Use 5 Whys to find root cause |
| Too many actions | Focus on vital few |
| No verification | Define success metrics upfront |
| Blame individuals | Focus on system/process |
| Never closing | Set target dates, verify, close |

### Effectiveness Verification

Verify effectiveness by:
- No recurrence for defined period (30-90 days)
- Process capability improvement (Cpk)
- Reduced defect rate
- Positive trend in quality metrics
- Successful audits

### When to Escalate

Escalate CAPA when:
- Actions consistently missed
- Root cause not identified
- Problem recurs after closure
- Cross-functional barriers
- Resource constraints

## Corrective vs Preventive

| Aspect | Corrective | Preventive |
|--------|------------|------------|
| **Trigger** | Existing problem | Potential problem |
| **Goal** | Eliminate and prevent recurrence | Prevent occurrence |
| **Source** | NCR, complaint, audit finding | Risk assessment, trend analysis |
| **Timing** | Reactive | Proactive |

## Validation

```bash
# Validate all project files
tdt validate

# Validate specific file
tdt validate manufacturing/capas/CAPA-01KC5B6P6PSHZ6TMCSDJQQ6HG3.tdt.yaml
```
