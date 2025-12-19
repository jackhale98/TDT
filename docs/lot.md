# TDT LOT Entity (Production Lot / Device History Record)

This document describes the LOT entity type in TDT (Tessera Design Toolkit).

## Overview

LOTs track production batches through manufacturing, serving as Device History Records (DHR) for medical device and regulated manufacturing. Each LOT captures what's being made, materials used, process execution steps, and quality records - providing full traceability from raw materials to finished goods.

## Entity Type

- **Prefix**: `LOT`
- **File extension**: `.tdt.yaml`
- **Directory**: `manufacturing/lots/`

## Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (LOT-[26-char ULID]) |
| `title` | string | Short descriptive title (1-200 chars) |
| `status` | enum | `draft`, `review`, `approved`, `released`, `obsolete` |
| `created` | datetime | Creation timestamp (ISO 8601) |
| `author` | string | Author name |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `lot_number` | string | User-defined lot identifier (e.g., "2024-001") |
| `quantity` | integer | Number of units in this lot |
| `lot_status` | enum | Lot workflow status (see below) |
| `start_date` | date | Production start date |
| `completion_date` | date | Production completion date |
| `materials_used` | array[MaterialUsed] | Materials consumed (traceability) |
| `execution` | array[ExecutionStep] | Process execution records |
| `notes` | string | Markdown notes |
| `tags` | array[string] | Tags for filtering |
| `entity_revision` | integer | Entity revision number (default: 1) |

### Lot Status

| Status | Description |
|--------|-------------|
| `in_progress` | Production ongoing |
| `on_hold` | Paused (quality hold, waiting for material) |
| `completed` | Production finished successfully |
| `scrapped` | Lot scrapped/rejected |

### MaterialUsed Object

| Field | Type | Description |
|-------|------|-------------|
| `component` | EntityId | Component entity used |
| `supplier_lot` | string | Supplier lot/batch number (free text) |
| `quantity` | integer | Quantity consumed |

### ExecutionStep Object

| Field | Type | Description |
|-------|------|-------------|
| `process` | EntityId | Process entity being executed |
| `execution_status` | enum | `pending`, `in_progress`, `completed`, `skipped` |
| `completed_date` | date | When step was completed |
| `operator` | string | Person who performed the step |
| `notes` | string | Execution notes |
| `data` | object | Optional measurement/process data |

### Execution Status

| Status | Description |
|--------|-------------|
| `pending` | Step not yet started |
| `in_progress` | Currently executing |
| `completed` | Step finished successfully |
| `skipped` | Step skipped (with justification in notes) |

### Links

| Field | Type | Description |
|-------|------|-------------|
| `links.product` | EntityId | Product being made (ASM or CMP) |
| `links.processes` | array[EntityId] | Linked PROC entities in sequence |
| `links.work_instructions` | array[EntityId] | Linked WORK entities |
| `links.ncrs` | array[EntityId] | NCRs raised during production |
| `links.results` | array[EntityId] | In-process inspection results |

## Example

```yaml
id: LOT-01KC5B6E1RKCPKGACCH569FX5R
title: "Production Lot 2024-001"
lot_number: "2024-001"
quantity: 25

lot_status: in_progress
start_date: 2024-01-15
completion_date: ~

materials_used:
  - component: CMP-01HC2JB7SMQX7RS1Y0GFKBHPTD
    supplier_lot: "SUP-ABC-123"
    quantity: 25
  - component: CMP-01HC2JB8XYZQ7RS1Y0GFKBHPTE
    supplier_lot: "SUP-DEF-456"
    quantity: 50

execution:
  - process: PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
    execution_status: completed
    completed_date: 2024-01-15
    operator: "J. Smith"
    notes: "No issues"
    data: {}
  - process: PROC-01KC5B2HDDQ0JAXFVXYYZ9DWEA
    execution_status: in_progress
    operator: "M. Jones"
    notes: ""

notes: |
  # Production Notes
  - Material received 2024-01-14
  - First article inspection passed

tags: [production, 2024-q1]
status: approved

links:
  product: ASM-01HC2JB7SMQX7RS1Y0GFKBHPTD
  processes:
    - PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
    - PROC-01KC5B2HDDQ0JAXFVXYYZ9DWEA
  work_instructions:
    - WORK-01KC5B3PDDQ0JAXFVXYYZ9DWEB
  ncrs: []
  results:
    - RSLT-01KC5B4RDDQ0JAXFVXYYZ9DWEC

created: 2024-01-15T10:00:00Z
author: J. Smith
entity_revision: 1
```

## CLI Commands

### Create a new LOT

```bash
# Create with default template
tdt lot new

# Create with title and lot number
tdt lot new --title "Production Lot 2024-001" --lot-number "2024-001"

# Create with product and quantity
tdt lot new --title "Widget Batch" --product ASM@1 --quantity 100

# Create and immediately edit
tdt lot new --title "New Lot" --edit

# Non-interactive (skip editor)
tdt lot new --title "Batch 2024-002" --no-edit
```

