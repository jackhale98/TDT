# PDT - Plain-text Product Development Toolkit

A CLI tool for managing product development artifacts as plain-text YAML files. PDT provides structured tracking of requirements, risks, tests, and other entities with full traceability and validation.

## Features

- **Plain-text YAML files** - Human-readable, git-friendly, diff-able
- **Schema validation** - JSON Schema validation with helpful error messages
- **Traceability** - Link entities together and generate traceability matrices
- **ULID-based IDs** - Unique, sortable identifiers for all entities
- **Short ID aliases** - Use `REQ@1`, `RISK@2`, etc. instead of typing long IDs
- **Beautiful error messages** - Line numbers, context, and actionable suggestions
- **FMEA Risk Management** - Built-in support for Failure Mode and Effects Analysis
- **BOM Management** - Components and assemblies with supplier tracking
- **Tolerance Analysis** - Features, mates, and stackups with worst-case, RSS, and Monte Carlo analysis

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

# List all requirements (shows REQ@N short IDs)
pdt req list

# Show a specific requirement using short ID
pdt req show REQ@1                 # Use prefixed short ID from list
pdt req show REQ-01HC2             # Or partial ID match

# Create a risk
pdt risk new --title "Battery Overheating" -t design

# Validate all project files
pdt validate
```

## Short IDs

After running `list` commands, PDT assigns entity-prefixed short IDs (`REQ@1`, `RISK@1`, etc.) to entities:

```bash
$ pdt req list
@       ID               TYPE     TITLE                                STATUS     PRIORITY
--------------------------------------------------------------------------------------------
REQ@1   REQ-01HC2JB7...  input    Operating Temperature Range          approved   high
REQ@2   REQ-01HC2JB8...  output   Thermal Management Specification     draft      high

$ pdt risk list
@        ID                TYPE      TITLE                            STATUS     LEVEL    RPN
----------------------------------------------------------------------------------------------
RISK@1   RISK-01HC2JB7...  design    Battery Overheating              review     medium   108

# Use prefixed short IDs instead of full IDs
pdt req show REQ@1
pdt risk show RISK@1
pdt link add REQ@1 --type verified_by TEST@1
pdt trace from REQ@1
```

Short IDs are persistent per entity type - the same entity keeps its short ID across list commands.
This enables cross-entity linking (e.g., linking `REQ@1` to `TEST@1`).

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
├── processes/               # Manufacturing process definitions
├── controls/                # Control plan items (SPC, inspection)
├── work_instructions/       # Operator procedures
├── ncrs/                    # Non-conformance reports
└── capas/                   # Corrective/preventive actions
```

## Entity Types

| Prefix | Entity | Description |
|--------|--------|-------------|
| REQ | Requirement | Design inputs and outputs |
| RISK | Risk | Risk / FMEA item |
| TEST | Test | Verification or validation protocol |
| RSLT | Result | Test result / execution record |
| TOL | Tolerance | Tolerance stackup |
| MATE | Mate | Feature mate (for stackups) |
| ASM | Assembly | Assembly definition |
| CMP | Component | Component definition |
| FEAT | Feature | Feature (on a component) |
| PROC | Process | Manufacturing process definition |
| CTRL | Control | Control plan item (SPC, inspection) |
| WORK | Work Instruction | Operator procedures |
| NCR | Non-Conformance | Non-conformance report |
| CAPA | CAPA | Corrective/preventive action |
| QUOT | Quote | Quote / cost record |
| SUP | Supplier | Approved supplier |

## Output Formats

Use `-f/--format` to control output format:

```bash
pdt req list -f json        # JSON output (for scripting)
pdt req list -f yaml        # YAML output
pdt req list -f csv         # CSV output (for spreadsheets)
pdt req list -f tsv         # Tab-separated (default for lists)
pdt req list -f md          # Markdown table
pdt req list -f id          # Just IDs, one per line

pdt req show REQ-01 -f json # Full entity as JSON
pdt req show REQ-01 -f yaml # Full entity as YAML
```

## Commands

### Project Management

```bash
pdt init                    # Initialize a new project
pdt init --git              # Initialize with git repository
pdt validate                # Validate all project files
pdt validate --keep-going   # Continue after errors
pdt validate --summary      # Show summary only
pdt validate --fix          # Auto-fix calculated values (RPN, risk level)
pdt validate --strict       # Treat warnings as errors
```

### Requirements

