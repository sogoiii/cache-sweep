---
name: explain
description: Analyze a code suggestion/finding with diagrams and clear explanation
arguments:
  - name: finding
    description: The suggestion, finding, or issue to analyze
    required: true
---

<explain-finding>

## Your Task

Analyze and explain the following finding to help me understand it deeply:

**Finding:** $ARGUMENTS.finding

## Process

1. **Locate**: Find the code location(s) mentioned. If no file:line given, search for relevant code.

2. **Validate**: Read the actual code. Confirm whether the issue exists as described.
   - If the original analysis was wrong, say so clearly
   - If it's outdated (already fixed), note that

3. **Analyze**: Trace the execution flow. Understand:
   - What triggers this code path?
   - What's the current behavior?
   - What's the alleged problem?

4. **Visualize**: Create ASCII diagrams to explain. Use:
   - Flow charts for execution paths
   - Box diagrams for architecture
   - Tables for comparisons/tradeoffs
   - Timeline diagrams for async/concurrent issues

5. **Explain Simply**:
   - What happens now (current flow)
   - Why it might be a problem
   - What a fix would look like

6. **Assess**: Provide your judgment:
   | Aspect | Assessment |
   |--------|------------|
   | Issue valid? | Yes/No/Partially |
   | Severity | Critical/Medium/Low/Non-issue |
   | Fix complexity | Lines of code, risk |
   | Recommendation | Fix/Skip/Discuss |

## Rules

- ALWAYS read the code first—never trust the finding blindly
- Be honest if the finding is wrong or overstated
- Keep explanations concise but complete
- Diagrams should fit in ~60 char width
- Ask clarifying questions if the finding is ambiguous

</explain-finding>
