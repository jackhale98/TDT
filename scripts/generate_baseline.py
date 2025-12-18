#!/usr/bin/env python3
"""
Generate a realistic baseline TDT project for testing and demos.

Usage:
    python generate_baseline.py [output_dir]

This creates CSV files that can be imported with:
    tdt import req requirements.csv
    tdt import cmp components.csv
    etc.
"""

import csv
import os
import sys
import random
from pathlib import Path

# Project: Industrial Linear Actuator
# A realistic electromechanical product with ~50 requirements, ~30 components, etc.

OUTPUT_DIR = sys.argv[1] if len(sys.argv) > 1 else "baseline_csvs"

# =============================================================================
# REQUIREMENTS
# =============================================================================

REQUIREMENTS = [
    # Performance Requirements
    {"title": "Stroke Length", "type": "input", "priority": "critical", "status": "approved",
     "text": "The actuator shall have a stroke length of 150mm ± 1mm", "category": "performance",
     "rationale": "Required for full range of motion in target application", "tags": "mechanical,critical"},
    {"title": "Maximum Force", "type": "input", "priority": "critical", "status": "approved",
     "text": "The actuator shall produce a minimum of 500N continuous force", "category": "performance",
     "rationale": "Load requirements from customer specification", "tags": "mechanical,force"},
    {"title": "Speed Range", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall achieve speeds from 5mm/s to 50mm/s", "category": "performance",
     "rationale": "Variable speed required for different operating modes", "tags": "mechanical,speed"},
    {"title": "Positioning Accuracy", "type": "input", "priority": "high", "status": "approved",
     "text": "Position repeatability shall be ±0.1mm", "category": "performance",
     "rationale": "Precision positioning required for automation application", "tags": "mechanical,precision"},
    {"title": "Duty Cycle", "type": "input", "priority": "medium", "status": "approved",
     "text": "The actuator shall operate at 25% duty cycle minimum", "category": "performance",
     "rationale": "Industrial application requires sustained operation", "tags": "electrical,thermal"},

    # Environmental Requirements
    {"title": "Operating Temperature", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall operate from -20°C to +50°C ambient", "category": "environmental",
     "rationale": "Industrial environment temperature range", "tags": "environmental,thermal"},
    {"title": "IP Rating", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall meet IP65 ingress protection", "category": "environmental",
     "rationale": "Protection against dust and water jets required", "tags": "environmental,sealing"},
    {"title": "Vibration Resistance", "type": "input", "priority": "medium", "status": "approved",
     "text": "The actuator shall withstand 2G vibration 10-500Hz", "category": "environmental",
     "rationale": "Mounted on vibrating machinery", "tags": "environmental,mechanical"},
    {"title": "EMC Compliance", "type": "input", "priority": "medium", "status": "approved",
     "text": "The actuator shall comply with EN 61000-6-2 and EN 61000-6-4", "category": "environmental",
     "rationale": "Required for CE marking", "tags": "electrical,regulatory"},

    # Electrical Requirements
    {"title": "Input Voltage", "type": "input", "priority": "critical", "status": "approved",
     "text": "The actuator shall operate from 24VDC ±10%", "category": "electrical",
     "rationale": "Standard industrial control voltage", "tags": "electrical,power"},
    {"title": "Power Consumption", "type": "input", "priority": "medium", "status": "approved",
     "text": "Maximum power consumption shall not exceed 150W", "category": "electrical",
     "rationale": "Power budget constraint from system design", "tags": "electrical,power"},
    {"title": "Control Interface", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall provide RS-485 Modbus RTU interface", "category": "electrical",
     "rationale": "Integration with industrial PLCs", "tags": "electrical,interface"},
    {"title": "Feedback Signal", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall provide analog 0-10V position feedback", "category": "electrical",
     "rationale": "Closed-loop position control requirement", "tags": "electrical,feedback"},

    # Mechanical Requirements
    {"title": "Mounting Interface", "type": "input", "priority": "medium", "status": "approved",
     "text": "The actuator shall have ISO 15552 compliant mounting", "category": "mechanical",
     "rationale": "Standard mounting for easy integration", "tags": "mechanical,interface"},
    {"title": "Rod End Connection", "type": "input", "priority": "medium", "status": "approved",
     "text": "The actuator rod shall have M10x1.25 male thread", "category": "mechanical",
     "rationale": "Standard thread for load attachment", "tags": "mechanical,interface"},
    {"title": "Weight Limit", "type": "input", "priority": "low", "status": "approved",
     "text": "Total actuator weight shall not exceed 3.5kg", "category": "mechanical",
     "rationale": "Installation handling requirement", "tags": "mechanical,weight"},

    # Safety Requirements
    {"title": "Overload Protection", "type": "input", "priority": "critical", "status": "approved",
     "text": "The actuator shall detect and respond to overload within 100ms", "category": "safety",
     "rationale": "Prevent damage from obstruction or jamming", "tags": "safety,protection"},
    {"title": "Limit Switches", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall have adjustable end-of-travel limits", "category": "safety",
     "rationale": "Prevent mechanical over-travel damage", "tags": "safety,mechanical"},
    {"title": "Manual Override", "type": "input", "priority": "medium", "status": "approved",
     "text": "The actuator shall allow manual movement when unpowered", "category": "safety",
     "rationale": "Emergency release capability", "tags": "safety,manual"},

    # Reliability Requirements
    {"title": "Design Life", "type": "input", "priority": "high", "status": "approved",
     "text": "The actuator shall achieve 1 million full stroke cycles minimum", "category": "reliability",
     "rationale": "5-year service life at expected usage rate", "tags": "reliability,life"},
    {"title": "MTBF", "type": "input", "priority": "medium", "status": "approved",
     "text": "MTBF shall exceed 50,000 operating hours", "category": "reliability",
     "rationale": "Industrial reliability requirement", "tags": "reliability,mtbf"},

    # Derived Requirements
    {"title": "Motor Selection", "type": "output", "priority": "high", "status": "approved",
     "text": "Motor shall be NEMA 23 brushless DC, minimum 0.5Nm continuous torque", "category": "electrical",
     "rationale": "Derived from force and speed requirements", "tags": "electrical,motor"},
    {"title": "Lead Screw Pitch", "type": "output", "priority": "high", "status": "approved",
     "text": "Lead screw pitch shall be 5mm for optimal speed/force tradeoff", "category": "mechanical",
     "rationale": "Calculated from speed and force requirements", "tags": "mechanical,drivetrain"},
    {"title": "Bearing Selection", "type": "output", "priority": "medium", "status": "approved",
     "text": "Support bearings shall be angular contact with C3 clearance", "category": "mechanical",
     "rationale": "Required for axial load and temperature range", "tags": "mechanical,bearings"},
    {"title": "Seal Design", "type": "output", "priority": "high", "status": "approved",
     "text": "Rod seal shall be double-lip NBR with dust wiper", "category": "mechanical",
     "rationale": "Required for IP65 rating at operating temperature", "tags": "mechanical,sealing"},
]