```bash
pdt req new                           # Create with template
pdt req new --title "Title" -t input  # Create with options
pdt req new -i                        # Interactive wizard (schema-driven)
pdt req list                          # List all
pdt req list --status draft           # Filter by status
pdt req list --priority high          # Filter by priority
pdt req list --type input             # Filter by type
pdt req list --search "temperature"   # Search in title/text
pdt req list --orphans                # Show unlinked requirements
pdt req show REQ-01HC2                # Show details (partial ID match)
pdt req edit REQ-01HC2                # Open in editor
```

### Risks (FMEA)

```bash
pdt risk new                           # Create with template
pdt risk new --title "Overheating"     # Create with title
pdt risk new -t process                # Create process risk
pdt risk new --severity 8 --occurrence 5 --detection 3  # Set FMEA ratings
pdt risk new -i                        # Interactive wizard
pdt risk list                          # List all risks
pdt risk list --level high             # Filter by risk level
pdt risk list --by-rpn                 # Sort by RPN (highest first)
pdt risk list --min-rpn 100            # Filter by minimum RPN
pdt risk list --unmitigated            # Show risks without mitigations
pdt risk show RISK-01HC2               # Show details
pdt risk edit RISK-01HC2               # Open in editor
```

### Tests (Verification/Validation)

```bash
pdt test new                                  # Create with template
pdt test new --title "Temperature Test"       # Create with title
pdt test new -t verification -l system        # Create verification test at system level
pdt test new -m analysis                      # Create with analysis method (IADT)
pdt test new -i                               # Interactive wizard
pdt test list                                 # List all tests
pdt test list --type verification             # Filter by test type
pdt test list --level unit                    # Filter by test level
pdt test list --method inspection             # Filter by IADT method
pdt test list --orphans                       # Show tests without linked requirements
pdt test show TEST-01HC2                      # Show details
pdt test edit TEST-01HC2                      # Open in editor
```

### Test Results

```bash
pdt rslt new --test TEST-01HC2                # Create result for a test
pdt rslt new --test @1 --verdict pass         # Use short ID, set verdict
pdt rslt new -i                               # Interactive wizard
pdt rslt list                                 # List all results
pdt rslt list --verdict fail                  # Filter by verdict
pdt rslt list --verdict issues                # Show fail/conditional/incomplete
pdt rslt list --test TEST-01HC2               # Show results for a specific test
pdt rslt list --with-failures                 # Show only results with failures
pdt rslt list --recent 7                      # Show results from last 7 days
pdt rslt show RSLT-01HC2                      # Show details
pdt rslt edit RSLT-01HC2                      # Open in editor
```

### Components (BOM)

```bash
pdt cmp new                                   # Create with template
pdt cmp new --title "Motor Assembly" --part-number "PN-001"
pdt cmp new --make-buy buy --category mechanical
pdt cmp list                                  # List all components
pdt cmp list --make-buy buy                   # Filter by make/buy
pdt cmp list --category electrical            # Filter by category
pdt cmp show CMP@1                            # Show details
pdt cmp edit CMP@1                            # Open in editor
```

### Assemblies (BOM)

```bash
pdt asm new                                   # Create with template
pdt asm new --title "Main Assembly" --part-number "ASM-001"
pdt asm list                                  # List all assemblies
pdt asm show ASM@1                            # Show details
pdt asm bom ASM@1                             # Show flattened BOM
pdt asm edit ASM@1                            # Open in editor
```

### Suppliers (Approved Vendors)

```bash
pdt sup new --name "Acme Manufacturing Corp"  # Create supplier
pdt sup new -n "Acme Corp" --short-name "Acme" --website "https://acme.com"
pdt sup new -i                                # Interactive mode
pdt sup list                                  # List all suppliers
pdt sup list -c machining                     # Filter by capability
pdt sup list --search "acme"                  # Search in name
pdt sup show SUP@1                            # Show details
pdt sup edit SUP@1                            # Open in editor
```

### Quotes (Supplier Quotations)

```bash
pdt quote new --component CMP@1 --supplier SUP@1        # Quote for component
pdt quote new --assembly ASM@1 --supplier SUP@1         # Quote for assembly
pdt quote new -c CMP@1 -s SUP@1 --price 12.50 --lead-time 14
pdt quote new -i                              # Interactive mode
pdt quote list                                # List all quotes
pdt quote list -Q pending                     # Filter by quote status
pdt quote list --component CMP@1              # Filter by component
pdt quote list --supplier SUP@1               # Filter by supplier
pdt quote show QUOT@1                         # Show details
pdt quote compare CMP@1                       # Compare quotes for item
pdt quote edit QUOT@1                         # Open in editor
```

### Features (Tolerances)

