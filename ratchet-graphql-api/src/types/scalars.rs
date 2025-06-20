//! Custom scalar types for GraphQL

use async_graphql::{Scalar, ScalarType, Value};
use ratchet_api_types::ApiId;
use serde::{Deserialize, Serialize};

/// Custom ApiId scalar for GraphQL
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQLApiId(pub ApiId);

#[Scalar]
impl ScalarType for GraphQLApiId {
    fn parse(value: Value) -> async_graphql::InputValueResult<Self> {
        match value {
            Value::String(s) => Ok(GraphQLApiId(ApiId(s))),
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0 .0.clone())
    }
}

impl From<ApiId> for GraphQLApiId {
    fn from(id: ApiId) -> Self {
        GraphQLApiId(id)
    }
}

impl From<GraphQLApiId> for ApiId {
    fn from(id: GraphQLApiId) -> Self {
        id.0
    }
}

impl From<String> for GraphQLApiId {
    fn from(s: String) -> Self {
        GraphQLApiId(ApiId(s))
    }
}

impl From<GraphQLApiId> for String {
    fn from(id: GraphQLApiId) -> Self {
        id.0 .0
    }
}
