use std::{convert::TryFrom, ops::Deref};

#[cfg(feature = "sqlx")]
use geozero::ToWkb;

#[cfg(feature = "sqlx")]
use sqlx::{
    encode::IsNull,
    postgres::{PgHasArrayType, PgTypeInfo, PgValueRef},
    Postgres,
};

#[cfg(feature = "async-graphql")]
use async_graphql::{InputValueError, InputValueResult, Number, ScalarType, Value};

use crate::Geometry;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
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

#[cfg(feature = "sqlx")]
impl PgHasArrayType for Point {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_geometry")
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

#[cfg(feature = "sqlx")]
impl<'en> sqlx::Encode<'en, Postgres> for Point {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> IsNull {
        let x = geo::Geometry::Point(self.0)
            .to_ewkb(geozero::CoordDimensions::xy(), None)
            .unwrap();
        buf.extend(x);
        sqlx::encode::IsNull::No
    }
}

#[cfg(feature = "async-graphql")]
#[async_graphql::Scalar]
impl ScalarType for Point {
    #[cfg(not(feature = "serde"))]
    fn parse(_value: Value) -> InputValueResult<Self> {
        Err(InputValueError::custom("parsing not implemented"))
    }

    #[cfg(feature = "serde")]
    fn parse(value: Value) -> InputValueResult<Self> {
        use geozero::{geojson::GeoJson, ToGeo};

        match value {
            Value::String(x) => {
                let geo = GeoJson(&x)
                    .to_geo()
                    .map_err(|_| InputValueError::custom("failed to parse GeoJSON string"))?;
                match geo {
                    geo::Geometry::Point(x) => Ok(Self(x)),
                    _ => Err(InputValueError::custom("Got invalid type for Point")),
                }
            }
            _ => Err(InputValueError::custom(
                "parsing not implemented for this type (only string)",
            )),
        }
    }

    fn to_value(&self) -> Value {
        Value::List(vec![
            Value::Number(Number::from_f64(self.x()).unwrap()),
            Value::Number(Number::from_f64(self.y()).unwrap()),
        ])
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use geozero::ToJson;
        use serde::ser::{Error, SerializeMap};
        use serde_json::Value;
        use std::collections::BTreeMap;

        let s = geo::Geometry::Point(self.0)
            .to_json()
            .map_err(Error::custom)?;
        let s: BTreeMap<String, Value> = serde_json::from_str(&s).map_err(Error::custom)?;

        let mut map = serializer.serialize_map(Some(s.len()))?;
        for (k, v) in s {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

#[cfg(all(test, feature = "sqlx"))]
mod sqlx_tests {
    use super::Point;

    async fn pg_roundtrip(data_to: &Point, type_name: &str) -> Point {
        use sqlx::postgres::PgPoolOptions;
        let conn = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://postgres:password@localhost/postgres")
            .await
            .unwrap();
        let mut conn = conn.begin().await.unwrap();

        sqlx::query(&format!(
            "CREATE TABLE test ( id SERIAL PRIMARY KEY, geom GEOMETRY({type_name}, 26910) )"
        ))
        .execute(&mut conn)
        .await
        .unwrap();

        sqlx::query("INSERT INTO test (geom) VALUES ($1)")
            .bind(&data_to)
            .execute(&mut conn)
            .await
            .unwrap();

        let (data_from,): (Point,) = sqlx::query_as("SELECT geom FROM test")
            .fetch_one(&mut conn)
            .await
            .unwrap();

        data_from
    }

    #[tokio::test]
    async fn polygon() {
        let data_to = Point(geo::Point::from((0., 1.)));
        let data_from = pg_roundtrip(&data_to, "Point").await;
        assert_eq!(data_to, data_from);
    }
}