# =============================================================================
# COMPONENTS
# =============================================================================

COMPONENTS = [
    # Make parts - mechanical
    {"part_number": "LA-HSG-001", "title": "Main Housing", "make_buy": "make", "category": "mechanical",
     "description": "Extruded aluminum housing with machined features", "material": "6063-T6 Aluminum",
     "finish": "Clear anodize", "mass": 0.850, "cost": 45.00, "tags": "structural,machined"},
    {"part_number": "LA-CAP-001", "title": "Front End Cap", "make_buy": "make", "category": "mechanical",
     "description": "Machined end cap with seal groove and bearing bore", "material": "6061-T6 Aluminum",
     "finish": "Clear anodize", "mass": 0.120, "cost": 18.00, "tags": "structural,machined"},
    {"part_number": "LA-CAP-002", "title": "Rear End Cap", "make_buy": "make", "category": "mechanical",
     "description": "Machined end cap with motor mount and bearing bore", "material": "6061-T6 Aluminum",
     "finish": "Clear anodize", "mass": 0.180, "cost": 22.00, "tags": "structural,machined"},
    {"part_number": "LA-ROD-001", "title": "Extension Rod", "make_buy": "make", "category": "mechanical",
     "description": "Ground and chrome plated piston rod", "material": "1045 Steel",
     "finish": "Hard chrome", "mass": 0.340, "cost": 35.00, "tags": "precision,ground"},
    {"part_number": "LA-NUT-001", "title": "Lead Screw Nut", "make_buy": "make", "category": "mechanical",
     "description": "Bronze lead screw nut with anti-backlash feature", "material": "C93200 Bronze",
     "finish": "As machined", "mass": 0.085, "cost": 28.00, "tags": "precision,wear"},

    # Buy parts - mechanical
    {"part_number": "LA-SCR-001", "title": "Lead Screw", "make_buy": "buy", "category": "mechanical",
     "description": "Precision ground lead screw Tr16x5", "material": "1045 Steel hardened",
     "finish": "Black oxide", "mass": 0.420, "cost": 65.00, "tags": "precision,drivetrain"},
    {"part_number": "LA-BRG-001", "title": "Front Bearing", "make_buy": "buy", "category": "mechanical",
     "description": "Angular contact bearing 6002-2RS", "material": "52100 Steel",
     "finish": "Standard", "mass": 0.032, "cost": 8.50, "tags": "bearing,precision"},
    {"part_number": "LA-BRG-002", "title": "Rear Bearing", "make_buy": "buy", "category": "mechanical",
     "description": "Deep groove bearing 6003-2RS", "material": "52100 Steel",
     "finish": "Standard", "mass": 0.042, "cost": 6.50, "tags": "bearing,support"},
    {"part_number": "LA-SEL-001", "title": "Rod Seal", "make_buy": "buy", "category": "mechanical",
     "description": "Double-lip rod seal 16x24x7", "material": "NBR rubber",
     "finish": "Standard", "mass": 0.008, "cost": 3.25, "tags": "seal,wear"},
    {"part_number": "LA-SEL-002", "title": "Dust Wiper", "make_buy": "buy", "category": "mechanical",
     "description": "Polyurethane dust wiper 16x22x4", "material": "Polyurethane",
     "finish": "Standard", "mass": 0.004, "cost": 1.85, "tags": "seal,protection"},
    {"part_number": "LA-ORI-001", "title": "End Cap O-Ring", "make_buy": "buy", "category": "mechanical",
     "description": "Static O-ring 45x3.5 NBR 70A", "material": "NBR rubber",
     "finish": "Standard", "mass": 0.006, "cost": 0.45, "tags": "seal,static"},

    # Buy parts - electrical
    {"part_number": "LA-MOT-001", "title": "BLDC Motor", "make_buy": "buy", "category": "electrical",
     "description": "NEMA 23 brushless DC motor 24V 0.6Nm", "material": "Various",
     "finish": "Black powder coat", "mass": 0.580, "cost": 85.00, "tags": "motor,drivetrain"},
    {"part_number": "LA-ENC-001", "title": "Rotary Encoder", "make_buy": "buy", "category": "electrical",
     "description": "Incremental encoder 1000 PPR", "material": "Various",
     "finish": "Standard", "mass": 0.045, "cost": 28.00, "tags": "sensor,feedback"},
    {"part_number": "LA-DRV-001", "title": "Motor Driver", "make_buy": "buy", "category": "electrical",
     "description": "BLDC motor driver module 24V 10A", "material": "PCB assembly",
     "finish": "Conformal coat", "mass": 0.065, "cost": 42.00, "tags": "electronics,control"},
    {"part_number": "LA-LIM-001", "title": "Limit Switch", "make_buy": "buy", "category": "electrical",
     "description": "Micro limit switch with lever", "material": "Various",
     "finish": "Standard", "mass": 0.012, "cost": 2.80, "tags": "sensor,safety"},
    {"part_number": "LA-CON-001", "title": "Power Connector", "make_buy": "buy", "category": "electrical",
     "description": "M12 4-pin power connector IP67", "material": "Brass/plastic",
     "finish": "Nickel plate", "mass": 0.025, "cost": 8.50, "tags": "connector,interface"},
    {"part_number": "LA-CON-002", "title": "Signal Connector", "make_buy": "buy", "category": "electrical",
     "description": "M12 8-pin signal connector IP67", "material": "Brass/plastic",
     "finish": "Nickel plate", "mass": 0.028, "cost": 12.00, "tags": "connector,interface"},

    # Fasteners and consumables
    {"part_number": "LA-FST-001", "title": "End Cap Screws", "make_buy": "buy", "category": "fastener",
     "description": "M4x12 socket head cap screw A2-70", "material": "Stainless steel",
     "finish": "Passivated", "mass": 0.003, "cost": 0.08, "tags": "fastener,assembly"},
    {"part_number": "LA-FST-002", "title": "Motor Mount Screws", "make_buy": "buy", "category": "fastener",
     "description": "M3x8 socket head cap screw A2-70", "material": "Stainless steel",
     "finish": "Passivated", "mass": 0.002, "cost": 0.06, "tags": "fastener,assembly"},
    {"part_number": "LA-CON-003", "title": "Thread Locker", "make_buy": "buy", "category": "consumable",
     "description": "Loctite 243 medium strength", "material": "Anaerobic adhesive",
     "finish": "N/A", "mass": 0.001, "cost": 0.15, "tags": "consumable,assembly"},
]

