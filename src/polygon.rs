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

#[cfg(feature = "sqlx")]
impl PgHasArrayType for Polygon {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_geometry")
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

                    Value::List(vec![x, y])
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

#[cfg(feature = "sqlx")]
impl<'en> sqlx::Encode<'en, Postgres> for Polygon {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> IsNull {
        let x = geo::Geometry::Polygon(self.0.clone())
            .to_ewkb(geozero::CoordDimensions::xy(), None)
            .unwrap();
        buf.extend(x);
        sqlx::encode::IsNull::No
    }
}

#[cfg(feature = "async-graphql")]
#[async_graphql::Scalar]
impl ScalarType for Polygon {
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
                    geo::Geometry::Polygon(x) => Ok(Self(x)),
                    _ => Err(InputValueError::custom("Got invalid type for Polygon")),
                }
            }
            _ => Err(InputValueError::custom(
                "parsing not implemented for this type (only string)",
            )),
        }
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
        S: serde::Serializer,
    {
        use geozero::ToJson;
        use serde::ser::{Error, SerializeMap};
        use serde_json::Value;
        use std::collections::BTreeMap;

        let s = geo::Geometry::Polygon(self.0.clone())
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
    use super::Polygon;
    use geo::LineString;

    async fn pg_roundtrip(data_to: &Polygon, type_name: &str) -> Polygon {
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
        .execute(&mut *conn)
        .await
        .unwrap();

        sqlx::query("INSERT INTO test (geom) VALUES ($1)")
            .bind(data_to)
            .execute(&mut *conn)
            .await
            .unwrap();

        let (data_from,): (Polygon,) = sqlx::query_as("SELECT geom FROM test")
            .fetch_one(&mut *conn)
            .await
            .unwrap();

        data_from
    }

    #[tokio::test]
    async fn polygon() {
        let polygon = geo::Polygon::<f64>::new(
            LineString::from(vec![(0., 0.), (1., 1.), (1., 0.), (0., 0.)]),
            vec![LineString::from(vec![
                (0.1, 0.1),
                (0.9, 0.9),
                (0.9, 0.1),
                (0.1, 0.1),
            ])],
        );
        let data_to = Polygon(polygon);
        let data_from = pg_roundtrip(&data_to, "Polygon").await;
        assert_eq!(data_to, data_from);
    }
}
