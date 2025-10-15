#!/usr/bin/env bash
# Grammar validation script for Blazelint BNF
# Checks for completeness, consistency, and common issues

set -euo pipefail

BNF_FILE="${1:-docs/BNF.md}"
ERRORS=0
WARNINGS=0

# Color codes for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Blazelint Grammar Validation ===${NC}"
echo "Checking: $BNF_FILE"
echo ""

# Extract grammar rules (skip markdown code fences)
GRAMMAR=$(sed -n '/^```bnf$/,/^```$/p' "$BNF_FILE" | sed '1d;$d')

# Extract all non-terminals (left-hand side of ::=)
NON_TERMINALS=$(echo "$GRAMMAR" | grep -E '^\s*<[^>]+>\s*::=' | sed -E 's/^\s*<([^>]+)>.*/\1/' | sort -u)

# Extract all referenced non-terminals (right-hand side)
# Only match valid identifiers inside < >, not quoted strings or operators
REFERENCED=$(echo "$GRAMMAR" | grep -oE '<[a-zA-Z_][a-zA-Z0-9_]*>' | sed 's/[<>]//g' | sort -u)

echo -e "${BLUE}[1] Checking Defined Non-Terminals${NC}"
echo "Found $(echo "$NON_TERMINALS" | wc -l | tr -d ' ') defined non-terminals"

# Check for duplicate definitions
DUPLICATES=$(echo "$NON_TERMINALS" | sort | uniq -d)
if [ -n "$DUPLICATES" ]; then
    echo -e "${RED}✗ ERROR: Duplicate non-terminal definitions found:${NC}"
    echo "$DUPLICATES" | while read -r dup; do
        echo "  - <$dup>"
    done
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✓ No duplicate definitions${NC}"
fi

echo ""
echo -e "${BLUE}[2] Checking Referenced Non-Terminals${NC}"
echo "Found $(echo "$REFERENCED" | wc -l | tr -d ' ') referenced non-terminals"

# Check for undefined non-terminals (referenced but not defined)
UNDEFINED=""
for ref in $REFERENCED; do
    if ! echo "$NON_TERMINALS" | grep -q "^$ref\$"; then
        UNDEFINED="${UNDEFINED}${ref}\n"
        echo -e "${RED}✗ ERROR: Non-terminal <$ref> is referenced but not defined${NC}"
        ERRORS=$((ERRORS + 1))
    fi
done

if [ -z "$UNDEFINED" ]; then
    echo -e "${GREEN}✓ All referenced non-terminals are defined${NC}"
fi

echo ""
echo -e "${BLUE}[3] Checking Unused Non-Terminals${NC}"

# Check for defined but never referenced non-terminals (except <program>)
UNUSED=""
for def in $NON_TERMINALS; do
    if [ "$def" != "program" ] && ! echo "$REFERENCED" | grep -q "^$def\$"; then
        UNUSED="${UNUSED}${def}\n"
        echo -e "${YELLOW}⚠ WARNING: Non-terminal <$def> is defined but never referenced${NC}"
        WARNINGS=$((WARNINGS + 1))
    fi
done

if [ -z "$UNUSED" ]; then
    echo -e "${GREEN}✓ No unused non-terminals${NC}"
fi

echo ""
echo -e "${BLUE}[4] Checking for Common Issues${NC}"

# Check for left recursion (simple check)
LEFT_RECURSIVE=$(echo "$GRAMMAR" | grep -E '^\s*<([^>]+)>\s*::=\s*<\1>' || true)
if [ -n "$LEFT_RECURSIVE" ]; then
    echo -e "${YELLOW}⚠ WARNING: Possible left recursion detected:${NC}"
    echo "$LEFT_RECURSIVE" | while read -r line; do
        echo "  $line"
    done
    WARNINGS=$((WARNINGS + 1))
else
    echo -e "${GREEN}✓ No obvious left recursion${NC}"
fi

# Check for empty productions (epsilon)
EMPTY_PRODS=$(echo "$GRAMMAR" | grep -E '::=\s*ε' | wc -l | tr -d ' ')
if [ "$EMPTY_PRODS" -gt 0 ]; then
    echo -e "${GREEN}✓ Found $EMPTY_PRODS epsilon (empty) productions${NC}"
fi

# Check for terminals that look like non-terminals (missing quotes)
SUSPICIOUS=$(echo "$GRAMMAR" | grep -oE '\s[a-z_]+\s' | grep -v -E '(in|is|ε)' | sort -u || true)
if [ -n "$SUSPICIOUS" ]; then
    echo -e "${YELLOW}⚠ WARNING: Possible unquoted terminals (should these be in quotes?):${NC}"
    echo "$SUSPICIOUS" | while read -r term; do
        if [ -n "$term" ]; then
            echo "  - $term"
        fi
    done
    WARNINGS=$((WARNINGS + 1))
fi

echo ""
echo -e "${BLUE}[5] Checking Token Coverage${NC}"

# Extract tokens from lexer
if [ -f "src/lexer.rs" ]; then
    # Get all Token enum variants
    LEXER_TOKENS=$(sed -n '/^pub enum Token/,/^}/p' src/lexer.rs | 
                   grep -E '^\s+[A-Z][a-zA-Z]*' | 
                   sed -E 's/^\s+([A-Z][a-zA-Z]*).*/\1/' | 
                   sort -u)
    
    # Check if common tokens are referenced in grammar
    echo "Checking if lexer tokens are used in grammar..."
    
    # Sample of important tokens to check
    for token in "Import" "Public" "Function" "If" "Else" "While" "Foreach" "Return"; do
        TOKEN_LOWER=$(echo "$token" | tr '[:upper:]' '[:lower:]')
        if ! echo "$GRAMMAR" | grep -qi "\"$TOKEN_LOWER\""; then
            echo -e "${YELLOW}⚠ WARNING: Token '$token' in lexer but not clearly in grammar${NC}"
            WARNINGS=$((WARNINGS + 1))
        fi
    done
    
    echo -e "${GREEN}✓ Basic token coverage check complete${NC}"
else
    echo -e "${YELLOW}⚠ WARNING: src/lexer.rs not found, skipping token coverage check${NC}"
fi

echo ""
echo -e "${BLUE}[6] Grammar Statistics${NC}"
echo "  Total non-terminals defined: $(echo "$NON_TERMINALS" | wc -l | tr -d ' ')"
echo "  Total non-terminals referenced: $(echo "$REFERENCED" | wc -l | tr -d ' ')"
echo "  Total grammar rules: $(echo "$GRAMMAR" | grep -c '::=' || echo 0)"
echo "  Total lines in grammar: $(echo "$GRAMMAR" | wc -l | tr -d ' ')"

# Count operators
OPERATORS=$(echo "$GRAMMAR" | grep -oE '\"[^\"]+\"' | sort -u | wc -l | tr -d ' ')
echo "  Unique terminal symbols: $OPERATORS"

echo ""
echo -e "${BLUE}=== Validation Summary ===${NC}"

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✓ Grammar validation passed!${NC}"
    echo "  No errors or warnings found."
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}⚠ Grammar validation completed with warnings${NC}"
    echo "  Errors: $ERRORS"
    echo "  Warnings: $WARNINGS"
    exit 0
else
    echo -e "${RED}✗ Grammar validation failed!${NC}"
    echo "  Errors: $ERRORS"
    echo "  Warnings: $WARNINGS"
    exit 1
fi