# =============================================================================
# SUPPLIERS
# =============================================================================

SUPPLIERS = [
    {"name": "Precision Motion Systems", "short_name": "PMS", "category": "drivetrain",
     "contact_name": "Mike Chen", "contact_email": "mchen@precisionmotion.example",
     "contact_phone": "+1-555-0101", "website": "https://precisionmotion.example",
     "tags": "motors,screws,bearings"},
    {"name": "Allied Sealing Technologies", "short_name": "AST", "category": "sealing",
     "contact_name": "Sarah Johnson", "contact_email": "sjohnson@alliedsealing.example",
     "contact_phone": "+1-555-0102", "website": "https://alliedsealing.example",
     "tags": "seals,orings"},
    {"name": "Global Electronics Supply", "short_name": "GES", "category": "electronics",
     "contact_name": "David Park", "contact_email": "dpark@globalelec.example",
     "contact_phone": "+1-555-0103", "website": "https://globalelec.example",
     "tags": "electronics,connectors,sensors"},
    {"name": "MetalWorks CNC", "short_name": "MWCNC", "category": "machining",
     "contact_name": "Tom Williams", "contact_email": "twilliams@metalworkscnc.example",
     "contact_phone": "+1-555-0104", "website": "https://metalworkscnc.example",
     "tags": "machining,make"},
    {"name": "FastenerWorld", "short_name": "FW", "category": "fasteners",
     "contact_name": "Lisa Brown", "contact_email": "lbrown@fastenerworld.example",
     "contact_phone": "+1-555-0105", "website": "https://fastenerworld.example",
     "tags": "fasteners,hardware"},
]

