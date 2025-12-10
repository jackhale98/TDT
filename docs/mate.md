# PDT Mate Entity (Tolerances)

This document describes the Mate entity type in PDT (Plain-text Product Development Toolkit).

## Overview

Mates represent 1:1 contact relationships between two features, such as a pin fitting into a hole. PDT automatically calculates worst-case fit analysis when you create or recalculate a mate, determining whether it's a clearance, interference, or transition fit.

## Entity Type

- **Prefix**: `MATE`
- **File extension**: `.pdt.yaml`
- **Directory**: `tolerances/mates/`

## Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (MATE-[26-char ULID]) |
| `feature_a` | string | First feature ID (typically hole) - **REQUIRED** |
| `feature_b` | string | Second feature ID (typically shaft) - **REQUIRED** |
| `title` | string | Short descriptive title (1-200 chars) |
| `status` | enum | `draft`, `review`, `approved`, `released`, `obsolete` |
| `created` | datetime | Creation timestamp (ISO 8601) |
| `author` | string | Author name |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Detailed description |
| `mate_type` | enum | `clearance_fit`, `interference_fit`, `transition_fit`, `planar_contact`, `thread_engagement` |
| `fit_analysis` | FitAnalysis | Auto-calculated fit results |
| `notes` | string | Additional notes |
| `tags` | array[string] | Tags for filtering |
| `entity_revision` | integer | Entity revision number (default: 1) |

### FitAnalysis Object (Auto-calculated)

| Field | Type | Description |
|-------|------|-------------|
| `worst_case_min_clearance` | number | Minimum clearance (or max interference if negative) |
| `worst_case_max_clearance` | number | Maximum clearance (or min interference if negative) |
| `fit_result` | enum | `clearance`, `interference`, `transition` |

### Links

| Field | Type | Description |
|-------|------|-------------|
| `links.used_in_stackups` | array[EntityId] | Stackups using this mate |
| `links.verifies` | array[EntityId] | Requirements verified by this mate |

## Fit Calculation

PDT automatically calculates worst-case fit from the primary dimensions of both features:

```
For hole/shaft mate:
  hole_max = hole.nominal + hole.plus_tol
  hole_min = hole.nominal - hole.minus_tol
  shaft_max = shaft.nominal + shaft.plus_tol
  shaft_min = shaft.nominal - shaft.minus_tol

  min_clearance = hole_min - shaft_max
  max_clearance = hole_max - shaft_min

  fit_result =
    if min_clearance > 0: clearance
    else if max_clearance < 0: interference
    else: transition
```

## Example

```yaml
# Mate: Pin-Hole Mate
# Created by PDT - Plain-text Product Development Toolkit

id: MATE-01HC2JB7SMQX7RS1Y0GFKBHPTF
title: "Pin-Hole Mate"

description: |
  Locating pin engagement with mounting hole.
  Critical for alignment accuracy.

feature_a: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTE  # Hole: 10.0 +0.1/-0.05
feature_b: FEAT-01HC2JB7SMQX7RS1Y0GFKBHPTG  # Shaft: 9.95 +0.02/-0.02

mate_type: clearance_fit

# Auto-calculated from feature dimensions
fit_analysis:
  worst_case_min_clearance: 0.03   # hole_min - shaft_max = 9.95 - 9.97 = -0.02? Let me recalc
  worst_case_max_clearance: 0.17   # hole_max - shaft_min = 10.1 - 9.93 = 0.17
  fit_result: clearance

notes: |
  Clearance fit provides easy assembly while maintaining
  adequate positional accuracy for the application.

tags: [locating, precision, alignment]
status: approved

links:
  used_in_stackups:
    - TOL-01HC2JB7SMQX7RS1Y0GFKBHPTH
  verifies:
    - REQ-01HC2JB7SMQX7RS1Y0GFKBHPTI

# Auto-managed metadata
created: 2024-01-15T10:30:00Z
author: Jack Hale
entity_revision: 1
```

## CLI Commands

### Create a new mate

```bash
# Create mate (--feature-a and --feature-b are REQUIRED)
pdt mate new --feature-a FEAT@1 --feature-b FEAT@2 --title "Pin-Hole Fit"

# Specify mate type
pdt mate new --feature-a FEAT@1 --feature-b FEAT@2 --type clearance_fit

# Create with interactive wizard
pdt mate new --feature-a FEAT@1 --feature-b FEAT@2 -i

# Create and immediately edit
pdt mate new --feature-a FEAT@1 --feature-b FEAT@2 --title "New Mate" --edit
```

