---
name: code-refactoring-audit
description: Systematic approach to identify oversized files and create refactoring strategies for codebases. Uses Unix tools to find files over size thresholds and provides splitting recommendations.
version: 1.0.0
author: Hermes Agent
license: MIT
metadata:
  hermes:
    tags: [Code Refactoring, Code Analysis, Unix Tools, File Splitting, Code Quality, Maintainability]
    related_skills: [codebase-inspection, systematic-debugging]
prerequisites:
  commands: [find, wc, awk, grep, sort]
---

# Code Refactoring Audit

Systematically identify oversized files in codebases and create actionable refactoring strategies using lightweight Unix tools.

## When to Use

- User needs to identify files violating code size limits
- Planning a refactoring initiative to improve code organization
- Auditing codebases for maintainability issues
- Creating systematic splitting strategies for large files
- Working in environments where external dependencies aren't available

## Prerequisites

Standard Unix tools (pre-installed on most systems):
- `find` - file discovery
- `wc` - word/line counting  
- `awk` - pattern scanning and processing
- `grep` - pattern matching
- `sort` - sorting utilities

## 1. Basic File Size Audit

Get all source files over a specified line count threshold:

```bash
# Find all source files over 300 lines
find . -type f \( -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.go" -o -name "*.rs" -o -name "*.java" -o -name "*.cpp" -o -name "*.h" \) \
  -not -path "./venv/*" -not -path "./node_modules/*" -not -path "./web/node_modules/*" \
  -exec wc -l {} \; | sort -nr | awk '$1 >= 300'

# Count how many files exceed the threshold
find . -type f \( -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.go" -o -name "*.rs" -o -name "*.java" -o -name "*.cpp" -o -name "*.h" \) \
  -not -path "./venv/*" -not -path "./node_modules/*" -not -path "./web/node_modules/*" \
  -exec wc -l {} \; | sort -nr | awk '$1 >= 300' | wc -l
```

## 2. Targeted File Analysis

Examine the largest files first to understand their structure:

```bash
# Get top 10 largest files
find . -type f \( -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.go" -o -name "*.rs" -o -name "*.java" -o -name "*.cpp" -o -name "*.h" \) \
  -not -path "./venv/*" -not -path "./node_modules/*" -not -path "./web/node_modules/*" \
  -exec wc -l {} \; | sort -nr | head -20

# Analyze structure of a specific large file
grep -n "^class\|^def\|^async def" ./large_file.py | head -20
```

## 3. Language-Specific Analysis

Focus on specific programming languages:

```bash
# Python files only
find . -name "*.py" -not -path "./venv/*" -exec wc -l {} \; | sort -nr | awk '$1 >= 300'

# JavaScript/TypeScript files only  
find . -name "*.js" -o -name "*.ts" -not -path "./node_modules/*" -exec wc -l {} \; | sort -nr | awk '$1 >= 300'
```

## 4. Refactoring Priority Assessment

Create a priority matrix for refactoring:

```bash
# Files by size ranges
echo "=== Files over 10,000 lines ==="
find . -name "*.py" -not -path "./venv/*" -exec wc -l {} \; | sort -nr | awk '$1 >= 10000'

echo "=== Files 5,000-10,000 lines ==="  
find . -name "*.py" -not -path "./venv/*" -exec wc -l {} \; | sort -nr | awk '$1 >= 5000 && $1 < 10000'

echo "=== Files 1,000-5,000 lines ==="
find . -name "*.py" -not -path "./venv/*" -exec wc -l {} \; | sort -nr | awk '$1 >= 1000 && $1 < 5000'
```

## 5. Structure Analysis for Splitting

Analyze internal structure to guide splitting decisions:

```bash
# Count classes in a file
grep -c "^class " ./large_file.py

# Count functions in a file  
grep -c "^def " ./large_file.py

# Count import statements
grep -c "^import\|^from" ./large_file.py

# Identify potential split points by function complexity
grep -n "^def\|^class" ./large_file.py
```

## 6. Split Strategy Development

Based on analysis, develop systematic splitting approaches:

### For Massive Files (10k+ lines)
1. **Extract core classes** to separate modules
2. **Split utility functions** into utility modules
3. **Separate configuration handling**
4. **Move platform-specific code** to adapters
5. **Extract complex algorithms** to dedicated modules

### For Large Files (3k-10k lines)
1. **Group related functions** into logical modules
2. **Separate concerns** (config, validation, processing)
3. **Extract complex business logic**
4. **Create specialized handler classes**

### For Medium Files (1k-3k lines)
1. **Split by feature/area of responsibility**
2. **Extract common utilities**
3. **Separate data models from business logic**

## 7. Implementation Commands

Create the split structure:

```bash
# Create directory structure for a large file
mkdir -p ./split_modules/core
mkdir -p ./split_modules/utils
mkdir -p ./split_modules/config

# Extract specific function definitions (example for Python)
sed -n '100,200p' ./large_file.py > ./split_modules/utils/helpers.py
```

## 8. Validation After Splitting

Verify the refactoring improved code organization:

```bash
# Check new file sizes
find ./split_modules -name "*.py" -exec wc -l {} \; | sort -nr

# Verify no functionality was lost
diff <(./original_file.py --help 2>&1 || true) <(./new_entry_point.py --help 2>&1 || true) || echo "Outputs differ - check functionality"
```

## Pitfalls

1. **Always exclude dependencies** - node_modules, venv, .venv can cause false positives and performance issues
2. **Preserve functionality** - Test after splitting to ensure no features are broken
3. **Consider dependencies** - Check for internal imports when moving functions/classes
4. **Maintain interfaces** - Preserve public APIs when splitting modules
5. **Document changes** - Update documentation to reflect new module structure

## Best Practices

1. **Start with the largest files** - They offer the biggest impact for refactoring
2. **Split by responsibility** - Follow single responsibility principle
3. **Maintain backward compatibility** - Where possible, preserve existing APIs
4. **Test incrementally** - Test after each major split
5. **Update imports** - Ensure all internal imports are updated after splitting
6. **Document the refactoring** - Create migration guides for significant changes

## Example Output

```
=== Code Refactoring Audit Results ===
Total files over 300 lines: 367
Files over 10,000 lines: 3
Files over 5,000 lines: 8  
Files over 1,000 lines: 42

Top 5 largest files:
1. run_agent.py (11,038 lines)
2. cli.py (10,041 lines)  
3. gateway/run.py (9,741 lines)
4. hermes_cli/main.py (6,121 lines)
5. tests/run_agent/test_run_agent.py (4,000 lines)

Recommended splitting strategy:
1. Priority 1: Split run_agent.py into 5 modules
2. Priority 2: Split cli.py into 5 modules  
3. Priority 3: Split gateway/run.py into 4 modules
4. Priority 4: Split test files into focused modules
```
