use std::ops::Deref;

#[cfg(feature = "sqlx")]
use std::convert::TryFrom;

#[cfg(feature = "sqlx")]
use sqlx::{
    postgres::{PgTypeInfo, PgValueRef},
    Postgres,
};

#[cfg(feature = "async-graphql")]
use async_graphql::{InputValueError, InputValueResult, Number, ScalarType, Value};

#[cfg(feature = "sqlx")]
use crate::Geometry;

#[derive(Clone, Debug)]
pub struct Polygon(pub geo::Polygon<f64>);

impl PartialEq for Polygon {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Polygon {}

impl Deref for Polygon {
    type Target = geo::Polygon<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<Postgres> for Polygon {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("geometry")
    }
}

impl Polygon {
    pub fn into_inner(self) -> geo::Polygon<f64> {
        self.0
    }

    #[cfg(feature = "async-graphql")]
    fn to_async_graphql_value(&self) -> async_graphql::Value {
        let ext = self.0.exterior();
        Value::List(
            ext.clone()
                .into_iter()
                .map(|coord| {
                    let x = Value::Number(Number::from_f64(coord.x).unwrap());
                    let y = Value::Number(Number::from_f64(coord.y).unwrap());

                    let coord = Value::List(vec![x, y]);
                    coord
                })
                .collect::<Vec<_>>(),
        )
    }
}

#[cfg(feature = "sqlx")]
impl<'de> sqlx::Decode<'de, Postgres> for Polygon {
    fn decode(value: PgValueRef<'de>) -> Result<Self, sqlx::error::BoxDynError> {
        let geometry = Geometry::decode(value)?;
        let polygon = geo::Polygon::<f64>::try_from(geometry.0)?;
        Ok(Polygon(polygon))
    }
}

#[cfg(feature = "async-graphql")]
#[async_graphql::Scalar]
impl ScalarType for Polygon {
    fn parse(_value: Value) -> InputValueResult<Self> {
        Err(InputValueError::custom("parsing not implemented"))
    }

    fn to_value(&self) -> Value {
        self.to_async_graphql_value()
    }
}