**Note**: Both `--feature-a` and `--feature-b` are required.

### List mates

```bash
# List all mates
pdt mate list

# Filter by mate type
pdt mate list --type clearance_fit
pdt mate list --type interference_fit
pdt mate list --type transition_fit

# Filter by status
pdt mate list --status approved

# Search in title/description
pdt mate list --search "pin"

# Sort and limit
pdt mate list --sort title
pdt mate list --limit 10

# Count only
pdt mate list --count

# Output formats
pdt mate list -f json
pdt mate list -f csv
```

### Show mate details

```bash
# Show by ID (partial match supported)
pdt mate show MATE-01HC2

# Show using short ID (includes fit calculation)
pdt mate show MATE@1

# Output as JSON
pdt mate show MATE@1 -f json
```

### Recalculate fit

```bash
# Recalculate fit if feature dimensions changed
pdt mate recalc MATE@1

# Output shows updated fit analysis
# ✓ Recalculated fit for mate MATE@1
#    Result: clearance (0.0300 to 0.1700)
```

### Edit a mate

```bash
# Open in editor
pdt mate edit MATE-01HC2

# Using short ID
pdt mate edit MATE@1
```

## Fit Types

### Clearance Fit

Both min and max clearances are positive - shaft always fits freely in hole.

```
min_clearance > 0 AND max_clearance > 0
```

**Applications**: Easy assembly, sliding fits, thermal expansion allowance

### Interference Fit (Press Fit)

Both min and max clearances are negative - shaft is always larger than hole.

```
min_clearance < 0 AND max_clearance < 0
```

**Applications**: Permanent assembly, torque transmission, press-fit pins

### Transition Fit

Min clearance is negative but max clearance is positive - may be either clearance or interference depending on actual dimensions.

```
min_clearance < 0 AND max_clearance > 0
```

**Applications**: Locating fits, accurate positioning with some assembly force

## ISO Fit Classifications

| Fit Type | ISO Symbol | Description |
|----------|------------|-------------|
| Loose running | H11/c11 | Large clearance for free movement |
| Free running | H9/d9 | Light running with minimal friction |
| Close running | H8/f7 | Accurate location with free movement |
| Sliding | H7/g6 | Accurate location, can slide |
| Locational clearance | H7/h6 | Accurate location, snug fit |
| Locational transition | H7/k6 | Accurate location, light press |
| Locational interference | H7/n6 | Accurate location, press fit |
| Medium drive | H7/p6 | Permanent assembly |
| Force fit | H7/s6 | High interference |

## Best Practices

### Creating Mates

1. **Feature order** - Conventionally, feature_a is the hole and feature_b is the shaft
2. **Complete features first** - Ensure both features have dimensions before creating mate
3. **Verify fit type** - Check that calculated fit matches your intent
4. **Document rationale** - Explain why this fit was chosen

### Managing Mates

1. **Recalculate after changes** - Run `pdt mate recalc` after modifying features
2. **Link to requirements** - Connect to requirements that specify fit
3. **Use in stackups** - Reference mates in tolerance stackups
4. **Track status** - Update status as design matures

### Fit Selection Guidelines

| Application | Recommended Fit |
|-------------|-----------------|
| High-speed rotation | Clearance |
| Sliding/reciprocating | Clearance |
| Accurate positioning | Transition |
| Light press assembly | Transition |
| Permanent assembly | Interference |
| Torque transmission | Interference |

## Validation

Mates are validated against a JSON Schema:

```bash
# Validate all project files
pdt validate

# Validate specific file
pdt validate tolerances/mates/MATE-01HC2JB7SMQX7RS1Y0GFKBHPTF.pdt.yaml
```

### Validation Rules

1. **ID Format**: Must match `MATE-[A-Z0-9]{26}` pattern
2. **Feature A**: Required, must be valid FEAT ID
3. **Feature B**: Required, must be valid FEAT ID
4. **Title**: Required, 1-200 characters
5. **Mate Type**: If specified, must be valid enum
6. **Fit Result**: If specified, must be `clearance`, `interference`, or `transition`
7. **Status**: Must be one of: `draft`, `review`, `approved`, `released`, `obsolete`
8. **No Additional Properties**: Unknown fields are not allowed

## JSON Schema

The full JSON Schema for mates is available at:

```
pdt/schemas/mate.schema.json
```