```bash
pdt feat new --component CMP@1 --type hole --title "Mounting Hole"
pdt feat new --component CMP@1 --type shaft   # Feature requires parent component
pdt feat list                                 # List all features
pdt feat list --component CMP@1               # Filter by component
pdt feat list --type hole                     # Filter by type
pdt feat show FEAT@1                          # Show details
pdt feat edit FEAT@1                          # Open in editor
```

### Mates (Tolerances)

```bash
pdt mate new --feature-a FEAT@1 --feature-b FEAT@2 --title "Pin-Hole Fit"
pdt mate list                                 # List all mates
pdt mate list --type clearance_fit            # Filter by mate type
pdt mate show MATE@1                          # Show details with fit calculation
pdt mate recalc MATE@1                        # Recalculate fit from features
pdt mate edit MATE@1                          # Open in editor
```

### Stackups (Tolerance Analysis)

```bash
pdt tol new --title "Gap Analysis" --target-nominal 1.0 --target-upper 1.5 --target-lower 0.5
pdt tol list                                  # List all stackups
pdt tol list --result pass                    # Filter by worst-case result
pdt tol list --critical                       # Show only critical stackups
pdt tol show TOL@1                            # Show details with analysis
pdt tol analyze TOL@1                         # Run worst-case, RSS, Monte Carlo
pdt tol analyze TOL@1 --iterations 50000      # Custom Monte Carlo iterations
pdt tol edit TOL@1                            # Open in editor
```

### Manufacturing Processes

```bash
pdt proc new --title "CNC Milling" --type machining
pdt proc new --title "Final Assembly" --type assembly --op-number "OP-020"
pdt proc list                                 # List all processes
pdt proc list --type machining                # Filter by process type
pdt proc list --status approved               # Filter by status
pdt proc show PROC@1                          # Show details
pdt proc edit PROC@1                          # Open in editor
```

Process types: `machining`, `assembly`, `inspection`, `test`, `finishing`, `packaging`, `handling`, `heat_treat`, `welding`, `coating`

### Control Plan Items (SPC, Inspection)

```bash
pdt ctrl new --title "Bore Diameter SPC" --type spc --process PROC@1
pdt ctrl new --title "Visual Check" --type visual --critical
pdt ctrl list                                 # List all controls
pdt ctrl list --type spc                      # Filter by control type
pdt ctrl list --process PROC@1                # Filter by process
pdt ctrl list --critical                      # Show only CTQ controls
pdt ctrl show CTRL@1                          # Show details
pdt ctrl edit CTRL@1                          # Open in editor
```

Control types: `spc`, `inspection`, `poka_yoke`, `visual`, `functional_test`, `attribute`

### Work Instructions

```bash
pdt work new --title "CNC Mill Setup" --process PROC@1 --doc-number "WI-MACH-001"
pdt work list                                 # List all work instructions
pdt work list --process PROC@1                # Filter by process
pdt work list --search "setup"                # Search in title
pdt work show WORK@1                          # Show details
pdt work edit WORK@1                          # Open in editor
```

### Non-Conformance Reports (NCRs)

```bash
pdt ncr new --title "Bore Diameter Out of Tolerance" --type internal --severity major
pdt ncr new --title "Supplier Material Issue" --type supplier --severity critical --category material
pdt ncr list                                  # List all NCRs
pdt ncr list --type internal                  # Filter by NCR type
pdt ncr list --severity critical              # Filter by severity
pdt ncr list --ncr-status open                # Filter by workflow status
pdt ncr show NCR@1                            # Show details
pdt ncr edit NCR@1                            # Open in editor
```

NCR types: `internal`, `supplier`, `customer`
Severity levels: `minor`, `major`, `critical`
Categories: `dimensional`, `cosmetic`, `material`, `functional`, `documentation`, `process`, `packaging`

### Corrective/Preventive Actions (CAPAs)

```bash
pdt capa new --title "Tool Wear Detection" --type corrective --ncr NCR@1
pdt capa new --title "Process Improvement" --type preventive --source trend_analysis
pdt capa list                                 # List all CAPAs
pdt capa list --type corrective               # Filter by CAPA type
pdt capa list --capa-status implementation    # Filter by workflow status
pdt capa list --overdue                       # Show overdue CAPAs
pdt capa show CAPA@1                          # Show details
pdt capa edit CAPA@1                          # Open in editor
```

CAPA types: `corrective`, `preventive`
Source types: `ncr`, `audit`, `customer_complaint`, `trend_analysis`, `risk`

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

## Risk Example (FMEA)

