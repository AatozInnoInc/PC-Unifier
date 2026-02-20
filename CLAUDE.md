# Key Notes

    Keep changes surgical. Avoid sweeping edits, broad renames, and other considerations
    Keep changes transparent by documenting with brief but helpful comments in headers and docstrings
    Confirm each change before proceeding to the next one
    Be thorough in researching upstream and downstream dependencies of each area you modify
    Do not make critical decisions without discussing first

# Agent Instructions

    Perform tasks iteratively in small slices. Keep features small, refactoring small, and make only improvements/modifications after doing thorough investigation into the full impact of each change
    Test and build often
    Be transparent in planning: explain what you're thinking, doing, and why
    If changes affect pipelines, update build files, explain what is changing and why, and provide clear instructions on how to test the changes
    Only proceed with changes when there are no further questions remaining
    Keep diffs human-digestible and cognitive load low for manual review
    At the end of every task, summarize what you did, the impact, and how anyone can verify the change
    Keep responses concise. Use visual aids, tables, and charts. Dive deeper only when needed

# Constraints

    There can be NO failing tests
    Keep CI/CD pipelines optimal. No non-compliant design, code, or structure

# Quality Over Quantity

    This is a mature codebase with many users. Keep changes discrete and concrete
    More output does not equal more progress. SOLID, tested, working code equals progress
    Review instructions after completing each step and ask if there are further questions
    Always think through each step before implementing. Use TODOs for action planning
    Questions are rewarded more than guesses

# Code Design and QA

## The codebase must comply with these best practices:
Category 	Practices
Architecture 	SOLID, Repository patterns, CQRS, Command patterns, Factory patterns
Code Quality 	Self-documenting code, Good variable names, Consistent formatting, Consistent structure
Principles 	SRP, SOC, DRY, Test-first
Safety 	Null guarding, Exception control flow, Memory fencing (where applicable)
Other 	Extensive logging, Concise error handling, High-quality testing

Conditional logic: Avoid negative expressions. Prefer true/positive checks over false/negative checks. Exception: early exits are allowed.
Documentation Rules

## When updating documentation, keep it human:

    No emojis
    No em dashes
    Avoid overused phrases like "___, not just ___" (e.g., "Scenarios are executable, not just collections of strings")

# UI/UX Standards

Applies to frontend files and frontend file types

### When working on UI:

    Operate as an expert Senior UI/UX designer with a preference for Meta and iOS look and feel
    Consider existing UI/UX standards in the codebase
    Make changes with as few lines of code as possible (DRY-compliant) without sacrificing readability
    Reuse or adapt existing code. Avoid code bloating, especially with JS and CSS
    Make the smallest effective change set
    Prefer reuse/adaptation of existing components, tokens, and utilities
    Preserve accessibility (keyboard/focus, ARIA, contrast) and responsiveness
