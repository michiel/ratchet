# DAG Workflow Engine Plan for Ratchet

## Executive Summary

This document outlines the design and implementation plan for adding DAG (Directed Acyclic Graph) workflow capabilities to Ratchet. The design prioritizes future visual editing capabilities while maintaining programmatic flexibility and ensuring compatibility with the existing task execution infrastructure.

## Goals

### Primary Goals
1. **Visual Editor Ready**: Design data structures that map naturally to visual representations
2. **Branching Logic**: Support conditional execution paths based on task outputs
3. **Parallel Execution**: Enable concurrent task execution where dependencies allow
4. **State Management**: Track workflow execution state for resumability and debugging
5. **Backward Compatible**: Integrate seamlessly with existing Ratchet tasks

### Secondary Goals
1. **Workflow Versioning**: Support multiple versions of workflows in production
2. **Sub-workflows**: Allow workflows to contain other workflows
3. **Dynamic Workflows**: Support runtime workflow generation
4. **Event Triggers**: Enable event-based workflow initiation

## Core Concepts

### 1. Workflow Definition Structure

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct WorkflowDefinition {
    pub id: Uuid,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub metadata: WorkflowMetadata,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub variables: HashMap<String, VariableDefinition>,
    pub triggers: Vec<WorkflowTrigger>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkflowMetadata {
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub visual_layout: Option<VisualLayout>, // For visual editor
}
```

### 2. Node Types for Visual Representation

```rust
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum WorkflowNode {
    Start {
        id: String,
        position: Option<Position>,
    },
    Task {
        id: String,
        task_ref: TaskReference,
        name: String,
        description: Option<String>,
        input_mapping: HashMap<String, DataMapping>,
        output_mapping: HashMap<String, String>,
        retry_policy: Option<RetryPolicy>,
        timeout: Option<Duration>,
        position: Option<Position>,
    },
    Condition {
        id: String,
        name: String,
        expression: ConditionExpression,
        position: Option<Position>,
    },
    Parallel {
        id: String,
        name: String,
        wait_for_all: bool,
        position: Option<Position>,
    },
    Loop {
        id: String,
        name: String,
        items_expression: String, // JSONPath to array
        item_variable: String,
        max_iterations: Option<u32>,
        position: Option<Position>,
    },
    SubWorkflow {
        id: String,
        workflow_ref: WorkflowReference,
        input_mapping: HashMap<String, DataMapping>,
        position: Option<Position>,
    },
    End {
        id: String,
        output_mapping: HashMap<String, DataMapping>,
        position: Option<Position>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}
```

### 3. Edge Definition for Flow Control

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct WorkflowEdge {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub condition: Option<EdgeCondition>,
    pub label: Option<String>,
    pub visual_points: Option<Vec<Position>>, // For curved edges
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EdgeCondition {
    Always,
    Expression(String), // JSONPath expression
    OutputEquals { field: String, value: serde_json::Value },
    OutputMatches { field: String, pattern: String },
    StatusEquals(TaskStatus),
}
```

### 4. Data Mapping and Variable System

```rust
#[derive(Serialize, Deserialize, Debug)]
pub enum DataMapping {
    Static(serde_json::Value),
    Variable(String),
    Expression(String), // JSONPath expression
    Transform {
        source: Box<DataMapping>,
        operation: TransformOperation,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TransformOperation {
    ToString,
    ToNumber,
    ToBoolean,
    JsonStringify,
    JsonParse,
    Base64Encode,
    Base64Decode,
    Custom(String), // JavaScript expression
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VariableDefinition {
    pub name: String,
    pub var_type: VariableType,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
    pub description: Option<String>,
}
```

## Execution Model

### 1. Workflow Execution State

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub workflow_version: String,
    pub status: WorkflowStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub context: WorkflowContext,
    pub node_executions: HashMap<String, NodeExecution>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkflowContext {
    pub variables: HashMap<String, serde_json::Value>,
    pub inputs: serde_json::Value,
    pub outputs: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeExecution {
    pub node_id: String,
    pub status: NodeStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}
```

### 2. Execution Engine Architecture

```rust
pub struct WorkflowExecutor {
    task_executor: Arc<dyn TaskExecutor>,
    state_store: Arc<dyn WorkflowStateStore>,
    event_bus: Arc<dyn EventBus>,
}

#[async_trait]
pub trait WorkflowStateStore: Send + Sync {
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<()>;
    async fn load_execution(&self, id: Uuid) -> Result<Option<WorkflowExecution>>;
    async fn update_node_status(&self, 
        execution_id: Uuid, 
        node_id: &str, 
        status: NodeStatus
    ) -> Result<()>;
}
```

## Visual Editor Integration

### 1. Visual Layout Information

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct VisualLayout {
    pub canvas_size: CanvasSize,
    pub zoom_level: f64,
    pub node_styles: HashMap<String, NodeStyle>,
    pub edge_styles: HashMap<String, EdgeStyle>,
    pub groups: Vec<NodeGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeStyle {
    pub color: Option<String>,
    pub icon: Option<String>,
    pub shape: Option<NodeShape>,
    pub size: Option<NodeSize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeShape {
    Rectangle,
    RoundedRectangle,
    Circle,
    Diamond,
    Hexagon,
}
```

### 2. Visual Editor API Endpoints

```graphql
type Mutation {
  # Workflow Designer Operations
  createWorkflow(input: CreateWorkflowInput!): Workflow!
  updateWorkflowLayout(id: ID!, layout: VisualLayoutInput!): Workflow!
  
  # Node Operations
  addWorkflowNode(workflowId: ID!, node: WorkflowNodeInput!): WorkflowNode!
  updateWorkflowNode(workflowId: ID!, nodeId: ID!, updates: NodeUpdateInput!): WorkflowNode!
  deleteWorkflowNode(workflowId: ID!, nodeId: ID!): Boolean!
  
  # Edge Operations
  connectNodes(workflowId: ID!, edge: WorkflowEdgeInput!): WorkflowEdge!
  updateEdgeCondition(workflowId: ID!, edgeId: ID!, condition: EdgeConditionInput): WorkflowEdge!
  deleteEdge(workflowId: ID!, edgeId: ID!): Boolean!
  
  # Validation
  validateWorkflow(id: ID!): ValidationResult!
}

type Query {
  # Designer Support
  availableTasks: [TaskDefinition!]!
  workflowTemplates: [WorkflowTemplate!]!
  
  # Execution Monitoring
  workflowExecution(id: ID!): WorkflowExecution
  workflowExecutions(workflowId: ID!, status: WorkflowStatus): [WorkflowExecution!]!
}

subscription {
  # Real-time execution updates for visual feedback
  workflowExecutionUpdates(executionId: ID!): ExecutionUpdate!
}
```

## Example Workflow Definitions

### 1. Simple Sequential Workflow (YAML)

```yaml
id: "550e8400-e29b-41d4-a716-446655440000"
version: "1.0.0"
name: "Data Processing Pipeline"
description: "Process uploaded data through validation and transformation"

variables:
  input_file:
    type: string
    required: true
  output_format:
    type: string
    default: "json"

nodes:
  - type: Start
    id: start
    position: { x: 100, y: 200 }
  
  - type: Task
    id: validate_data
    task_ref: 
      id: "data-validator"
      version: "1.0.0"
    name: "Validate Input Data"
    input_mapping:
      file_path: { Variable: "input_file" }
    position: { x: 300, y: 200 }
  
  - type: Task
    id: transform_data
    task_ref:
      id: "data-transformer"
      version: "2.1.0"
    name: "Transform Data"
    input_mapping:
      data: { Expression: "$.validate_data.output.validated_data" }
      format: { Variable: "output_format" }
    position: { x: 500, y: 200 }
  
  - type: End
    id: end
    output_mapping:
      result: { Expression: "$.transform_data.output" }
    position: { x: 700, y: 200 }

edges:
  - id: edge1
    from_node: start
    to_node: validate_data
  
  - id: edge2
    from_node: validate_data
    to_node: transform_data
  
  - id: edge3
    from_node: transform_data
    to_node: end
```

### 2. Branching Workflow with Conditions

```yaml
nodes:
  - type: Task
    id: check_customer
    task_ref: { id: "customer-checker" }
    name: "Check Customer Type"
    position: { x: 300, y: 200 }
  
  - type: Condition
    id: customer_type_branch
    name: "Customer Type?"
    expression: "$.check_customer.output.customer_type"
    position: { x: 500, y: 200 }
  
  - type: Task
    id: premium_process
    task_ref: { id: "premium-handler" }
    name: "Premium Customer Process"
    position: { x: 700, y: 100 }
  
  - type: Task
    id: standard_process
    task_ref: { id: "standard-handler" }
    name: "Standard Customer Process"
    position: { x: 700, y: 300 }

edges:
  - id: to_condition
    from_node: check_customer
    to_node: customer_type_branch
  
  - id: to_premium
    from_node: customer_type_branch
    to_node: premium_process
    condition:
      OutputEquals: 
        field: "value"
        value: "premium"
    label: "Premium"
  
  - id: to_standard
    from_node: customer_type_branch
    to_node: standard_process
    condition:
      OutputEquals:
        field: "value"
        value: "standard"
    label: "Standard"
```

### 3. Parallel Processing Workflow

```yaml
nodes:
  - type: Parallel
    id: parallel_processing
    name: "Process in Parallel"
    wait_for_all: true
    position: { x: 300, y: 200 }
  
  - type: Task
    id: process_images
    task_ref: { id: "image-processor" }
    name: "Process Images"
    position: { x: 500, y: 100 }
  
  - type: Task
    id: process_metadata
    task_ref: { id: "metadata-extractor" }
    name: "Extract Metadata"
    position: { x: 500, y: 200 }
  
  - type: Task
    id: generate_thumbnails
    task_ref: { id: "thumbnail-generator" }
    name: "Generate Thumbnails"
    position: { x: 500, y: 300 }

edges:
  - from_node: parallel_processing
    to_node: process_images
  - from_node: parallel_processing
    to_node: process_metadata
  - from_node: parallel_processing
    to_node: generate_thumbnails
```

## Implementation Phases

### Phase 1: Core Workflow Engine (2-3 months)
1. **Data Models**: Implement workflow definition structures
2. **Execution Engine**: Basic sequential and parallel execution
3. **State Management**: Workflow state persistence and recovery
4. **API Layer**: GraphQL mutations and queries for workflow management

### Phase 2: Advanced Features (2-3 months)
1. **Branching Logic**: Implement condition nodes and edge conditions
2. **Loop Support**: Add loop nodes with iteration control
3. **Sub-workflows**: Enable workflow composition
4. **Error Handling**: Retry policies and error branches

### Phase 3: Visual Editor Foundation (3-4 months)
1. **Layout Engine**: Automatic layout algorithms for DAGs
2. **Visual API**: Real-time updates and drag-drop support
3. **Validation**: Visual feedback for invalid workflows
4. **Templates**: Pre-built workflow templates

### Phase 4: Production Features (2-3 months)
1. **Versioning**: Workflow version management
2. **Monitoring**: Execution visualization and debugging
3. **Triggers**: Event-based and scheduled triggers
4. **Optimization**: Execution performance improvements

## Technical Considerations

### 1. DAG Validation
- Cycle detection algorithms
- Dependency resolution
- Type checking for data mappings
- Dead path detection

### 2. Execution Optimization
- Task result caching
- Parallel execution scheduling
- Resource allocation strategies
- Checkpoint and recovery mechanisms

### 3. Visual Editor Requirements
- WebSocket support for real-time updates
- Collaborative editing capabilities
- Undo/redo functionality
- Import/export formats (YAML, JSON, BPMN)

### 4. Integration Points
- Existing task registry
- Current job queue system
- Authentication and authorization
- Monitoring and metrics

## Database Schema

```sql
-- Workflow definitions
CREATE TABLE workflows (
    id UUID PRIMARY KEY,
    version VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    definition JSONB NOT NULL,
    visual_layout JSONB,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    created_by UUID REFERENCES users(id),
    UNIQUE(id, version)
);

-- Workflow executions
CREATE TABLE workflow_executions (
    id UUID PRIMARY KEY,
    workflow_id UUID NOT NULL,
    workflow_version VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    context JSONB NOT NULL,
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    created_by UUID REFERENCES users(id),
    FOREIGN KEY (workflow_id, workflow_version) 
        REFERENCES workflows(id, version)
);

-- Node execution state
CREATE TABLE node_executions (
    id UUID PRIMARY KEY,
    execution_id UUID REFERENCES workflow_executions(id),
    node_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    input_data JSONB,
    output_data JSONB,
    error TEXT,
    attempts INTEGER DEFAULT 0
);

-- Indexes for performance
CREATE INDEX idx_workflow_executions_status ON workflow_executions(status);
CREATE INDEX idx_workflow_executions_workflow ON workflow_executions(workflow_id);
CREATE INDEX idx_node_executions_execution ON node_executions(execution_id);
```

## Success Criteria

1. **Functionality**
   - Support for all defined node types
   - Reliable execution with state persistence
   - < 100ms overhead per node transition

2. **Visual Editor Readiness**
   - All workflow elements have position data
   - Real-time execution visualization
   - Drag-and-drop workflow creation

3. **Developer Experience**
   - Simple YAML/JSON workflow definitions
   - Clear error messages
   - Comprehensive documentation

4. **Production Readiness**
   - Horizontal scalability
   - Workflow versioning
   - Monitoring and debugging tools

## Future Enhancements

1. **AI-Assisted Workflow Design**
   - Suggest optimal workflow patterns
   - Automatic error handling insertion
   - Performance optimization recommendations

2. **Advanced Patterns**
   - Saga pattern support
   - Compensation workflows
   - Dynamic workflow generation

3. **External Integrations**
   - BPMN import/export
   - Apache Airflow compatibility
   - Temporal.io workflow migration

4. **Collaboration Features**
   - Multi-user workflow editing
   - Change tracking and approval
   - Workflow marketplace

## Conclusion

This plan provides a foundation for implementing a visual-editor-ready DAG workflow system in Ratchet. The design prioritizes flexibility, visual representation, and gradual implementation while maintaining compatibility with existing Ratchet features.