```yaml
id: RISK-01HC2JB7SMQX7RS1Y0GFKBHPTD
type: design
title: "Battery Thermal Runaway"

category: "Electrical Safety"
tags: [battery, thermal, safety]

description: |
  Risk of thermal runaway in lithium-ion battery pack during
  charging or high-temperature operation.

failure_mode: |
  Battery cells exceed thermal limits causing cascading
  thermal runaway across the pack.

cause: |
  Internal short circuit, overcharging, or external heat source
  causing cell temperature to exceed safe limits.

effect: |
  Fire, explosion, or toxic gas release endangering users
  and damaging equipment.

# FMEA Risk Assessment (1-10 scale)
severity: 9      # Impact if failure occurs
occurrence: 3    # Likelihood of occurrence
detection: 4     # Ability to detect before failure
rpn: 108         # Risk Priority Number (S x O x D)

mitigations:
  - action: "Add thermal cutoff protection circuit"
    type: prevention
    status: completed
    owner: "John Smith"
  - action: "Add temperature monitoring sensors"
    type: detection
    status: in_progress
    owner: "Jane Doe"

status: review
risk_level: medium

links:
  related_to:
    - REQ-01HC2JB7SMQX7RS1Y0GFKBHPTE
  mitigated_by:
    - REQ-01HC2JB7SMQX7RS1Y0GFKBHPTF
  verified_by:
    - TEST-01HC2JB7SMQX7RS1Y0GFKBHPTG

created: 2024-01-15T10:30:00Z
author: Jane Doe
revision: 2
```

## Test Example (Verification/Validation Protocol)

```yaml
id: TEST-01HC2JB7SMQX7RS1Y0GFKBHPTF
type: verification
test_level: system
test_method: test
title: "Temperature Cycling Test"

category: "Environmental"
tags: [thermal, environmental, reliability]

objective: |
  Verify the device operates within specified temperature range
  as required by REQ-01HC2JB7SMQX7RS1Y0GFKBHPTD.

preconditions:
  - "Unit at room temperature (23C +/- 2C)"
  - "All test equipment calibrated"
  - "Power supply connected"

equipment:
  - name: "Temperature Chamber"
    specification: "-40C to +100C range, 0.5C accuracy"
    calibration_required: true
  - name: "Multimeter"
    specification: "DC voltage measurement"
    calibration_required: true

procedure:
  - step: 1
    action: "Place unit in chamber at 23C, power on"
    expected: "Unit boots successfully"
    acceptance: "All LEDs illuminate correctly"
  - step: 2
    action: "Ramp chamber to -20C at 2C/min"
    expected: "Unit remains operational"
    acceptance: "No errors logged"
  - step: 3
    action: "Hold at -20C for 4 hours"
    expected: "Continuous operation"
    acceptance: "All functions pass self-test"
  - step: 4
    action: "Ramp chamber to +50C at 2C/min"
    expected: "Unit remains operational"
    acceptance: "No errors logged"

acceptance_criteria:
  - "All steps pass"
  - "No errors in system log"
  - "All functions operational at temperature extremes"

environment:
  temperature: "Per procedure"
  humidity: "< 80% RH (non-condensing)"

estimated_duration: "8 hours"

priority: high
status: approved

links:
  verifies:
    - REQ-01HC2JB7SMQX7RS1Y0GFKBHPTD
  mitigates:
    - RISK-01HC2JB7SMQX7RS1Y0GFKBHPTE

created: 2024-01-15T10:30:00Z
author: Jane Doe
revision: 1
```

## Result Example

```yaml
id: RSLT-01HC2JB7SMQX7RS1Y0GFKBHPTG
test_id: TEST-01HC2JB7SMQX7RS1Y0GFKBHPTF
test_revision: 1
title: "Temperature Cycling Test - Run 1"

verdict: pass
verdict_rationale: |
  All steps completed successfully. Device operated within
  specification at both temperature extremes.

category: "Environmental"

executed_date: 2024-02-01T09:00:00Z
executed_by: "John Smith"

sample_info:
  sample_id: "SN-001234"
  serial_number: "001234"
  lot_number: "LOT-2024-001"
  configuration: "Rev B hardware, v1.2.0 firmware"

environment:
  temperature: "-20C to +50C per procedure"
  humidity: "45% RH"
  location: "Lab A, Environmental Chamber #3"

equipment_used:
  - name: "Temperature Chamber"
    asset_id: "ENV-CHAM-003"
    calibration_date: "2024-01-15"
    calibration_due: "2025-01-15"

step_results:
  - step: 1
    result: pass
    observed: "Unit booted in 12 seconds"
  - step: 2
    result: pass
    observed: "No anomalies during ramp"
  - step: 3
    result: pass
    observed: "Self-test passed at 1h, 2h, 3h, 4h intervals"
    measurement:
      value: -20.1
      unit: "C"
      min: -21
      max: -19
  - step: 4
    result: pass
    observed: "No anomalies during ramp"

deviations: []
failures: []

duration: "8h 15m"
notes: |
  Test completed without incident. Minor temperature overshoot
  observed during cold ramp (reached -20.5C briefly).

status: approved

links:
  test: TEST-01HC2JB7SMQX7RS1Y0GFKBHPTF

created: 2024-02-01T17:30:00Z
author: John Smith
revision: 1
```

