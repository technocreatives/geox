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

impl TryFrom<Geometry> for Polygon {
    type Error = geo_types::Error;

    fn try_from(value: Geometry) -> Result<Self, Self::Error> {
        geo::Polygon::try_from(value.0).map(Polygon)
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

#[cfg(feature = "serde")]
use serde::Serialize;

#[cfg(feature = "serde")]
impl Serialize for Polygon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        use geozero::ToJson;
        use std::collections::BTreeMap;
        use serde_json::Value;
        use serde::ser::{Error, SerializeMap};

        let s = geo::Geometry::Polygon(self.0.clone()).to_json().map_err(Error::custom)?;
        let s: BTreeMap<String, Value> = serde_json::from_str(&s).map_err(Error::custom)?;

        let mut map = serializer.serialize_map(Some(s.len()))?;
        for (k, v) in s {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}