### List LOTs

```bash
# List all lots
tdt lot list

# Filter by lot status
tdt lot list --lot-status in_progress
tdt lot list --lot-status completed
tdt lot list --lot-status on_hold

# Filter by product
tdt lot list --product ASM@1

# Search in title
tdt lot list --search "2024"

# Output formats
tdt lot list -f json
tdt lot list -f csv
tdt lot list -f md
```

### Show LOT details

```bash
# Show by ID
tdt lot show LOT-01KC5

# Show using short ID
tdt lot show LOT@1

# Output as JSON
tdt lot show LOT@1 -f json
```

### Edit a LOT

```bash
# Open in editor
tdt lot edit LOT-01KC5

# Using short ID
tdt lot edit LOT@1
```

### Update execution step

```bash
# Mark a step as completed
tdt lot step LOT@1 --process PROC@1 --status completed

# Add operator and notes
tdt lot step LOT@1 --process PROC@2 --status completed --operator "J. Smith" --notes "Passed inspection"

# Mark step as skipped
tdt lot step LOT@1 --process PROC@3 --status skipped --notes "Not required for this configuration"
```

### Complete a LOT

```bash
# Mark lot as completed
tdt lot complete LOT@1

# Complete with notes
tdt lot complete LOT@1 --notes "All inspections passed"

# Skip confirmation
tdt lot complete LOT@1 -y
```

### Delete or archive a LOT

```bash
# Permanently delete (checks for incoming links first)
tdt lot delete LOT@1

# Force delete even if referenced
tdt lot delete LOT@1 --force

# Archive instead of delete (moves to .tdt/archive/)
tdt lot archive LOT@1
```

## LOT Workflow

```
┌─────────────┐     ┌─────────┐     ┌───────────┐     ┌──────────┐
│ IN_PROGRESS │────▶│ ON_HOLD │────▶│ COMPLETED │     │ SCRAPPED │
└─────────────┘     └─────────┘     └───────────┘     └──────────┘
       │                 │                                  ▲
       │                 │                                  │
       └─────────────────┴──────────────────────────────────┘
                      (if quality issue)
```

### Production Flow

1. **Create LOT** - `tdt lot new --product ASM@1 --quantity 25`
2. **Record materials** - Add `materials_used` with supplier lot numbers
3. **Execute steps** - `tdt lot step LOT@1 --process PROC@1 --status completed`
4. **Record inspections** - Link RSLT entities for in-process checks
5. **Handle issues** - Create NCRs if problems found
6. **Complete** - `tdt lot complete LOT@1`

## Git-Based Traceability

For regulated environments (FDA 21 CFR 820, ISO 13485), use git for audit trail:

```bash
# Branch per lot
git checkout -b lot/2024-001

# Create lot record
tdt lot new --title "Lot 2024-001" --product ASM@1 --quantity 25
git commit -m "LOT@1: Started production"

# Each step = commit
tdt lot step LOT@1 --process PROC@1 --status completed --operator "J.Smith"
git commit -m "LOT@1: Completed OP-010"

# NCR during production
tdt ncr new --lot LOT@1 --title "Dimensional issue"
git commit -m "LOT@1: NCR opened for dimensional issue"

# Complete and merge
tdt lot complete LOT@1
git commit -m "LOT@1: Production complete"
git checkout main && git merge lot/2024-001
```

Git provides:
- **Immutable history** - Audit trail cannot be altered
- **Author + timestamp** - Operator records for each change
- **Signed commits** - Electronic signatures (21 CFR Part 11)
- **Pull request review** - Approval workflow

## Best Practices

### Material Traceability

1. **Record supplier lots** - Always capture `supplier_lot` for traceability
2. **Link components** - Use entity references for components consumed
3. **Quantity tracking** - Track quantities used per material

### Execution Records

1. **One step at a time** - Update steps as they're completed
2. **Capture operator** - Record who performed each step
3. **Add notes** - Document any deviations or observations
4. **Link work instructions** - Reference WORK entities for procedures

### Quality Integration

1. **Link NCRs** - Create NCRs for any non-conformances
2. **Record inspections** - Link RSLT entities for test results
3. **Hold when needed** - Use `on_hold` status for quality holds
4. **Complete carefully** - Verify all steps before completing

### DHR Requirements

For Device History Records:
- Dates of manufacture
- Quantity manufactured
- Quantity released for distribution
- Acceptance records (RSLT links)
- Primary identification label/control number
- Any device identifiers (UDI)

## Validation

```bash
# Validate all project files
tdt validate

# Validate specific file
tdt validate manufacturing/lots/LOT-01KC5B6E1RKCPKGACCH569FX5R.tdt.yaml
```