# =============================================================================
# RISKS
# =============================================================================

RISKS = [
    # Design Risks
    {"title": "Motor Overheating", "type": "design", "category": "thermal",
     "description": "Motor may overheat under continuous high-load operation",
     "failure_mode": "Thermal shutdown or winding damage during extended operation",
     "cause": "Insufficient heat dissipation path from motor to housing",
     "effect": "System shutdown, potential motor damage, warranty returns",
     "severity": 7, "occurrence": 4, "detection": 5, "tags": "thermal,motor"},
    {"title": "Lead Screw Wear", "type": "design", "category": "wear",
     "description": "Accelerated wear on lead screw nut interface",
     "failure_mode": "Excessive backlash and positioning error over time",
     "cause": "Inadequate lubrication or contamination ingress",
     "effect": "Degraded positioning accuracy, shortened service life",
     "severity": 6, "occurrence": 5, "detection": 6, "tags": "wear,drivetrain"},
    {"title": "Seal Failure", "type": "design", "category": "sealing",
     "description": "Rod seal may fail under extreme temperature cycling",
     "failure_mode": "Seal extrusion or hardening leading to leakage",
     "cause": "Temperature cycling beyond seal material limits",
     "effect": "Loss of IP65 rating, contamination ingress",
     "severity": 8, "occurrence": 3, "detection": 4, "tags": "sealing,environmental"},
    {"title": "Encoder Miscounting", "type": "design", "category": "electrical",
     "description": "Encoder may miscount under EMI conditions",
     "failure_mode": "Position feedback errors and drift",
     "cause": "Insufficient EMI shielding on encoder signals",
     "effect": "Positioning errors, potential safety issue",
     "severity": 7, "occurrence": 3, "detection": 5, "tags": "electrical,emc"},
    {"title": "Bearing Preload Loss", "type": "design", "category": "mechanical",
     "description": "Angular contact bearing preload may change with temperature",
     "failure_mode": "Increased axial play or excessive preload",
     "cause": "Differential thermal expansion in bearing assembly",
     "effect": "Reduced life, noise, or binding",
     "severity": 5, "occurrence": 4, "detection": 6, "tags": "mechanical,bearings"},

    # Process Risks
    {"title": "Housing Bore Tolerance", "type": "process", "category": "machining",
     "description": "Housing bore may go out of tolerance",
     "failure_mode": "Bore diameter or concentricity out of specification",
     "cause": "Tool wear, thermal growth, or setup error",
     "effect": "Bearing fit issues, assembly problems",
     "severity": 6, "occurrence": 4, "detection": 3, "tags": "machining,dimensional"},
    {"title": "Anodize Thickness Variation", "type": "process", "category": "finishing",
     "description": "Anodize coating may have thickness variation",
     "failure_mode": "Uneven or out-of-spec coating thickness",
     "cause": "Bath chemistry variation or rack positioning",
     "effect": "Fit issues with bearings, cosmetic defects",
     "severity": 4, "occurrence": 5, "detection": 4, "tags": "finishing,surface"},
    {"title": "Wrong Fastener Torque", "type": "process", "category": "assembly",
     "description": "Fasteners may be under or over-torqued",
     "failure_mode": "Loose or stripped fasteners",
     "cause": "Operator error or uncalibrated tools",
     "effect": "Assembly loosening in service or stripped threads",
     "severity": 6, "occurrence": 4, "detection": 5, "tags": "assembly,fastener"},
    {"title": "Seal Installation Damage", "type": "process", "category": "assembly",
     "description": "Seals may be damaged during installation",
     "failure_mode": "Cut, twisted, or improperly seated seal",
     "cause": "Sharp edges, improper technique, or missing lubrication",
     "effect": "Immediate or premature seal failure",
     "severity": 7, "occurrence": 4, "detection": 5, "tags": "assembly,sealing"},
    {"title": "Motor-Screw Misalignment", "type": "process", "category": "assembly",
     "description": "Motor shaft may be misaligned with lead screw",
     "failure_mode": "Angular or parallel misalignment",
     "cause": "Tolerance stackup or coupling installation error",
     "effect": "Vibration, noise, reduced bearing life",
     "severity": 5, "occurrence": 5, "detection": 4, "tags": "assembly,alignment"},
]

