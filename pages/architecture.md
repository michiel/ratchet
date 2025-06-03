---
layout: default
title: Architecture Overview
permalink: /architecture/
---

# Architecture Overview

Ratchet is built with a modular, layered architecture designed for scalability, maintainability, and performance. This page provides a comprehensive overview of the system architecture and its components.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │   CLI Client    │  │  Web Frontend   │  │  External API   ││
│  │  (ratchet-cli)  │  │   (Refine.dev)  │  │    Clients      ││
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘│
└───────────┼────────────────────┼────────────────────┼──────────┘
            │                    │                    │
            ▼                    ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                         API Layer                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Axum Web Server                       │   │
│  │  ┌─────────────────┐              ┌──────────────────┐  │   │
│  │  │   REST API      │              │   GraphQL API   │  │   │
│  │  │ • /tasks        │              │ • Query         │  │   │
│  │  │ • /jobs         │              │ • Mutation      │  │   │
│  │  │ • /executions   │              │ • Subscription  │  │   │
│  │  │ • /schedules    │              │ • Playground    │  │   │
│  │  │ • /workers      │              │                  │  │   │
│  │  └─────────────────┘              └──────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Service Layer                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │  RatchetEngine  │  │    Service      │  │  Execution      ││
│  │                 │  │    Provider     │  │    Manager      ││
│  │ • Task Service  │  │ • Dependency    │  │ • Worker Pool   ││
│  │ • HTTP Service  │  │   Injection     │  │ • Job Queue     ││
│  │ • Config Service│  │ • Service Init  │  │ • Load Balancer ││
│  └─────────────────┘  └─────────────────┘  └─────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Data & Storage Layer                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │    SQLite DB    │  │   Task Cache    │  │   File System   ││
│  │   (Sea-ORM)     │  │   (In-Memory)   │  │  (Task Storage) ││
│  └─────────────────┘  └─────────────────┘  └─────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Client Layer

The client layer provides multiple interfaces for interacting with Ratchet:

- **CLI Client** (`ratchet-cli`): Command-line interface for task management and execution
- **Web Frontend**: Refine.dev-compatible REST API for web applications
- **External API Clients**: Third-party integrations via REST or GraphQL

### 2. API Layer

Built on Axum web framework, the API layer offers:

#### REST API
- Full CRUD operations for tasks, jobs, executions, and schedules
- Pagination, filtering, and sorting support
- OpenAPI 3.0 specification
- Refine.dev compatibility

#### GraphQL API
- Query, mutation, and subscription support
- GraphQL Playground for exploration
- Type-safe schema with automatic documentation

#### Middleware Stack
- **CORS**: Cross-origin resource sharing
- **Rate Limiting**: Token bucket algorithm
- **Request ID**: Tracing and debugging
- **Validation**: Input validation and sanitization
- **Error Handling**: Consistent error responses

### 3. Service Layer

The service layer contains the business logic:

#### RatchetEngine
The central service coordinator providing:
- Task service for task management
- HTTP service for external requests
- Configuration service
- Registry service for task discovery

#### Service Provider
- Dependency injection container
- Service initialization and lifecycle management
- Inter-service communication

#### Execution Manager
- Worker pool management
- Job queue with priority support
- Load balancing strategies

### 4. Execution Layer

Responsible for actual task execution:

#### Process Executor
- Worker process pool
- IPC (Inter-Process Communication) transport
- Health monitoring and recovery

#### Job Queue Manager
- Priority-based job scheduling
- Batch processing support
- Job state management

#### Resilience Features
- **Retry System**: Configurable backoff strategies
- **Circuit Breaker**: Failure detection and recovery
- **Task Cache**: LRU caching with memory awareness

### 5. Data & Storage Layer

Persistent storage and caching:

#### SQLite Database (via Sea-ORM)
- Tasks, executions, jobs, and schedules
- Migration system for schema evolution
- Repository pattern for clean data access

#### Task Cache
- In-memory LRU cache
- Reduces database load
- Configurable TTL and size limits

#### File System
- Task file storage
- Configuration files
- Log file persistence

## Process Isolation Model

Ratchet uses a process-based isolation model for security and stability:

```
┌─────────────────────┐
│   Main Process      │
│  (Orchestrator)     │
└──────────┬──────────┘
           │
           ├─── IPC Channel ──┬─── IPC Channel ──┬─── IPC Channel
           │                  │                  │
    ┌──────▼────────┐  ┌──────▼────────┐  ┌──────▼────────┐
    │ Worker Process│  │ Worker Process│  │ Worker Process│
    │   (Isolated)  │  │   (Isolated)  │  │   (Isolated)  │
    └───────────────┘  └───────────────┘  └───────────────┘
```

### Benefits
- **Security**: Tasks run in isolated processes
- **Stability**: Crashes don't affect other tasks
- **Resource Control**: Per-process limits
- **Monitoring**: Individual process metrics

## Modular Crate Structure

Ratchet is organized into multiple crates for modularity:

| Crate | Purpose |
|-------|---------|
| `ratchet-core` | Core types and traits |
| `ratchet-api` | API types and contracts |
| `ratchet-storage` | Database and persistence |
| `ratchet-runtime` | Task execution runtime |
| `ratchet-resilience` | Retry and circuit breaker |
| `ratchet-caching` | Caching implementations |
| `ratchet-config` | Configuration management |
| `ratchet-plugin` | Plugin system |
| `ratchet-lib` | Main library with all features |
| `ratchet-cli` | Command-line interface |

## Design Principles

### 1. Type Safety
- Strong typing throughout the codebase
- Compile-time guarantees
- Minimal use of `unwrap()`

### 2. Error Handling
- Comprehensive error types
- Context propagation
- User-friendly error messages

### 3. Testability
- Dependency injection
- Mock implementations
- Integration test suite

### 4. Performance
- Async/await for concurrency
- Connection pooling
- Efficient data structures

### 5. Security
- Input validation
- SQL injection prevention
- Process isolation

## Next Steps

- Explore [Example Uses]({{ "/examples" | relative_url }}) to see Ratchet in action
- Learn about [Server Configuration]({{ "/server-configuration" | relative_url }}) options
- Understand [Logging & Error Handling]({{ "/logging-error-handling" | relative_url }}) capabilities