## Component Example (BOM)

```yaml
id: CMP-01HC2JB7SMQX7RS1Y0GFKBHPTD
part_number: "PN-001"
revision: "A"
title: "Widget Bracket"

make_buy: buy
category: mechanical
material: "6061-T6 Aluminum"
mass_kg: 0.125
unit_cost: 12.50

suppliers:
  - name: "Acme Corp"
    supplier_pn: "ACM-123"
    lead_time_days: 14
    moq: 100
    unit_cost: 11.00

documents:
  - type: "drawing"
    path: "drawings/PN-001.pdf"
    revision: "A"

tags: [mechanical, bracket]
status: approved

links:
  used_in: []

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## Supplier Example

```yaml
id: SUP-01HC2JB7SMQX7RS1Y0GFKBHPTA
name: "Acme Manufacturing Corp"
short_name: "Acme"
website: "https://acme-mfg.com"

contacts:
  - name: "John Smith"
    role: "Sales Manager"
    email: "john.smith@acme-mfg.com"
    phone: "+1-555-123-4567"
    primary: true

addresses:
  - type: headquarters
    street: "123 Industrial Way"
    city: "San Francisco"
    state: "CA"
    postal: "94102"
    country: "USA"

payment_terms: "Net 30"
currency: USD

certifications:
  - name: "ISO 9001:2015"
    expiry: 2026-06-30

capabilities: [machining, sheet_metal, assembly, finishing]

notes: "Preferred supplier for precision machined parts."
tags: [preferred, machining]
status: approved

links:
  approved_for: []

created: 2024-01-10T09:00:00Z
author: Jack Hale
entity_revision: 1
```

## Quote Example (Supplier Quotation)

```yaml
id: QUOT-01HC2JB7SMQX7RS1Y0GFKBHPTD
title: "Acme Corp Quote"

# Link to supplier entity (create supplier first with pdt sup new)
supplier: SUP-01HC2JB7SMQX7RS1Y0GFKBHPTA

# Quotes link to either component OR assembly (not both)
component: CMP-01HC2JB7SMQX7RS1Y0GFKBHPTC
# assembly: ASM-...  # Use this instead for assembly quotes

# Supplier's quote reference number
quote_ref: "ACM-Q-2024-001"

currency: USD

# Quantity-based pricing tiers
price_breaks:
  - min_qty: 1
    unit_price: 15.00
    lead_time_days: 14
  - min_qty: 100
    unit_price: 12.50
    lead_time_days: 14
  - min_qty: 500
    unit_price: 10.00
    lead_time_days: 21

moq: 1
tooling_cost: 500.00
lead_time_days: 14

quote_date: 2024-01-15
valid_until: 2024-04-15

quote_status: received   # pending | received | accepted | rejected | expired
tags: [bracket]
status: draft

links:
  related_quotes: []

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## Feature Example (Tolerances)

```yaml
id: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTE
component: CMP-01HC2JB7SMQX7RS1Y0GFKBHPTD
feature_type: hole
title: "Mounting Hole A"

# Dimensions use plus_tol/minus_tol (NOT +/- symbol)
dimensions:
  - name: "diameter"
    nominal: 10.0
    plus_tol: 0.1      # +0.1
    minus_tol: 0.05    # -0.05
    units: "mm"

gdt:
  - symbol: position
    value: 0.25
    units: "mm"
    datum_refs: ["A", "B", "C"]
    material_condition: mmc

drawing:
  number: "DWG-001"
  revision: "A"
  zone: "B3"

tags: [mounting]
status: approved

links:
  used_in_mates: []
  used_in_stackups: []

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## Mate Example (Fit Calculation)

```yaml
id: MATE-01HC2JB7SMQX7RS1Y0GFKBHPTF
title: "Pin-Hole Mate"
description: "Locating pin engagement"

feature_a: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTE  # Hole
feature_b: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTG  # Shaft

mate_type: clearance_fit

# Auto-calculated from feature dimensions
fit_analysis:
  worst_case_min_clearance: 0.02
  worst_case_max_clearance: 0.15
  fit_result: clearance    # clearance | interference | transition

notes: "Critical for alignment"
tags: [alignment, locating]
status: approved

