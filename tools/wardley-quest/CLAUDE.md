# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Strategic Evolution Quest is a text-based RPG for learning Wardley Mapping, DDD, and Team Topologies. Players interact with scenarios through LLMs (ChatGPT, Claude, etc.) by loading scenario.md files.

Based on the book "Architecture Modernization" by Nick Tune & Jean-Georges Perrin.

## Structure

```
wardley-quest/
├── scenario.md              # Main game scenario
├── README.md                # Project documentation
├── addon-*/                 # 9 addon quests
│   ├── README.md           # Addon overview
│   └── scenario.md         # Addon scenario
```

**Addons by phase:**
- Discovery: addon-discovery, addon-eventstorming
- Strategy: addon-portfolio
- Design: addon-domainboundary, addon-apidesign
- Implementation: addon-platform, addon-datamodeling
- Operations: addon-techdebt
- All phases: addon-change

## Scenario File Structure

Each scenario.md follows a consistent structure:

1. **Title & Quote** - Thematic opening
2. **Overview** - Prerequisites, learning objectives
3. **Core Concepts** - Framework/methodology explanation with ASCII diagrams
4. **NPCs** - Characters who provide quests and guidance
5. **Game Mechanics** - Skill trees, scores, progression systems
6. **Practical Scenarios** - Choice-based situations with [A/B/C] options
7. **Bad Endings** (5+) - Failure scenarios with lessons learned
8. **Undetermined Bad Endings** - Future failure possibilities
9. **Good Endings** (3+) - Success narratives
10. **Cross-quest Connections** - Links to related addons
11. **References** - Books and resources

## Content Conventions

- **ASCII diagrams** using box-drawing characters (┌─┐│└─┘)
- **Tables** for structured information
- **Japanese language** throughout
- **NPCs have distinct personalities** and provide specific quests
- **Bad endings** teach through failure; good endings show success patterns
- **Choice scenarios** present realistic dilemmas with trade-offs

## When Adding/Editing Content

- Maintain the established section structure
- Use consistent ASCII diagram style (65-char width boxes)
- Each addon should have 5+ bad endings + undetermined endings + 3+ good endings
- NPCs should have: name, role, characteristic quotes, provided quests
- Link related concepts to other addons in the connections section
- Update main scenario.md addon list when adding new addons
- Update README.md addon table and detailed descriptions

## No Build/Test Commands

This is a content-only project. No compilation, linting, or testing required.
