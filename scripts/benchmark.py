#!/usr/bin/env python3
"""
TDT Performance Benchmark

Generates large datasets and times various operations.
"""

import csv
import os
import subprocess
import sys
import time
import random
import tempfile
import shutil
from pathlib import Path

# Configuration - adjust these for different test sizes
NUM_REQUIREMENTS = 500
NUM_COMPONENTS = 200
NUM_RISKS = 100
NUM_TESTS = 150
NUM_SUPPLIERS = 20

# Resolve to absolute path before we change directories
TDT_BIN = str(Path(sys.argv[1]).resolve()) if len(sys.argv) > 1 else "tdt"

# Sample data pools for realistic generation
CATEGORIES = ["performance", "safety", "environmental", "electrical", "mechanical", "thermal", "reliability", "interface"]
PRIORITIES = ["critical", "high", "medium", "low"]
STATUSES = ["draft", "approved", "review"]
REQ_TYPES = ["input", "output"]
RISK_TYPES = ["design", "process"]
TEST_TYPES = ["verification", "validation"]
TEST_LEVELS = ["unit", "integration", "system", "acceptance"]
TEST_METHODS = ["test", "inspection", "analysis", "demonstration"]
MAKE_BUY = ["make", "buy", "buy", "buy"]  # Weighted toward buy
CMP_CATEGORIES = ["mechanical", "electrical", "fastener", "consumable"]

ADJECTIVES = ["Primary", "Secondary", "Auxiliary", "Main", "Critical", "Standard", "Enhanced", "Advanced", "Basic", "Core"]
NOUNS_REQ = ["Temperature", "Pressure", "Speed", "Force", "Voltage", "Current", "Power", "Torque", "Flow", "Position",
             "Accuracy", "Repeatability", "Response", "Bandwidth", "Efficiency", "Life", "Weight", "Size", "Cost", "Noise"]
NOUNS_CMP = ["Housing", "Bracket", "Shaft", "Bearing", "Seal", "Motor", "Sensor", "Controller", "Connector", "Cable",
             "Screw", "Nut", "Washer", "Spring", "Plate", "Cover", "Frame", "Mount", "Clip", "Gasket"]
NOUNS_RISK = ["Failure", "Degradation", "Wear", "Corrosion", "Fatigue", "Overload", "Misalignment", "Contamination",
              "Overheating", "Short Circuit", "Leakage", "Vibration", "Noise", "Drift", "Interference"]

def generate_requirements(n):
    """Generate n requirement rows."""
    rows = []
    for i in range(n):
        adj = random.choice(ADJECTIVES)
        noun = random.choice(NOUNS_REQ)
        cat = random.choice(CATEGORIES)
        rows.append({
            "title": f"{adj} {noun} Requirement {i+1}",
            "type": random.choice(REQ_TYPES),
            "priority": random.choice(PRIORITIES),
            "status": random.choice(STATUSES),
            "category": cat,
            "text": f"The system shall meet {noun.lower()} requirements for {cat} performance.",
            "rationale": f"Required for {cat} compliance and system performance.",
            "tags": f"{cat},{random.choice(PRIORITIES)}"
        })
    return rows

def generate_components(n):
    """Generate n component rows."""
    rows = []
    for i in range(n):
        adj = random.choice(ADJECTIVES)
        noun = random.choice(NOUNS_CMP)
        mb = random.choice(MAKE_BUY)
        cat = random.choice(CMP_CATEGORIES)
        rows.append({
            "part_number": f"PN-{i+1:04d}",
            "title": f"{adj} {noun} {i+1}",
            "make_buy": mb,
            "category": cat,
            "description": f"{adj} {noun} for system assembly",
            "material": "Various",
            "finish": "Standard",
            "mass": round(random.uniform(0.01, 2.0), 3),
            "cost": round(random.uniform(0.50, 150.0), 2),
            "tags": f"{cat},{mb}"
        })
    return rows

def generate_risks(n):
    """Generate n risk rows."""
    rows = []
    for i in range(n):
        noun = random.choice(NOUNS_RISK)
        cat = random.choice(CATEGORIES)
        rtype = random.choice(RISK_TYPES)
        sev = random.randint(1, 10)
        occ = random.randint(1, 10)
        det = random.randint(1, 10)
        rows.append({
            "title": f"{noun} Risk {i+1}",
            "type": rtype,
            "category": cat,
            "description": f"Potential {noun.lower()} in {cat} subsystem",
            "failure_mode": f"{noun} during operation",
            "cause": f"Design or process deficiency in {cat} area",
            "effect": f"System {noun.lower()} leading to performance degradation",
            "severity": sev,
            "occurrence": occ,
            "detection": det,
            "tags": f"{cat},{rtype}"
        })
    return rows