links:
  used_in_stackups: []
  verifies: []

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## Stackup Example (Tolerance Analysis)

```yaml
id: TOL-01HC2JB7SMQX7RS1Y0GFKBHPTH
title: "Gap Analysis"

target:
  name: "Gap"
  nominal: 1.0
  upper_limit: 1.5
  lower_limit: 0.5
  units: "mm"
  critical: true

# Contributors use plus_tol/minus_tol (NOT +/- symbol)
contributors:
  - name: "Part A Length"
    feature_id: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTE
    direction: positive
    nominal: 10.0
    plus_tol: 0.1
    minus_tol: 0.05
    distribution: normal
    source: "DWG-001 Rev A"
  - name: "Part B Length"
    direction: negative
    nominal: 9.0
    plus_tol: 0.08
    minus_tol: 0.08
    distribution: normal
    source: "DWG-002 Rev A"

# Auto-calculated by 'pdt tol analyze'
analysis_results:
  worst_case:
    min: 0.87
    max: 1.18
    margin: 0.32
    result: pass
  rss:
    mean: 1.0
    sigma_3: 0.11
    margin: 0.39
    cpk: 4.56
    yield_percent: 99.9999
  monte_carlo:
    iterations: 10000
    mean: 1.0
    std_dev: 0.037
    min: 0.85
    max: 1.14
    yield_percent: 100.0
    percentile_2_5: 0.93
    percentile_97_5: 1.07

disposition: approved
tags: [critical, assembly]
status: approved

links:
  verifies: []
  mates_used: []

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## Manufacturing Process Example

```yaml
id: PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
title: "CNC Milling - Housing"
description: |
  Precision CNC milling of main housing from aluminum billet.

process_type: machining
operation_number: "OP-010"

equipment:
  - name: "Haas VF-2 CNC Mill"
    equipment_id: "EQ-001"
    capability: "3-axis, 30x16x20 travel"

parameters:
  - name: "Spindle Speed"
    value: 8000
    units: "RPM"
    min: 7500
    max: 8500
  - name: "Feed Rate"
    value: 500
    units: "mm/min"

cycle_time_minutes: 15.5
setup_time_minutes: 30

capability:
  cpk: 1.45
  sample_size: 50
  study_date: 2024-01-15

operator_skill: intermediate

safety:
  ppe: [safety_glasses, hearing_protection, steel_toe_boots]
  hazards: ["rotating machinery", "sharp edges", "coolant splash"]

tags: [machining, housing, critical]
status: approved

links:
  produces:
    - CMP-01HC2JB7SMQX7RS1Y0GFKBHPTD
  controls:
    - CTRL-01KC5B5M87QMYVJT048X27TJ5S
  work_instructions:
    - WORK-01KC5B5XKGWKFTTA9YWTGJB9GE

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## Control Plan Item Example (SPC)

```yaml
id: CTRL-01KC5B5M87QMYVJT048X27TJ5S
title: "Bore Diameter SPC"
description: |
  Statistical process control for critical bore diameter.

control_type: spc
control_category: variable

characteristic:
  name: "Bore Diameter"
  nominal: 25.0
  upper_limit: 25.025
  lower_limit: 25.000
  units: "mm"
  critical: true

measurement:
  method: "Bore gauge measurement"
  equipment: "Mitutoyo Bore Gauge GA-045"
  gage_rr_percent: 12.5

sampling:
  type: continuous
  frequency: "5 parts"
  sample_size: 1

control_limits:
  ucl: 25.018
  lcl: 25.007
  target: 25.0125

reaction_plan: |
  1. Quarantine affected parts
  2. Notify supervisor immediately
  3. Adjust offset per SOP-123
  4. Verify correction with 3 consecutive good parts

tags: [spc, bore, critical]
status: approved

links:
  process: PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
  feature: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTE
  verifies:
    - REQ-01HC2JB7SMQX7RS1Y0GFKBHPTD

created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## NCR Example (Non-Conformance Report)

```yaml
id: NCR-01KC5B6E1RKCPKGACCH569FX5R
title: "Bore Diameter Out of Tolerance"
ncr_number: "NCR-2024-0042"
report_date: 2024-01-20

ncr_type: internal
severity: major
category: dimensional

detection:
  found_at: in_process
  found_by: "J. Smith"
  found_date: 2024-01-20
  operation: "CNC Milling - Op 010"

affected_items:
  part_number: "PN-12345"
  lot_number: "LOT-2024-01-20A"
  serial_numbers: ["SN-001", "SN-002", "SN-003"]
  quantity_affected: 3

defect:
  characteristic: "Bore Diameter"
  specification: "25.00 +0.025/-0.000 mm"
  actual: "24.985 mm"
  deviation: -0.015