# =============================================================================
# TESTS
# =============================================================================

TESTS = [
    # Verification tests
    {"title": "Stroke Length Verification", "type": "verification", "level": "system", "method": "test",
     "category": "dimensional", "priority": "critical",
     "objective": "Verify actuator achieves specified stroke length",
     "description": "Measure full stroke extension and retraction using calibrated linear scale",
     "estimated_duration": "15 min", "tags": "dimensional,critical"},
    {"title": "Force Output Test", "type": "verification", "level": "system", "method": "test",
     "category": "performance", "priority": "critical",
     "objective": "Verify actuator produces specified continuous force",
     "description": "Apply increasing load via dynamometer until stall, measure continuous force capability",
     "estimated_duration": "30 min", "tags": "force,performance"},
    {"title": "Speed Range Verification", "type": "verification", "level": "system", "method": "test",
     "category": "performance", "priority": "high",
     "objective": "Verify actuator speed range meets specification",
     "description": "Measure extension/retraction speed at min and max settings using high-speed camera or laser",
     "estimated_duration": "20 min", "tags": "speed,performance"},
    {"title": "Position Repeatability Test", "type": "verification", "level": "system", "method": "test",
     "category": "performance", "priority": "high",
     "objective": "Verify position repeatability specification",
     "description": "Command 10 cycles to same position, measure variation with dial indicator",
     "estimated_duration": "45 min", "tags": "precision,performance"},
    {"title": "IP65 Ingress Test", "type": "verification", "level": "system", "method": "test",
     "category": "environmental", "priority": "high",
     "objective": "Verify IP65 dust and water jet protection",
     "description": "Subject to dust chamber test and 6.3mm water jet at 12.5 l/min per IEC 60529",
     "estimated_duration": "4 hr", "tags": "environmental,sealing"},
    {"title": "Temperature Cycling Test", "type": "verification", "level": "system", "method": "test",
     "category": "environmental", "priority": "high",
     "objective": "Verify operation across temperature range",
     "description": "Operate through 10 cycles of -20°C to +50°C with 30 min dwells",
     "estimated_duration": "24 hr", "tags": "environmental,thermal"},
    {"title": "EMC Emissions Test", "type": "verification", "level": "system", "method": "test",
     "category": "electrical", "priority": "medium",
     "objective": "Verify compliance with EN 61000-6-4 emissions limits",
     "description": "Conducted and radiated emissions per EN 55011",
     "estimated_duration": "8 hr", "tags": "emc,regulatory"},
    {"title": "Motor Thermal Test", "type": "verification", "level": "unit", "method": "test",
     "category": "thermal", "priority": "high",
     "objective": "Verify motor temperature rise is acceptable",
     "description": "Run at rated load for 2 hours, monitor winding temperature via resistance",
     "estimated_duration": "3 hr", "tags": "thermal,motor"},

    # Inspections
    {"title": "Housing Dimensional Inspection", "type": "verification", "level": "unit", "method": "inspection",
     "category": "dimensional", "priority": "high",
     "objective": "Verify housing critical dimensions",
     "description": "CMM inspection of bearing bores, seal grooves, and mounting features",
     "estimated_duration": "45 min", "tags": "dimensional,machined"},
    {"title": "First Article Inspection", "type": "verification", "level": "system", "method": "inspection",
     "category": "dimensional", "priority": "critical",
     "objective": "Complete dimensional inspection of first production unit",
     "description": "Full CMM inspection per drawing requirements",
     "estimated_duration": "4 hr", "tags": "fai,dimensional"},

    # Validation tests
    {"title": "Customer Application Trial", "type": "validation", "level": "acceptance", "method": "demonstration",
     "category": "application", "priority": "high",
     "objective": "Validate actuator performance in customer application",
     "description": "Install in customer machine, run typical duty cycle for 1 week",
     "estimated_duration": "168 hr", "tags": "validation,customer"},
    {"title": "Lifecycle Durability Test", "type": "validation", "level": "system", "method": "test",
     "category": "reliability", "priority": "high",
     "objective": "Validate actuator achieves design life cycles",
     "description": "Continuous cycling at rated load until failure or 1M cycles",
     "estimated_duration": "2000 hr", "tags": "reliability,endurance"},
]