def generate_tests(n):
    """Generate n test rows."""
    rows = []
    for i in range(n):
        adj = random.choice(ADJECTIVES)
        noun = random.choice(NOUNS_REQ)
        cat = random.choice(CATEGORIES)
        ttype = random.choice(TEST_TYPES)
        rows.append({
            "title": f"{adj} {noun} Test {i+1}",
            "type": ttype,
            "level": random.choice(TEST_LEVELS),
            "method": random.choice(TEST_METHODS),
            "category": cat,
            "priority": random.choice(PRIORITIES),
            "objective": f"Verify {noun.lower()} performance meets specification",
            "description": f"Test procedure for {noun.lower()} {cat} requirements",
            "estimated_duration": f"{random.randint(15, 480)} min",
            "tags": f"{cat},{ttype}"
        })
    return rows

def generate_suppliers(n):
    """Generate n supplier rows."""
    rows = []
    for i in range(n):
        rows.append({
            "name": f"Supplier Company {i+1}",
            "short_name": f"SUP{i+1:02d}",
            "category": random.choice(CMP_CATEGORIES),
            "contact_name": f"Contact {i+1}",
            "contact_email": f"contact{i+1}@supplier{i+1}.example",
            "contact_phone": f"+1-555-{i+1:04d}",
            "website": f"https://supplier{i+1}.example",
            "tags": random.choice(CMP_CATEGORIES)
        })
    return rows

def write_csv(filepath, headers, rows):
    """Write rows to CSV."""
    with open(filepath, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=headers, extrasaction='ignore')
        writer.writeheader()
        writer.writerows(rows)

def run_timed(cmd, label):
    """Run command and return timing."""
    start = time.perf_counter()
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    elapsed = time.perf_counter() - start
    success = result.returncode == 0
    return elapsed, success, result.stdout, result.stderr