containment:
  - action: "Quarantine affected lot"
    date: 2024-01-20
    completed_by: "J. Smith"
    status: completed
  - action: "100% inspection of in-process inventory"
    date: 2024-01-20
    completed_by: "Q. Team"
    status: completed

disposition:
  decision: rework
  decision_date: 2024-01-21
  decision_by: "R. Williams"
  justification: "Can re-bore to next oversized tolerance per ECN-123"
  mrb_required: true

cost_impact:
  rework_cost: 150.00
  scrap_cost: 0.00
  currency: "USD"

ncr_status: closed
tags: [bore, rework]
status: approved

links:
  component: CMP-01HC2JB7SMQX7RS1Y0GFKBHPTD
  process: PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
  control: CTRL-01KC5B5M87QMYVJT048X27TJ5S
  capa: CAPA-01KC5B6P6PSHZ6TMCSDJQQ6HG3

created: 2024-01-20T14:30:00Z
author: J. Smith
entity_revision: 2
```

## CAPA Example (Corrective Action)

```yaml
id: CAPA-01KC5B6P6PSHZ6TMCSDJQQ6HG3
title: "Tool Wear Detection Improvement"
capa_number: "CAPA-2024-0015"

capa_type: corrective

source:
  type: ncr
  reference: NCR-01KC5B6E1RKCPKGACCH569FX5R

problem_statement: |
  Bore diameter NCRs occurring due to undetected tool wear.
  3 NCRs in past month related to undersized bores.

root_cause_analysis:
  method: five_why
  root_cause: |
    Lack of systematic tool life monitoring in CNC program.
    Operators relying on visual inspection which is unreliable.
  contributing_factors:
    - "No tool life tracking in CNC controller"
    - "Insufficient in-process inspection frequency"
    - "No automatic tool wear compensation"

actions:
  - action_number: 1
    description: "Implement tool life management in CNC controller"
    action_type: corrective
    owner: "Manufacturing Engineering"
    due_date: 2024-02-15
    completed_date: 2024-02-10
    status: completed
    evidence: "ECN-456 implemented, verified in production"
  - action_number: 2
    description: "Increase SPC sampling frequency from 5 to 3 parts"
    action_type: preventive
    owner: "Quality Engineering"
    due_date: 2024-02-01
    completed_date: 2024-02-01
    status: verified
    evidence: "Control plan updated, operators trained"

effectiveness:
  verified: true
  verified_date: 2024-03-15
  result: effective
  evidence: "Zero bore diameter NCRs in 60 days post-implementation"

closure:
  closed: true
  closed_date: 2024-03-20
  closed_by: "Quality Manager"

timeline:
  initiated_date: 2024-01-21
  target_date: 2024-03-31

capa_status: closed
tags: [tool_wear, machining]
status: approved

links:
  ncrs:
    - NCR-01KC5B6E1RKCPKGACCH569FX5R
  processes_modified:
    - PROC-01KC5B2GDDQ0JAXFVXYYZ9DWDZ
  controls_added: []

created: 2024-01-21T09:00:00Z
author: Quality Manager
entity_revision: 3
```

## Manufacturing Quality Loop

PDT supports the complete manufacturing quality loop:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    PROC      │────▶│    CTRL      │────▶│    WORK      │
│  (Process)   │     │  (Control)   │     │ (Work Inst)  │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │                    ▼                    │
       │             ┌──────────────┐            │
       │             │    NCR       │◀───────────┘
       │             │ (Non-Conf)   │
       │             └──────────────┘
       │                    │
       │                    ▼
       │             ┌──────────────┐
       └────────────▶│    CAPA      │
                     │  (Corrective)│
                     └──────────────┘
```

1. **PROC** defines *what* manufacturing operations to perform
2. **CTRL** defines *how* to monitor/control the process (SPC, inspection)
3. **WORK** provides step-by-step instructions for *operators*
4. **NCR** captures quality issues found during manufacturing
5. **CAPA** drives systematic improvement back to processes

## Tolerance Format

PDT uses `plus_tol` and `minus_tol` fields instead of the `±` symbol (which is hard to type):

```yaml
# Correct: 10.0 +0.1/-0.05
dimensions:
  - name: "diameter"
    nominal: 10.0
    plus_tol: 0.1     # Positive deviation allowed
    minus_tol: 0.05   # Negative deviation allowed (stored as positive number)
```

Both values are stored as **positive numbers**. The actual tolerance range is:
- Maximum: `nominal + plus_tol` = 10.1
- Minimum: `nominal - minus_tol` = 9.95

