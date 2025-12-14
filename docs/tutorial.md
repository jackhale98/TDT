# TDT Tutorial: Building a Complete Product Development Project

This tutorial walks you through creating a complete TDT project from scratch. We'll build a simple **LED Flashlight** product, covering all entity types and showing how they connect together for full traceability.

## Table of Contents

1. [Project Setup](#1-project-setup)
2. [Requirements](#2-requirements)
3. [Components & Bill of Materials](#3-components--bill-of-materials)
4. [Suppliers & Quotes](#4-suppliers--quotes)
5. [Features & Tolerances](#5-features--tolerances)
6. [Mates & Fit Analysis](#6-mates--fit-analysis)
7. [Tolerance Stackups](#7-tolerance-stackups)
8. [Risk Management](#8-risk-management)
9. [Manufacturing Processes](#9-manufacturing-processes)
10. [Process Controls](#10-process-controls)
11. [Test Protocols](#11-test-protocols)
12. [Test Results](#12-test-results)
13. [Assemblies](#13-assemblies)
14. [Handling Issues (NCRs & CAPAs)](#14-handling-issues-ncrs--capas)
15. [Project Status & Validation](#15-project-status--validation)
16. [Traceability](#16-traceability)
17. [Version Control Integration](#17-version-control-integration)
18. [Baseline Management](#18-baseline-management)
19. [Bulk Operations & Unix Pipelines](#19-bulk-operations--unix-pipelines)

---

## 1. Project Setup

### Initialize Your Project

Create a new directory and initialize TDT:

```bash
mkdir led-flashlight
cd led-flashlight
tdt init
```

This creates:
- A `.tdt/` directory with configuration
- A `.gitignore` file (excludes local short ID mappings)
- A git repository
- Standard directory structure for all entity types

### Configure Your Project

Edit `.tdt/config.yaml` to set your default author:

```yaml
# .tdt/config.yaml
author: "Your Name"
editor: "code"  # or vim, nano, etc.
```

You can also set this globally:
```bash
git config --global user.name "Your Name"
```

### CLI Tips

TDT commands support both long flags (readable) and short flags (efficient):

| Long Form | Short | Description |
|-----------|-------|-------------|
| `--title` | `-T` | Entity title |
| `--no-edit` | `-n` | Skip editor, create immediately |
| `--verifies` | `-R` | Requirements a test verifies |
| `--mitigates` | `-M` | Risks a test mitigates |
| `--bom` | `-b` | BOM items for assembly (ID:QTY pairs) |
| `--breaks` | `-B` | Price breaks for quote (QTY:PRICE:LEAD) |

Some commands also accept **positional arguments** for common operations:
- `tdt mate new FEAT@1 FEAT@2` - Features as positional args
- `tdt link add SRC DST link_type` - Link type as 3rd arg
- `tdt asm add ASM@1 CMP@1:2 CMP@2:1` - Multiple ID:QTY pairs

---

## 2. Requirements

Requirements define what your product must do. TDT supports **input requirements** (from customers/stakeholders) and **output requirements** (derived/allocated).

### Create Input Requirements

```bash
# Brightness requirement
tdt req new --title "Light Output" --type input --priority critical \
  --category performance --status approved --no-edit

# Battery life requirement
tdt req new --title "Battery Life" --type input --priority high \
  --category performance --status approved --no-edit

# Water resistance requirement
tdt req new --title "Water Resistance" --type input --priority medium \
  --category environmental --status approved --no-edit

# Drop resistance requirement
tdt req new --title "Drop Resistance" --type input --priority high \
  --category durability --status approved --no-edit
```

### View Your Requirements

```bash
tdt req list
```

Output:
```
SHORT    ID               TYPE   TITLE             STATUS    PRIORITY
----------------------------------------------------------------------
REQ@1    REQ-01KCA...     input  Light Output      approved  critical
REQ@2    REQ-01KCA...     input  Battery Life      approved  high
REQ@3    REQ-01KCA...     input  Water Resistance  approved  medium
REQ@4    REQ-01KCA...     input  Drop Resistance   approved  high
```

### Edit Requirements with Full Details

Open a requirement in your editor to add detailed information:

```bash
tdt req edit REQ@1
```

Add acceptance criteria, rationale, and source information:

```yaml
id: REQ-01KCA...
type: input
title: Light Output
source:
  document: PRD-2024-001
  section: "3.1"
  revision: A
category: performance
text: |
  The flashlight shall produce a minimum of 500 lumens
  at maximum brightness setting.
rationale: |
  500 lumens provides adequate illumination for outdoor use
  at distances up to 50 meters.
acceptance_criteria:
  - Light output >= 500 lumens measured at LED surface
  - Beam angle between 15-20 degrees
priority: critical
status: approved
tags:
  - optical
  - performance
links: {}
```

### Create Derived Requirements

Derived requirements flow down from input requirements:

```bash
tdt req new --title "LED Selection" --type derived --priority high \
  --category electrical --status approved --no-edit
```

Then link it to the parent requirement:

```bash
# Positional syntax (link type as 3rd argument)
# REQ@1 automatically gets derived_by -> REQ@5 (reciprocal)
tdt link add REQ@5 REQ@1 derives_from

# Long form - equivalent
tdt link add REQ@5 REQ@1 --link-type derives_from
```

> **Tip**: Reciprocal links are added by default, maintaining bidirectional traceability. Use `--no-reciprocal` to create one-way links.

---

## 3. Components & Bill of Materials

Components are the parts that make up your product. They can be **make** (manufactured) or **buy** (purchased).

### Create Components

```bash
# Housing (make part)
tdt cmp new --title "Flashlight Housing" --part-number "FL-HSG-001" \
  --make-buy make --category mechanical --no-edit

# LED module (buy part)
tdt cmp new --title "LED Module" --part-number "LED-500LM" \
  --make-buy buy --category electrical --no-edit

# Battery holder (buy part)
tdt cmp new --title "Battery Holder" --part-number "BH-2xAA" \
  --make-buy buy --category electrical --no-edit

# Lens (buy part)
tdt cmp new --title "Optical Lens" --part-number "LENS-20DEG" \
  --make-buy buy --category optical --no-edit

# O-ring seal (buy part)
tdt cmp new --title "Housing O-Ring" --part-number "OR-25x2" \
  --make-buy buy --category mechanical --no-edit

# End cap (make part)
tdt cmp new --title "End Cap" --part-number "FL-CAP-001" \
  --make-buy make --category mechanical --no-edit
```

### View Components

```bash
tdt cmp list
```

### Add Component Details

Edit a component to add specifications:

```bash
tdt cmp edit CMP@1
```

```yaml
id: CMP-01KCA...
part_number: FL-HSG-001
title: Flashlight Housing
description: |
  Main housing body, CNC machined from 6061-T6 aluminum.
  Provides structural support and heat dissipation for LED.
make_buy: make
category: mechanical
material: "6061-T6 Aluminum"
mass_kg: 0.085
unit_cost: 12.50
specifications:
  finish: "Type III Hard Anodize, Black"
  wall_thickness: "2.0mm minimum"
status: draft
```

---

## 4. Suppliers & Quotes

For buy parts, track suppliers and quotes.

### Create Suppliers

```bash
tdt sup new --name "BrightLED Inc" --short-name "BrightLED" \
  --contact-email "sales@brightled.example" --no-edit

tdt sup new --name "PowerCell Supply" --short-name "PowerCell" \
  --contact-email "orders@powercell.example" --no-edit
```

### Create Quotes

```bash
# Simple quote with single price
tdt quote new -T "LED Module Quote" -s SUP@1 -c CMP@2 \
  -p 3.50 --moq 100 -l 14 -n

# Quote with multiple price breaks (QTY:PRICE:LEAD_TIME)
tdt quote new -T "Battery Holder Quote" -s SUP@2 -c CMP@3 \
  --breaks "100:0.95:14,500:0.85:10,1000:0.75:7" -n
```

> **Price breaks**: Use `--breaks` with comma-separated `QTY:PRICE:LEAD_TIME` triplets for volume pricing. When calculating BOM costs, TDT automatically selects the best price break based on purchase quantity.

### View and Compare Quotes

```bash
# List all quotes
tdt quote list

# Compare quotes for a specific component
tdt quote compare CMP@2
```

The compare command shows all quotes for a component side-by-side, sorted by price:

```
Comparing 2 quotes for CMP@2

SHORT    TITLE                SUPPLIER   PRICE    MOQ    LEAD    TOOLING  STATUS
---------------------------------------------------------------------------------
QUOT@1   LED Module Quote     SUP@1      $3.50    100    14d     -        pending
QUOT@3   LED Alt Quote        SUP@3      $3.75    50     7d      -        pending

★ Lowest price: $3.50 from SUP@1
```

### Select a Quote for Costing

Link a quote to its component for BOM cost calculations:

```bash
# Set which quote to use for a component's pricing
tdt cmp set-quote CMP@2 QUOT@1

# Clear the selection (revert to manual unit_cost)
tdt cmp clear-quote CMP@2
```

When a quote is selected, `tdt asm cost` will use that quote's price breaks instead of the component's manual `unit_cost` field.

---

## 5. Features & Tolerances

Features are the geometric characteristics of components that matter for fit and function.

### Create Features

```bash
# Housing bore (where LED mounts)
tdt feat new --title "LED Mounting Bore" --component CMP@1 \
  --feature-type hole --no-edit

# Housing thread (for end cap)
tdt feat new --title "End Cap Thread" --component CMP@1 \
  --feature-type thread --no-edit

# LED module OD
tdt feat new --title "LED Module OD" --component CMP@2 \
  --feature-type shaft --no-edit

# End cap thread
tdt feat new --title "Cap Thread" --component CMP@6 \
  --feature-type thread --no-edit

# O-ring groove
tdt feat new --title "O-Ring Groove" --component CMP@6 \
  --feature-type slot --no-edit
```

### Add Tolerances to Features

Edit a feature to add dimensional tolerances:

```bash
tdt feat edit FEAT@1
```

```yaml
id: FEAT-01KCA...
title: LED Mounting Bore
component: CMP-01KCA...  # Housing
feature_type: hole
description: |
  Bore in housing front face for LED module press-fit.
  Critical for thermal contact and alignment.
nominal: 20.00
plus_tolerance: 0.021
minus_tolerance: 0.000
unit: mm
notes: |
  H7 tolerance class for transition/light interference fit.
  Surface finish Ra 1.6 or better for thermal contact.
critical: true
status: approved
```

### View Features

```bash
# All features
tdt feat list

# Features for a specific component
tdt feat list -c CMP@1

# With descriptions
tdt feat list --columns title,description,component
```

---

## 6. Mates & Fit Analysis

Mates define how features from different components interact.

### Create Mates

```bash
# LED to Housing fit (positional syntax)
tdt mate new FEAT@1 FEAT@3 -t interference_fit -T "LED-Housing Press Fit" -n

# End cap thread engagement (long form - equivalent)
tdt mate new --feature-a FEAT@2 --feature-b FEAT@4 \
  --mate-type thread_engagement --title "End Cap Thread Engagement" --no-edit
```

> **Tip**: `tdt mate new FEAT@1 FEAT@3 -t interference_fit -n` is a quick shorthand for creating mates. Use `--help` to see all options.

### Analyze Fit

```bash
tdt mate list
```

The fit analysis shows:
- **Worst-case clearance/interference** calculated from tolerances
- **Fit result**: clearance, interference, or transition
- **Match indicator**: ✓ if result matches intended type, ⚠ if mismatch

---

## 7. Tolerance Stackups

Stackups analyze how tolerances accumulate across multiple features.

### Create a Stackup

```bash
tdt tol new --title "LED to Lens Air Gap" --no-edit
```

### Add Features as Contributors (CLI Method - Recommended)

The fastest way to build a stackup is using `tdt tol add` to pull feature dimensions directly:

```bash
# Add features with direction: + for positive, ~ for negative
tdt tol add TOL@1 +FEAT@1 ~FEAT@3

# Add multiple features at once
tdt tol add TOL@1 +FEAT@1 ~FEAT@3 +FEAT@5

# Add features and run analysis immediately
tdt tol add TOL@1 +FEAT@1 ~FEAT@3 --analyze
```

**Direction prefixes:**
- `+FEAT@N` — Positive direction (adds to the stackup)
- `~FEAT@N` — Negative direction (subtracts from the stackup)

> **Note:** Use `~` instead of `-` for negative direction to avoid conflicts with CLI flags.

This automatically pulls the feature's nominal value and tolerances into the stackup contributor list.

### Remove Contributors

```bash
tdt tol rm TOL@1 FEAT@1 FEAT@3
```

### Edit Stackup Details

For advanced configuration (target limits, descriptions, distribution settings), edit the file directly:

```bash
tdt tol edit TOL@1
```

```yaml
id: TOL-01KCA...
title: LED to Lens Air Gap
description: |
  Critical air gap between LED emitter surface and lens.
  Affects beam focus and light output efficiency.
target:
  nominal: 2.5
  lower_limit: 2.0
  upper_limit: 3.0
  unit: mm
contributors:
  - feature_id: FEAT-01KCA...  # LED Mounting Bore
    description: "Housing bore depth"
    direction: positive
    nominal: 15.0
    tolerance: 0.1
    distribution: normal
    sigma: 3.0
  - feature_id: FEAT-01KCA...  # LED Module
    description: "LED module height"
    direction: negative
    nominal: 12.5
    tolerance: 0.15
    distribution: normal
    sigma: 3.0
analysis_results: {}
status: draft
```

### Run Analysis

```bash
tdt tol analyze TOL@1
```

This calculates:
- **Worst-case** min/max values
- **RSS (Root Sum Square)** statistical analysis
- **Monte Carlo** simulation with yield prediction
- **Cpk** process capability index

### View Results

```bash
tdt tol list
```

```
SHORT    TITLE                  RESULT     CPK     YIELD    STATUS
------------------------------------------------------------------
TOL@1    LED to Lens Air Gap    acceptable 1.45    99.8%    draft
```

---

## 8. Risk Management

Identify and mitigate risks to your design and manufacturing processes.

### Create Design Risks

```bash
tdt risk new --title "LED Overheating" --type design \
  --category thermal --no-edit
```

Edit to add FMEA details:

```bash
tdt risk edit RISK@1
```

```yaml
id: RISK-01KCA...
type: design
title: LED Overheating
category: thermal
failure_mode: |
  LED junction temperature exceeds maximum rating during
  continuous operation at maximum brightness.
cause: |
  Insufficient thermal path from LED to housing.
  Inadequate heat sink mass.
effect: |
  Reduced LED life, color shift, potential thermal shutdown.
severity: 8
occurrence: 4
detection: 6
# RPN = 8 × 4 × 6 = 192
mitigations:
  - action: "Add thermal interface material between LED and housing"
    owner: "Thermal Engineer"
    status: planned
  - action: "Increase housing wall thickness at LED mount"
    owner: "Mechanical Engineer"
    status: completed
status: draft
```

### Create Process Risks

```bash
tdt risk new --title "Housing Machining Defects" --type process \
  --category manufacturing --no-edit
```

### View Risks

```bash
tdt risk list
```

Shows risk level (Critical/High/Medium/Low) and RPN scores.

### Risk Matrix

Visualize risks in a severity × occurrence matrix:

```bash
tdt risk matrix
```

```
Risk Matrix
                            OCCURRENCE
              1    2    3    4    5    6    7    8    9   10
         ┌────────────────────────────────────────────────────
      10 │  -    -    -    -    -    -    -    -    -    -
       9 │  -    -    -    -    -    -    -    -    -    -
S      8 │  -    -    -    1    -    -    -    -    -    -
E      7 │  -    -    -    -    -    -    -    -    -    -
V      6 │  -    -    -    -    -    -    -    -    -    -
E      5 │  -    -    -    -    -    -    -    -    -    -
R      4 │  -    -    -    -    -    -    -    -    -    -
I      3 │  -    -    -    -    -    -    -    -    -    -
T      2 │  -    -    -    -    -    -    -    -    -    -
Y      1 │  -    -    -    -    -    -    -    -    -    -

Total: 1 risks | High: 1
```

Filter by risk type:

```bash
# Show only design risks
tdt risk matrix --risk-type design

# Show risk IDs in cells
tdt risk matrix --show-ids
```

---

## 9. Manufacturing Processes

Define how components are manufactured.

### Create Process Steps

```bash
# Housing machining
tdt proc new --title "Housing CNC Machining" --type machining \
  --operation-number "OP-010" --no-edit

# Housing anodizing
tdt proc new --title "Housing Anodizing" --type finishing \
  --operation-number "OP-020" --no-edit

# Final assembly
tdt proc new --title "Final Assembly" --type assembly \
  --operation-number "OP-030" --no-edit
```

### Link Processes to Components and Risks

```bash
# Link process to component it produces
tdt link add PROC@1 CMP@1 produces

# Link process to associated risks (automatically bidirectional)
tdt link add PROC@1 RISK@2 risks
```

Links are bidirectional by default - RISK@2 automatically gets an `affects` link back to PROC@1.

### Visualize Process Flow

See your manufacturing process sequence with linked controls:

```bash
tdt proc flow
```

```
Process Flow
────────────────────────────────────────────────────────────────

[OP-010] Housing CNC Machining (PROC@1)
  │ Type: machining
  │ Controls: CTRL@1 "LED Bore Diameter Check"
  ▼
[OP-020] Housing Anodizing (PROC@2)
  │ Type: finishing
  ▼
[OP-030] Final Assembly (PROC@3)
  │ Type: assembly
```

Options:

```bash
# Show controls for each process
tdt proc flow --controls

# Show work instructions
tdt proc flow --work-instructions

# Show flow for specific process only
tdt proc flow PROC@1
```

---

## 10. Process Controls

Controls ensure processes produce conforming product.

### Create Controls

```bash
# Bore diameter inspection
tdt ctrl new --title "LED Bore Diameter Check" --type inspection \
  --process PROC@1 --feature FEAT@1 --no-edit
```

Edit to add control details:

```bash
tdt ctrl edit CTRL@1
```

```yaml
id: CTRL-01KCA...
title: LED Bore Diameter Check
control_type: inspection
process: PROC-01KCA...
feature: FEAT-01KCA...
description: |
  100% inspection of LED mounting bore diameter.
characteristic: "Bore diameter 20.000 - 20.021 mm"
measurement_method: "Bore gauge, calibrated"
frequency: "Every part"
acceptance_criteria: "20.000 - 20.021 mm"
reaction_plan: |
  Out of tolerance: Quarantine part, notify quality.
  Trend toward limit: Adjust tool offset.
critical: true
status: draft
```

### View Controls

```bash
tdt ctrl list
```

---

## 11. Test Protocols

Define how requirements will be verified.

### Create Test Protocols

```bash
# Light output test (short flags)
tdt test new -T "Light Output Verification" -t verification -l system \
  -m test -p critical -R REQ@1 -n

# Battery life test
tdt test new -T "Battery Life Test" -t verification -l system -m test \
  -p high -R REQ@2 -n

# Water resistance test (long form - equivalent)
tdt test new --title "Water Resistance Test" \
  --type verification --level system --method test \
  --priority medium --verifies REQ@3 --no-edit

# Drop test (with risk mitigation)
tdt test new -T "Drop Test" -t verification -l system -m test \
  -p high -R REQ@4 -M RISK@1 -n
```

> **Flags**: `-T` title, `-t` type, `-l` level, `-m` method, `-p` priority, `-R` verifies, `-M` mitigates, `-n` no-edit

### Add Test Procedure Details

```bash
tdt test edit TEST@1
```

```yaml
id: TEST-01KCA...
type: verification
test_level: system
test_method: test
title: Light Output Verification
objective: |
  Verify flashlight meets minimum 500 lumen output requirement.
preconditions:
  - Fresh batteries installed
  - Unit at room temperature (23°C ± 2°C)
  - Integrating sphere calibrated
equipment:
  - name: "Integrating Sphere"
    specification: "12-inch diameter, calibrated"
    calibration_required: true
  - name: "Spectrometer"
    specification: "350-750nm range"
    calibration_required: true
procedure:
  - step: 1
    action: "Install fresh AA batteries"
    expected: "Power indicator lights"
  - step: 2
    action: "Place flashlight in integrating sphere"
    expected: "Centered in sphere aperture"
  - step: 3
    action: "Turn on at maximum brightness"
    expected: "Stable light output"
  - step: 4
    action: "Record lumen reading after 30 second stabilization"
    expected: ">= 500 lumens"
    acceptance: "Pass if >= 500 lumens"
acceptance_criteria:
  - "Light output >= 500 lumens"
  - "No visible flicker"
priority: critical
status: approved
links:
  verifies:
    - REQ-01KCA...
```

### View Tests

```bash
tdt test list
```

---

## 12. Test Results

Record the outcomes of test execution.

### Quick Test Execution with `test run`

The fastest way to record test execution is with `tdt test run`:

```bash
# Execute a test and record the result
tdt test run TEST@1 --verdict pass
```

Output:
```
✓ Created result RSLT@1 for test TEST@1 "Light Output Verification"
   Verdict: pass
   Executed by: Your Name
   Steps scaffolded: 4
   verification/results/RSLT-01KCA...tdt.yaml
```

The `test run` command:
- Creates a linked result entity automatically
- Scaffolds step results from the test's procedure steps
- Sets execution date and author automatically
- Prompts for verdict if not provided

```bash
# Run test with notes
tdt test run TEST@2 --verdict pass --notes "All parameters within spec"

# Run test and open editor for full details
tdt test run TEST@3 --verdict fail --edit

# Interactive mode (prompts for verdict)
tdt test run TEST@4
```

### Create Test Results Manually

For more control, create results directly:

```bash
# Passing result
tdt rslt new --test TEST@1 --verdict pass \
  --title "Light Output Test - Unit SN001" --no-edit

# Another passing result
tdt rslt new --test TEST@2 --verdict pass \
  --title "Battery Life Test - Unit SN001" --no-edit

# Failing result (for demonstration)
tdt rslt new --test TEST@3 --verdict fail \
  --title "Water Resistance Test - Unit SN001" --no-edit
```

### Add Result Details

```bash
tdt rslt edit RSLT@1
```

```yaml
id: RSLT-01KCA...
test_id: TEST-01KCA...
title: Light Output Test - Unit SN001
verdict: pass
executed_by: "Test Engineer"
executed_date: "2024-01-15T10:30:00Z"
measurements:
  - parameter: "Light Output"
    value: 523
    unit: "lumens"
    pass: true
  - parameter: "Color Temperature"
    value: 5800
    unit: "K"
    pass: true
notes: |
  Unit exceeded minimum requirement by 4.6%.
  Consistent with expected LED module performance.
status: approved
```

### View Results

```bash
tdt rslt list
```

```
SHORT    TEST     VERDICT      STATUS     AUTHOR
------------------------------------------------
RSLT@1   TEST@1   pass         approved   Test Engineer
RSLT@2   TEST@2   pass         draft      Test Engineer
RSLT@3   TEST@3   fail         draft      Test Engineer
```

### Test Execution Summary

Get aggregate statistics on test execution:

```bash
tdt rslt summary
```

```
Test Results Summary
────────────────────
Total Results: 3
  Pass:        2 (66.7%)
  Fail:        1 (33.3%)
  Conditional: 0 (0.0%)
  Incomplete:  0 (0.0%)

Recent Failures:
  RSLT@3  TEST@3 "Water Resistance Test"  2024-01-15

Requirement Coverage: 75.0% (3/4 requirements have passing tests)
```

Use `--detailed` for breakdown by test type:

```bash
tdt rslt summary --detailed
```

---

## 13. Assemblies

Define how components come together.

### Create Assembly with BOM

```bash
# Create assembly with BOM items in one command
tdt asm new -p "FL-ASM-001" -T "LED Flashlight Assembly" \
  --bom "CMP@1:1,CMP@2:1,CMP@3:1,CMP@4:1,CMP@5:1,CMP@6:1" -n

# Or create empty and add components later
tdt asm new -p "FL-ASM-001" -T "LED Flashlight Assembly" -n
```

### Add Components to BOM

```bash
# Add multiple components at once (ID:QTY format)
tdt asm add ASM@1 CMP@1:1 CMP@2:1 CMP@3:2 CMP@4:1

# Add single component with details
tdt asm add ASM@1 CMP@5 --qty 2 -r "U1,U2" --notes "Apply thread locker"
```

> **Tip**: Use `ID:QTY` pairs for quick bulk additions, or flags for detailed single-component entries.

### Edit BOM Details

```bash
tdt asm edit ASM@1
```

```yaml
id: ASM-01KCA...
part_number: FL-ASM-001
title: LED Flashlight Assembly
description: |
  Complete LED flashlight assembly, ready for packaging.
bom:
  - component_id: CMP-01KCA...  # Housing
    quantity: 1
    reference_designators: ["1"]
  - component_id: CMP-01KCA...  # LED Module
    quantity: 1
    reference_designators: ["2"]
  - component_id: CMP-01KCA...  # O-Ring
    quantity: 2
    reference_designators: ["5A", "5B"]
    notes: "Lubricate with silicone grease"
revision: "A"
status: draft
```

### View BOM

```bash
tdt asm bom ASM@1
```

Shows indented BOM with costs and masses rolled up.

### Calculate Assembly Cost

```bash
# Basic cost calculation
tdt asm cost ASM@1

# With line-by-line breakdown
tdt asm cost ASM@1 --breakdown

# Cost for producing 1000 units (uses price breaks)
tdt asm cost ASM@1 --breakdown --qty 1000
```

The `--qty` parameter specifies how many assemblies you're building. TDT multiplies each BOM quantity by the production quantity to determine purchase quantities, then looks up the appropriate price break from each component's selected quote.

Example output with `--breakdown`:

```
Assembly: LED Flashlight Assembly
Part Number: FL-ASM-001
Production Qty: 1000

ID         TITLE                      QTY   UNIT       LINE       SOURCE
---------------------------------------------------------------------------
CMP@1      Flashlight Housing         1     $12.50     $12.50     unit_cost
CMP@2      LED Module                 1     $3.50      $3.50      quote@1000
CMP@3      Battery Holder             1     $0.75      $0.75      quote@1000
CMP@4      Optical Lens               1     $2.00      $2.00      unit_cost
CMP@5      Housing O-Ring             2     $0.15      $0.30      quote@2000
CMP@6      End Cap                    1     $4.50      $4.50      unit_cost
---------------------------------------------------------------------------
Total Cost: $23.55

Note: Some components have quotes but no selected quote:
   • Optical Lens (2 quotes) - use: tdt cmp set-quote CMP@4 <quote-id>
   Run 'tdt quote compare <component>' to see available quotes
```

**Price source column:**
- `quote@N` - Using selected quote's price for purchase quantity N
- `unit_cost` - Using component's manual unit_cost field
- `none` - No pricing available

> **Tip**: If you see components with quotes but no selected quote, TDT reminds you to set one using `tdt cmp set-quote`.

### Calculate Assembly Mass

```bash
tdt asm mass ASM@1
```

---

## 14. Handling Issues (NCRs & CAPAs)

When things go wrong, track nonconformances and corrective actions.

### Create an NCR

When the water resistance test failed:

```bash
tdt ncr new --title "Water Ingress at O-Ring Seal" \
  --type internal --severity major --no-edit
```

Edit to add details:

```bash
tdt ncr edit NCR@1
```

```yaml
id: NCR-01KCA...
title: Water Ingress at O-Ring Seal
type: internal
severity: major
description: |
  During water resistance testing (TEST@3), water ingress
  observed at end cap O-ring seal after 30 minutes
  submersion at 1 meter depth.
immediate_action: |
  Quarantined test unit SN001.
  Halted further water resistance testing.
root_cause: |
  O-ring groove depth undersized by 0.15mm, preventing
  proper O-ring compression. Caused by tool wear on
  CNC lathe.
disposition: rework
status: open
links:
  from_result:
    - RSLT-01KCA...  # The failing test result
  component:
    - CMP-01KCA...   # End Cap
```

### Link NCR to Test Result

```bash
# Link NCR to the failing test result (automatically bidirectional)
tdt link add NCR@1 RSLT@3 from_result
```

This creates both the `from_result` link on the NCR and a `created_ncr` link on the result.

### Close an NCR

When ready to close an NCR with disposition:

```bash
tdt ncr close NCR@1 --disposition rework --rationale "Bore can be re-machined to spec"
```

```
Closing NCR@1 "Water Ingress at O-Ring Seal"
  Current status: open
  Disposition: rework
  Rationale: Bore can be re-machined to spec

✓ NCR closed
```

Available dispositions: `use-as-is`, `rework`, `scrap`, `return`

```bash
# Link to a CAPA when closing
tdt ncr close NCR@1 --disposition rework --capa CAPA@1

# Skip confirmation prompt
tdt ncr close NCR@1 --disposition scrap -y
```

### Create a CAPA

```bash
tdt capa new --title "O-Ring Groove Tool Wear Control" \
  --type corrective --no-edit
```

Edit to add action plan:

```bash
tdt capa edit CAPA@1
```

```yaml
id: CAPA-01KCA...
title: O-Ring Groove Tool Wear Control
type: corrective
description: |
  Implement controls to prevent O-ring groove dimension
  drift due to tool wear.
root_cause_analysis: |
  5-Why Analysis:
  1. Why water ingress? O-ring didn't seal.
  2. Why didn't O-ring seal? Groove too shallow.
  3. Why groove too shallow? Tool worn beyond limit.
  4. Why tool worn? No scheduled replacement.
  5. Why no schedule? Tool life not characterized.
actions:
  - description: "Characterize groove tool life"
    owner: "Manufacturing Engineer"
    due_date: "2024-02-01"
    status: completed
  - description: "Add tool change to PM schedule at 500 parts"
    owner: "Production Supervisor"
    due_date: "2024-02-15"
    status: in_progress
  - description: "Add groove depth to in-process inspection"
    owner: "Quality Engineer"
    due_date: "2024-02-15"
    status: planned
effectiveness_criteria: |
  Zero O-ring seal failures in next 100 units.
capa_status: in_progress
links:
  ncrs:
    - NCR-01KCA...
```

### Link CAPA to NCR

```bash
tdt link add CAPA@1 NCR@1 ncrs
```

### Verify CAPA Effectiveness

Record effectiveness verification when a CAPA is complete:

```bash
tdt capa verify CAPA@1 --result effective --method "Process audit and defect tracking"
```

```
✓ CAPA@1 verified as Effective
  Method: Process audit and defect tracking
  Status: Closed
```

Verification results: `effective`, `partial`, `ineffective`

```bash
# Partial effectiveness - CAPA stays open
tdt capa verify CAPA@1 --result partial --evidence "Defect rate reduced 50%"

# Add detailed evidence
tdt capa verify CAPA@1 --result effective \
  --method "30-day production audit" \
  --evidence "Zero O-ring seal failures in 150 units"
```

---

## 15. Project Status & Validation

### Check Project Status

```bash
tdt status
```

Shows a dashboard with:
- Requirements coverage (verified vs unverified)
- Risk summary (by level, average RPN)
- Test status (pass rate, pending tests)
- Quality metrics (open NCRs, CAPAs)
- BOM summary
- Tolerance analysis results

### Validate Project

```bash
tdt validate
```

Checks:
- YAML syntax validity
- Schema compliance
- Required field presence
- Link integrity (referenced entities exist)

### Check Verification Coverage

```bash
tdt trace coverage
```

```
Verification Coverage Report
════════════════════════════════════════════════════════════

Total requirements:     5
With verification:      4
Without verification:   1

Coverage: 80%

Uncovered Requirements:
────────────────────────────────────────────────────────────
  ○ REQ@5 - LED Selection
```

### Generate Engineering Reports

TDT can generate various engineering reports:

```bash
# Requirements Verification Matrix (RVM)
tdt report rvm

# FMEA report sorted by RPN
tdt report fmea

# Bill of Materials with costs
tdt report bom ASM@1 --with-cost --with-mass

# Test execution status
tdt report test-status

# All open issues (NCRs, CAPAs, failed tests)
tdt report open-issues
```

Export reports to files:

```bash
# Save RVM as markdown
tdt report rvm --output rvm-report.md

# Export FMEA to CSV
tdt report fmea --format csv --output fmea.csv
```

---

## 16. Traceability

TDT maintains full traceability across all entities through explicit links.

### Creating Links

Links connect entities to establish traceability:

```bash
# Basic syntax: source, target, link_type
tdt link add REQ@1 TEST@1 verified_by
```

**Reciprocal links are added by default.** For `verified_by`, this means TEST@1 automatically gets a `verifies` link back to REQ@1.

```bash
# Skip reciprocal link if needed (one-way only)
tdt link add REQ@1 TEST@1 verified_by --no-reciprocal
```

### Common Link Types

| From | To | Link Type | Reverse Link |
|------|-----|-----------|--------------|
| REQ | TEST | `verified_by` | `verifies` |
| REQ | REQ | `derives_from` | `derived_by` |
| REQ | FEAT | `allocated_to` | `allocated_from` |
| RISK | CMP/PROC/FEAT | `affects` | `risks` |
| NCR | RSLT | `from_result` | `created_ncr` |
| CAPA | NCR | `ncrs` | - |
| CAPA | PROC | `processes_modified` | `modified_by_capa` |

Run `tdt link add --help` to see all available link types with descriptions.

### Viewing Links

```bash
# Show all links for an entity
tdt link show REQ@1

# Show only outgoing links
tdt link show REQ@1 --outgoing

# Show only incoming links
tdt link show REQ@1 --incoming
```

### Removing Links

```bash
tdt link remove REQ@1 TEST@1 verified_by
```

### Finding Broken Links

Check for links that reference non-existent entities:

```bash
# Find broken links
tdt link check

# Find and fix broken links (removes them)
tdt link check --fix
```

### View Traceability Matrix

```bash
# Full matrix
tdt trace matrix

# With short ID aliases for readability
tdt trace matrix --aliases

# Filter by entity type
tdt trace matrix --source-type TEST --target-type REQ
```

### Trace From an Entity

See what depends on a specific entity:

```bash
tdt trace from REQ@1
```

Shows all tests, components, and other entities linked to this requirement.

### Trace To an Entity

See what an entity depends on:

```bash
tdt trace to TEST@1
```

### Find Orphaned Entities

Entities with no links (potential gaps):

```bash
tdt trace orphans
```

### Where-Used Analysis

Find everywhere an entity is referenced:

```bash
# Find all references to a component
tdt where-used CMP@1

# Direct references only (not transitive)
tdt where-used CMP@1 --direct-only
```

Example output:
```
Where CMP@1 (Flashlight Housing) is used:

In Assemblies:
  • ASM@1 - LED Flashlight Assembly (bom item, qty: 1)

In Features:
  • FEAT@1 - LED Mounting Bore (component)
  • FEAT@2 - End Cap Thread (component)

In Processes:
  • PROC@1 - Housing CNC Machining (produces)

In Quotes:
  • QUOT@5 - Housing Quote (component)
```

This is especially useful for impact analysis before making changes.

---

## 17. Version Control Integration

TDT integrates with git to provide entity-level version control commands.

### View Entity History

```bash
# Show commit history for an entity
tdt history REQ@1

# Limit to last 5 commits
tdt history REQ@1 -n 5

# Show full commit messages
tdt history REQ@1 --full

# Show with patches (actual changes)
tdt history REQ@1 --patch

# Filter by date range
tdt history REQ@1 --since 2024-01-01 --until 2024-06-30
```

### View Entity Blame

See who changed each line and when:

```bash
# Full blame
tdt blame REQ@1

# Specific line range
tdt blame REQ@1 --lines 10-20
```

### View Entity Diff

Compare entity changes:

```bash
# Show uncommitted changes
tdt diff REQ@1

# Show staged changes only
tdt diff REQ@1 --staged

# Compare to previous commit
tdt diff REQ@1 HEAD~1

# Compare between revisions
tdt diff REQ@1 v1.0..v2.0

# Show summary stats
tdt diff REQ@1 HEAD~1 --stat
```

---

## 18. Baseline Management

Baselines capture the state of your project at key milestones (design reviews, releases, etc.).

### Create a Baseline

```bash
# Create baseline (validates first)
tdt baseline create v1.0 -m "Initial release candidate"

# Skip validation (not recommended)
tdt baseline create v1.0-draft --skip-validation
```

Baselines are stored as git tags prefixed with `tdt-`.

### List Baselines

```bash
tdt baseline list
```

```
BASELINE        DATE         COMMIT   MESSAGE
-------------------------------------------------
tdt-v1.0        2024-01-15   a1b2c3d  Initial release candidate
tdt-v0.9        2024-01-01   e4f5g6h  Design review baseline
tdt-v0.5        2023-12-01   i7j8k9l  Preliminary design
```

### Compare Baselines

See what changed between baselines:

```bash
tdt baseline compare v0.9 v1.0
```

```
Changes from tdt-v0.9 to tdt-v1.0:

Added (3):
  + REQ@5 - LED Selection (requirements)
  + TEST@4 - Drop Test (tests)
  + CTRL@2 - Thread Depth Check (controls)

Modified (5):
  ~ REQ@1 - Light Output (requirements)
  ~ CMP@1 - Flashlight Housing (components)
  ~ RISK@1 - LED Overheating (risks)
  ~ PROC@1 - Housing CNC Machining (processes)
  ~ ASM@1 - LED Flashlight Assembly (assemblies)

Removed (1):
  - FEAT@7 - Unused Feature (features)
```

Show actual diffs for modified files:

```bash
tdt baseline compare v0.9 v1.0 --diff
```

This displays the git diff for each modified entity, making it easy to see exactly what changed between baselines.

### View Changes Since Baseline

```bash
# What's changed since v1.0?
tdt baseline changed v1.0
```

---

## 19. Bulk Operations & Unix Pipelines

TDT follows Unix philosophy: commands can be chained together using pipes.

### Bulk Status Changes

Update multiple entities at once:

```bash
# Set status by listing IDs
tdt bulk set-status approved REQ@1 REQ@2 REQ@3

# Set status for all entities of a type (with dry-run preview)
tdt bulk set-status review -t req --dry-run
```

### Unix Pipeline Integration

The real power comes from piping list output into bulk commands:

```bash
# Approve all draft requirements
tdt req list --status draft --format id | tdt bulk set-status approved

# Tag all high-priority risks
tdt risk list --level high --format id | tdt bulk add-tag urgent

# Set author on unverified requirements
tdt req list --unverified --format id | tdt bulk set-author "Jane Doe"

# Combine with standard Unix tools
tdt req list --format id | grep "input" | head -10 | tdt bulk add-tag "phase-1"
```

### Batch Tagging

Manage tags across multiple entities:

```bash
# Add release tag to approved components
tdt cmp list --status approved --format id | tdt bulk add-tag "v1.0"

# Remove obsolete tag
tdt bulk remove-tag "deprecated" -t cmp --all

# Tag failed test results for review
tdt rslt list --verdict fail --format id | tdt bulk add-tag needs-investigation
```

### Dry Run Mode

Always preview changes before applying:

```bash
# See what would change without modifying files
tdt req list --format id | tdt bulk set-status review --dry-run
```

This outputs which entities would be affected, letting you verify before committing.

---

## Summary

You've now created a complete TDT project with:

| Entity Type | Purpose | Commands |
|-------------|---------|----------|
| **Requirements** | What the product must do | `tdt req` |
| **Components** | Parts (make/buy) | `tdt cmp` |
| **Suppliers** | Vendor information | `tdt sup` |
| **Quotes** | Pricing and lead times | `tdt quote` |
| **Features** | Geometric characteristics | `tdt feat` |
| **Mates** | Feature interactions | `tdt mate` |
| **Stackups** | Tolerance accumulation | `tdt tol` |
| **Risks** | FMEA (design/process) | `tdt risk` |
| **Processes** | Manufacturing steps | `tdt proc` |
| **Controls** | Process controls | `tdt ctrl` |
| **Tests** | Verification protocols | `tdt test` |
| **Results** | Test outcomes | `tdt rslt` |
| **Assemblies** | BOM structure | `tdt asm` |
| **NCRs** | Nonconformances | `tdt ncr` |
| **CAPAs** | Corrective actions | `tdt capa` |

### Key Workflows

1. **Requirements → Tests → Results**: Verify requirements are met
2. **Components → Features → Mates → Stackups**: Ensure parts fit
3. **Suppliers → Quotes → Components → Assemblies**: Track pricing through BOM
4. **Risks → Mitigations → Controls**: Manage product/process risks
5. **NCRs → CAPAs**: Handle quality issues systematically

### Tips

- Use `--no-edit` for scripted creation, omit it to open editor immediately
- Use short IDs (`REQ@1`, `CMP@2`) for quick reference
- Use `tdt link add` to create traceability between entities
- Run `tdt validate` regularly to catch errors early
- Use `tdt status` for a project health overview

---

## Next Steps

- Explore individual entity documentation in the docs folder
- Set up your own project structure
- Integrate with your existing PLM/QMS systems via JSON/YAML export
- Use `tdt report` commands for generating documentation

For more information, visit [tessera.dev](https://tessera.dev)