def main():
    print("=" * 70)
    print("TDT PERFORMANCE BENCHMARK")
    print("=" * 70)
    print(f"\nConfiguration:")
    print(f"  Requirements: {NUM_REQUIREMENTS}")
    print(f"  Components:   {NUM_COMPONENTS}")
    print(f"  Risks:        {NUM_RISKS}")
    print(f"  Tests:        {NUM_TESTS}")
    print(f"  Suppliers:    {NUM_SUPPLIERS}")
    print(f"  Total:        {NUM_REQUIREMENTS + NUM_COMPONENTS + NUM_RISKS + NUM_TESTS + NUM_SUPPLIERS}")
    print(f"  TDT binary:   {TDT_BIN}")
    print()

    # Create temp directory
    work_dir = tempfile.mkdtemp(prefix="tdt-bench-")
    csv_dir = os.path.join(work_dir, "csvs")
    project_dir = os.path.join(work_dir, "project")
    os.makedirs(csv_dir)
    os.makedirs(project_dir)

    print(f"Working directory: {work_dir}\n")

    results = []

    # Generate CSVs
    print("Generating test data...")
    t0 = time.perf_counter()

    req_headers = ["title", "type", "priority", "status", "category", "text", "rationale", "tags"]
    write_csv(f"{csv_dir}/requirements.csv", req_headers, generate_requirements(NUM_REQUIREMENTS))

    cmp_headers = ["part_number", "title", "make_buy", "category", "description", "material", "finish", "mass", "cost", "tags"]
    write_csv(f"{csv_dir}/components.csv", cmp_headers, generate_components(NUM_COMPONENTS))

    risk_headers = ["title", "type", "category", "description", "failure_mode", "cause", "effect", "severity", "occurrence", "detection", "tags"]
    write_csv(f"{csv_dir}/risks.csv", risk_headers, generate_risks(NUM_RISKS))

    test_headers = ["title", "type", "level", "method", "category", "priority", "objective", "description", "estimated_duration", "tags"]
    write_csv(f"{csv_dir}/tests.csv", test_headers, generate_tests(NUM_TESTS))

    sup_headers = ["name", "short_name", "category", "contact_name", "contact_email", "contact_phone", "website", "tags"]
    write_csv(f"{csv_dir}/suppliers.csv", sup_headers, generate_suppliers(NUM_SUPPLIERS))

    gen_time = time.perf_counter() - t0
    print(f"  CSV generation: {gen_time:.3f}s\n")

    # Initialize project
    print("-" * 70)
    print("IMPORT BENCHMARKS")
    print("-" * 70)

    os.chdir(project_dir)
    elapsed, ok, _, _ = run_timed(f"{TDT_BIN} init -q", "init")
    results.append(("tdt init", elapsed, ok))
    print(f"  tdt init:              {elapsed:>8.3f}s {'✓' if ok else '✗'}")

    # Import each entity type
    for etype, count, csv_name in [
        ("req", NUM_REQUIREMENTS, "requirements.csv"),
        ("cmp", NUM_COMPONENTS, "components.csv"),
        ("sup", NUM_SUPPLIERS, "suppliers.csv"),
        ("risk", NUM_RISKS, "risks.csv"),
        ("test", NUM_TESTS, "tests.csv"),
    ]:
        elapsed, ok, _, _ = run_timed(f"{TDT_BIN} import {etype} {csv_dir}/{csv_name}", f"import {etype}")
        rate = count / elapsed if elapsed > 0 else 0
        results.append((f"import {etype} ({count})", elapsed, ok))
        print(f"  import {etype:4} ({count:4}):    {elapsed:>8.3f}s  ({rate:>6.0f}/s) {'✓' if ok else '✗'}")

    total_entities = NUM_REQUIREMENTS + NUM_COMPONENTS + NUM_RISKS + NUM_TESTS + NUM_SUPPLIERS

    # Validation
    print()
    print("-" * 70)
    print("VALIDATION BENCHMARKS")
    print("-" * 70)

    elapsed, ok, _, _ = run_timed(f"{TDT_BIN} validate", "validate")
    rate = total_entities / elapsed if elapsed > 0 else 0
    results.append(("validate", elapsed, ok))
    print(f"  tdt validate:          {elapsed:>8.3f}s  ({rate:>6.0f}/s) {'✓' if ok else '✗'}")

    elapsed, ok, _, _ = run_timed(f"{TDT_BIN} validate --fix", "validate --fix")
    results.append(("validate --fix", elapsed, ok))
    print(f"  tdt validate --fix:    {elapsed:>8.3f}s {'✓' if ok else '✗'}")

    # List operations
    print()
    print("-" * 70)
    print("LIST BENCHMARKS")
    print("-" * 70)

    for cmd, label in [
        (f"{TDT_BIN} req list", f"req list ({NUM_REQUIREMENTS})"),
        (f"{TDT_BIN} cmp list", f"cmp list ({NUM_COMPONENTS})"),
        (f"{TDT_BIN} risk list", f"risk list ({NUM_RISKS})"),
        (f"{TDT_BIN} test list", f"test list ({NUM_TESTS})"),
        (f"{TDT_BIN} req list --format json", "req list --format json"),
        (f"{TDT_BIN} req list --priority critical", "req list --priority critical"),
        (f"{TDT_BIN} risk list --by-rpn", "risk list --by-rpn"),
        (f"{TDT_BIN} req list --count", "req list --count"),
    ]:
        elapsed, ok, _, _ = run_timed(cmd, label)
        results.append((label, elapsed, ok))
        print(f"  {label:28} {elapsed:>8.3f}s {'✓' if ok else '✗'}")

    # Status and reports
    print()
    print("-" * 70)
    print("STATUS & REPORT BENCHMARKS")
    print("-" * 70)

    for cmd, label in [
        (f"{TDT_BIN} status", "status"),
        (f"{TDT_BIN} status --detailed", "status --detailed"),
        (f"{TDT_BIN} report rvm", "report rvm"),
        (f"{TDT_BIN} report fmea", "report fmea"),
        (f"{TDT_BIN} report test-status", "report test-status"),
        (f"{TDT_BIN} report open-issues", "report open-issues"),
        (f"{TDT_BIN} trace matrix", "trace matrix"),
    ]:
        elapsed, ok, _, _ = run_timed(cmd, label)
        results.append((label, elapsed, ok))
        print(f"  {label:28} {elapsed:>8.3f}s {'✓' if ok else '✗'}")

    # Cache operations
    print()
    print("-" * 70)
    print("CACHE BENCHMARKS")
    print("-" * 70)

    for cmd, label in [
        (f"{TDT_BIN} cache status", "cache status"),
        (f"{TDT_BIN} cache rebuild", "cache rebuild"),
        (f"{TDT_BIN} cache rebuild", "cache rebuild (warm)"),
    ]:
        elapsed, ok, _, _ = run_timed(cmd, label)
        results.append((label, elapsed, ok))
        print(f"  {label:28} {elapsed:>8.3f}s {'✓' if ok else '✗'}")

    # Summary
    print()
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)

    total_time = sum(r[1] for r in results)
    failed = sum(1 for r in results if not r[2])

    print(f"\n  Total entities:     {total_entities}")
    print(f"  Total benchmark:    {total_time:.3f}s")
    print(f"  Operations run:     {len(results)}")
    print(f"  Operations failed:  {failed}")

    # Cleanup prompt
    print(f"\n  Working directory:  {work_dir}")
    print(f"  (Run 'rm -rf {work_dir}' to clean up)")

if __name__ == "__main__":
    main()