## Validation

PDT validates files against JSON Schema with detailed error messages:

```
error[pdt::schema::validation]: Schema validation failed
  --> requirements/inputs/REQ-01HC2.pdt.yaml:8:1
   |
 8 | status: pending
   | ^^^^^^^^^^^^^^^ Invalid enum value
   |
  help: Valid values: draft, review, approved, released, obsolete
```

## Status Workflow

```
draft → review → approved → released
                    ↓           ↓
                 obsolete ← ← ← ┘
```

| Status | Description |
|--------|-------------|
| draft | Initial creation, still being written |
| review | Ready for stakeholder review |
| approved | Signed off and baselined |
| released | Released to production/manufacturing |
| obsolete | No longer applicable |

## Priority Levels

| Priority | Use For |
|----------|---------|
| critical | Safety, regulatory, blocking requirements |
| high | Core functionality, key differentiators |
| medium | Standard features, quality of life |
| low | Nice to have, future considerations |

## Risk Assessment (FMEA)

PDT uses FMEA (Failure Mode and Effects Analysis) methodology:

### FMEA Ratings (1-10 scale)

| Factor | 1 | 10 |
|--------|---|-----|
| **Severity** | Minimal impact | Catastrophic, safety hazard |
| **Occurrence** | Very unlikely | Almost certain |
| **Detection** | Always detected | Cannot be detected |

### Risk Priority Number (RPN)

RPN = Severity x Occurrence x Detection (range: 1-1000)

| RPN Range | Risk Level | Action |
|-----------|------------|--------|
| 1-50 | Low | Monitor, no immediate action needed |
| 51-150 | Medium | Plan mitigations, track progress |
| 151-400 | High | Prioritize mitigations, escalate |
| 401+ | Critical | Immediate action required |

### Mitigation Types

| Type | Purpose |
|------|---------|
| **prevention** | Reduces occurrence probability |
| **detection** | Improves ability to detect before failure |

## Test Engineering

### Verification vs Validation

| Type | Purpose | Question |
|------|---------|----------|
| **Verification** | Did we build it right? | Confirms design outputs meet inputs |
| **Validation** | Did we build the right thing? | Confirms product meets user needs |

### V-Model Test Levels

| Level | Tests Against | Scope |
|-------|---------------|-------|
| **Unit** | Detailed design | Individual components |
| **Integration** | Architecture design | Component interactions |
| **System** | System requirements | Complete system |
| **Acceptance** | User needs | End-user scenarios |

### IADT Methods

Tests can use different verification methods (Inspection, Analysis, Demonstration, Test):

| Method | Description | When to Use |
|--------|-------------|-------------|
| **Inspection** | Visual examination | Workmanship, labeling, documentation |
| **Analysis** | Calculation/simulation | Complex systems, safety-critical |
| **Demonstration** | Show functionality | User interface, simple operations |
| **Test** | Measured execution | Performance, environmental, stress |

## Tolerance Analysis

PDT supports three analysis methods for tolerance stackups:

### Worst-Case Analysis

Assumes all dimensions are at their worst-case limits simultaneously:
- **Min result**: All positive contributors at minimum, all negative at maximum
- **Max result**: All positive contributors at maximum, all negative at minimum
- **Conservative** but often overly pessimistic

### RSS (Root Sum Square) Analysis

Statistical analysis assuming normal distributions:
- Calculates mean and 3σ spread
- Computes Cpk (process capability index)
- Estimates yield percentage
- More realistic than worst-case for multi-contributor stacks

| Cpk | Yield | Quality Level |
|-----|-------|---------------|
| 0.33 | 68.27% | Poor |
| 0.67 | 95.45% | Marginal |
| 1.0 | 99.73% | Capable |
| 1.33 | 99.99% | Good |
| 1.67 | 99.9997% | Excellent |
| 2.0 | 99.9999% | Six Sigma |

### Monte Carlo Simulation

Runs thousands of random samples:
- Supports normal, uniform, and triangular distributions
- Provides actual yield percentage
- Reports 95% confidence interval (2.5th to 97.5th percentile)
- Default: 10,000 iterations

```bash
# Run analysis with default iterations
pdt tol analyze TOL@1

# Run with more iterations for higher confidence
pdt tol analyze TOL@1 --iterations 100000
```

### Test Verdicts

| Verdict | Meaning | Follow-up |
|---------|---------|-----------|
| **pass** | All criteria met | None required |
| **fail** | One or more criteria not met | Action items required |
| **conditional** | Passed with deviations | Document justification |
| **incomplete** | Could not complete test | Reschedule |
| **not_applicable** | Test not applicable | Document rationale |

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
