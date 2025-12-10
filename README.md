# PDT - Plain-text Product Development Toolkit

A CLI tool for managing product development artifacts as plain-text YAML files. PDT provides structured tracking of requirements, risks, tests, and other entities with full traceability and validation.

## Features

- **Plain-text YAML files** - Human-readable, git-friendly, diff-able
- **Schema validation** - JSON Schema validation with helpful error messages
- **Traceability** - Link entities together and generate traceability matrices
- **ULID-based IDs** - Unique, sortable identifiers for all entities
- **Beautiful error messages** - Line numbers, context, and actionable suggestions

## Installation

```bash
cargo install pdt
```

Or build from source:

```bash
git clone https://github.com/yourorg/pdt.git
cd pdt
cargo build --release
```

## Quick Start

```bash
# Initialize a new project
pdt init

# Create a requirement
pdt req new --title "Operating Temperature Range" --type input

# List all requirements
pdt req list

# Show a specific requirement (partial ID match)
pdt req show REQ-01HC2

# Validate all project files
pdt validate
```

## Project Structure

After `pdt init`, your project will have:

```
.pdt/
└── config.yaml              # Project configuration

requirements/
├── inputs/                  # Design inputs (customer requirements)
└── outputs/                 # Design outputs (specifications)

risks/
├── design/                  # Design risks
└── process/                 # Process risks

bom/
├── assemblies/              # Assembly definitions
├── components/              # Component definitions
└── quotes/                  # Supplier quotes

tolerances/
├── features/                # Feature tolerances
├── mates/                   # Mating features
└── stackups/                # Tolerance stackups

verification/
├── protocols/               # Verification test protocols
└── results/                 # Test results

validation/
├── protocols/               # Validation protocols
└── results/                 # Validation results

manufacturing/
├── processes/               # Manufacturing processes
└── controls/                # Process controls
```

## Entity Types

| Prefix | Entity | Description |
|--------|--------|-------------|
| REQ | Requirement | Design inputs and outputs |
| RISK | Risk | Design and process risks |
| TEST | Test | Verification test cases |
| CAPA | CAPA | Corrective and preventive actions |
| DOC | Document | Controlled documents |
| CHG | Change | Change requests |

## Commands

### Project Management

```bash
pdt init                    # Initialize a new project
pdt init --git              # Initialize with git repository
pdt validate                # Validate all project files
pdt validate --keep-going   # Continue after errors
pdt validate --summary      # Show summary only
```

### Requirements

```bash
pdt req new                           # Create with template
pdt req new --title "Title" -t input  # Create with options
pdt req new -i                        # Interactive wizard
pdt req list                          # List all
pdt req list --status draft           # Filter by status
pdt req list --priority high          # Filter by priority
pdt req list --type input             # Filter by type
pdt req list --search "temperature"   # Search in title/text
pdt req list --orphans                # Show unlinked requirements
pdt req show REQ-01HC2                # Show details (partial ID match)
pdt req edit REQ-01HC2                # Open in editor
```

### Link Management

```bash
pdt link add REQ-01 --type satisfied_by REQ-02    # Add link
pdt link remove REQ-01 --type satisfied_by REQ-02 # Remove link
pdt link show REQ-01                               # Show all links
pdt link check                                     # Check for broken links
```

### Traceability

```bash
pdt trace matrix                  # Show traceability matrix
pdt trace matrix --output csv     # Export as CSV
pdt trace matrix --output dot     # Export as GraphViz DOT
pdt trace from REQ-01             # What depends on this?
pdt trace to REQ-01               # What does this depend on?
pdt trace orphans                 # Find unlinked entities
pdt trace coverage                # Verification coverage report
pdt trace coverage --uncovered    # Show uncovered requirements
```

## Requirement Example

```yaml
id: REQ-01HC2JB7SMQX7RS1Y0GFKBHPTD
type: input
title: "Operating Temperature Range"

source:
  document: "Customer Requirements Spec"
  revision: "A"
  section: "3.2.1"
  date: 2024-01-15

category: "Environmental"
tags: [thermal, environmental, reliability]

text: |
  The device shall operate continuously in ambient temperatures
  from -20C to +50C without degradation of performance.

rationale: |
  Required for outdoor deployment in various climates.

acceptance_criteria:
  - "Unit powers on at -20C after 4h cold soak"
  - "Unit powers on at +50C after 4h hot soak"

priority: high
status: approved

links:
  satisfied_by:
    - REQ-01HC2JB7SMQX7RS1Y0GFKBHPTE
  verified_by:
    - TEST-01HC2JB7SMQX7RS1Y0GFKBHPTF

created: 2024-01-15T10:30:00Z
author: Jane Doe
revision: 1
```

## Validation

PDT validates files against JSON Schema with detailed error messages:

```
error[pdt::schema::validation]: Schema validation failed
  --> requirements/inputs/REQ-01HC2.pdt.yaml:8:1
   |
 8 | status: pending
   | ^^^^^^^^^^^^^^^ Invalid enum value
   |
  help: Valid values: draft, review, approved, obsolete
```

## Status Workflow

```
draft → review → approved
                    ↓
                 obsolete
```

| Status | Description |
|--------|-------------|
| draft | Initial creation, still being written |
| review | Ready for stakeholder review |
| approved | Signed off and baselined |
| obsolete | No longer applicable |

## Priority Levels

| Priority | Use For |
|----------|---------|
| critical | Safety, regulatory, blocking requirements |
| high | Core functionality, key differentiators |
| medium | Standard features, quality of life |
| low | Nice to have, future considerations |

## Best Practices

### Writing Requirements

- Use **"shall"** for mandatory requirements
- Use **"should"** for recommended requirements
- Use **"may"** for optional requirements
- Be specific and testable
- One requirement per file

### Organizing Requirements

- Use **categories** to group related requirements
- Use **tags** for cross-cutting concerns
- Separate **inputs** from **outputs** in different directories
- Link related requirements with `satisfied_by` relationships

## License

MIT License - See LICENSE file for details.