# =============================================================================
# CSV GENERATION
# =============================================================================

def write_csv(filename, headers, rows):
    """Write rows to a CSV file with given headers."""
    filepath = Path(OUTPUT_DIR) / filename
    with open(filepath, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=headers, extrasaction='ignore')
        writer.writeheader()
        writer.writerows(rows)
    print(f"  Created {filepath} ({len(rows)} rows)")

def main():
    # Create output directory
    Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)
    print(f"Generating baseline CSVs in {OUTPUT_DIR}/\n")

    # Requirements
    req_headers = ["title", "type", "priority", "status", "text", "rationale", "category", "tags"]
    write_csv("requirements.csv", req_headers, REQUIREMENTS)

    # Components
    cmp_headers = ["part_number", "title", "make_buy", "category", "description", "material", "finish", "mass", "cost", "tags"]
    write_csv("components.csv", cmp_headers, COMPONENTS)

    # Suppliers
    sup_headers = ["name", "short_name", "category", "contact_name", "contact_email", "contact_phone", "website", "tags"]
    write_csv("suppliers.csv", sup_headers, SUPPLIERS)

    # Risks
    risk_headers = ["title", "type", "category", "description", "failure_mode", "cause", "effect", "severity", "occurrence", "detection", "tags"]
    write_csv("risks.csv", risk_headers, RISKS)

    # Tests
    test_headers = ["title", "type", "level", "method", "category", "priority", "objective", "description", "estimated_duration", "tags"]
    write_csv("tests.csv", test_headers, TESTS)

    print(f"""
Import commands:
  cd <your-project>
  tdt import req {OUTPUT_DIR}/requirements.csv
  tdt import cmp {OUTPUT_DIR}/components.csv
  tdt import sup {OUTPUT_DIR}/suppliers.csv
  tdt import risk {OUTPUT_DIR}/risks.csv
  tdt import test {OUTPUT_DIR}/tests.csv

Then add links:
  tdt link add REQ@1 TEST@1 verified_by
  tdt validate --fix  # Auto-calculate RPN values
""")

if __name__ == "__main__":
    main()
