use std::{convert::TryFrom, ops::Deref};

#[cfg(feature = "sqlx")]
use sqlx::{
    postgres::{PgTypeInfo, PgValueRef},
    Postgres,
};

#[cfg(feature = "async-graphql")]
use async_graphql::{InputValueError, InputValueResult, Number, ScalarType, Value};

use crate::Geometry;

#[derive(Clone, Debug)]
pub struct Point(pub geo::Point<f64>);

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Point {}

impl Deref for Point {
    type Target = geo::Point<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<Postgres> for Point {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("geometry")
    }
}

impl Point {
    pub fn into_inner(self) -> geo::Point<f64> {
        self.0
    }
}

impl TryFrom<Geometry> for Point {
    type Error = geo_types::Error;

    fn try_from(value: Geometry) -> Result<Self, Self::Error> {
        geo::Point::try_from(value.0).map(Point)
    }
}

#[cfg(feature = "sqlx")]
impl<'de> sqlx::Decode<'de, Postgres> for Point {
    fn decode(value: PgValueRef<'de>) -> Result<Self, sqlx::error::BoxDynError> {
        let geometry = Geometry::decode(value)?;
        let point = geo::Point::<f64>::try_from(geometry.0)?;
        Ok(Point(point))
    }
}

#[cfg(feature = "async-graphql")]
#[async_graphql::Scalar]
impl ScalarType for Point {
    fn parse(_value: Value) -> InputValueResult<Self> {
        Err(InputValueError::custom("parsing not implemented"))
    }

    fn to_value(&self) -> Value {
        Value::List(vec![
            Value::Number(Number::from_f64(self.x()).unwrap()),
            Value::Number(Number::from_f64(self.y()).unwrap()),
        ])
    }
}
