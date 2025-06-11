# Test Repository

A test repository for validating repo commands

## Repository Structure

```
.
├── .ratchet/
│   ├── registry.yaml    # Repository metadata and configuration
│   └── index.json       # Fast task discovery index
├── tasks/               # Individual task implementations
├── collections/         # Task collections and workflows
├── templates/           # Task templates and boilerplate
└── README.md           # This file
```

## Getting Started

### Adding Tasks

1. Create a new directory under `tasks/`
2. Include all required files:
   - `metadata.json`: Task definition and configuration
   - `main.js`: Task implementation
   - `input.schema.json`: Input validation schema
   - `output.schema.json`: Output format definition
   - `tests/`: Test cases and examples

### Refreshing Metadata

After adding or modifying tasks, refresh the repository metadata:

```bash
ratchet repo refresh-metadata
```

### Using with Git+HTTP Registry

Configure this repository as a Git task source in your Ratchet configuration:

```yaml
registries:
  - name: "my-tasks"
    source:
      type: "git"
      url: "https://github.com/your-org/your-task-repo.git"
      ref: "main"
```

## License

See individual task metadata for licensing information.
