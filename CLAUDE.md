- ratchet-lib is a legacy package that we are migrating away from to modularized components
- docs/ARCHITECTURE.md contains the architecture, TODO.md contains the roadmap and priorities, docs/mcp-transports.md contains the mcp transport protocol descriptions, docs/LLM_TASK_DEVELOPMENT.md contains the instructions and description for an LLM to interactive with the ratchet binary. refer to and update these documents as necessary
- This project build one binary (ratchet), that contains all functionality and exposes this through commands and subcommands
- After all changes, ensure that the code compiles without errors and that all tests pass
- There are cargo tools available for analysis, testing, coverage reporting, dependency review, etc. use them as required and request installation of tools if they are not available
- The target platforms are Linux, macOS, and Windows. Ensure that the code is cross-platform compatible. This includes using rustls over openssl
- When writing documentation, follow the guidelines in docs/writing-guide.md

