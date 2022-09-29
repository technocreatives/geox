use std::ops::Deref;

#[cfg(feature = "sqlx")]
use geozero::{wkb, ToWkb};

#[cfg(feature = "sqlx")]
use sqlx::{
    encode::IsNull,
    postgres::{PgTypeInfo, PgValueRef},
    Postgres, ValueRef,
};

#[derive(Clone, Debug)]
pub struct Geometry(pub geo::Geometry<f64>);

impl PartialEq for Geometry {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Geometry {}

impl Deref for Geometry {
    type Target = geo::Geometry<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<Postgres> for Geometry {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("geometry")
    }
}

impl Geometry {
    pub fn into_inner(self) -> geo::Geometry<f64> {
        self.0
    }
}

#[cfg(feature = "sqlx")]
impl<'de> sqlx::Decode<'de, Postgres> for Geometry {
    fn decode(value: PgValueRef<'de>) -> Result<Self, sqlx::error::BoxDynError> {
        if value.is_null() {
            return Err(Box::new(sqlx::error::UnexpectedNullError));
        }
        let decode = wkb::Decode::<geo::Geometry<f64>>::decode(value)?;
        Ok(Geometry(decode.geometry.expect(
            "geometry parsing failed without error for non-null value",
        )))
    }
}

#[cfg(feature = "sqlx")]
impl<'en> sqlx::Encode<'en, Postgres> for Geometry {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> IsNull {
        let x = self
            .0
            .to_ewkb(geozero::CoordDimensions::xy(), None)
            .unwrap();
        buf.extend(x);
        sqlx::encode::IsNull::No
    }
}

#[cfg(feature = "serde1")]
impl serde::Serialize for Geometry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use geozero::ToJson;
        use serde::ser::{Error, SerializeMap};
        use serde_json::Value;
        use std::collections::BTreeMap;

        let s = self.0.to_json().map_err(Error::custom)?;
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
    use super::Geometry;
    use geo::{line_string, LineString, MultiLineString, Polygon};

    async fn pg_roundtrip(data_to: &Geometry, type_name: &str) -> Geometry {
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

        let (data_from,): (Geometry,) = sqlx::query_as("SELECT geom FROM test")
            .fetch_one(&mut conn)
            .await
            .unwrap();

        data_from
    }

    #[tokio::test]
    async fn point() {
        let data_to = Geometry(geo::Geometry::Point((0., 1.).into()));
        let data_from = pg_roundtrip(&data_to, "Point").await;
        assert_eq!(data_to, data_from);
    }

    #[tokio::test]
    async fn line() {
        let open_line_string: LineString<f64> = line_string![(x: 0., y: 0.), (x: 5., y: 0.)];
        let data_to = Geometry(geo::Geometry::MultiLineString(MultiLineString(vec![
            open_line_string,
        ])));
        let data_from = pg_roundtrip(&data_to, "MultiLineString").await;
        assert_eq!(data_to, data_from);
    }

    #[tokio::test]
    async fn polygon() {
        let polygon = Polygon::new(
            LineString::from(vec![(0., 0.), (1., 1.), (1., 0.), (0., 0.)]),
            vec![LineString::from(vec![
                (0.1, 0.1),
                (0.9, 0.9),
                (0.9, 0.1),
                (0.1, 0.1),
            ])],
        );
        let data_to = Geometry(geo::Geometry::Polygon(polygon));
        let data_from = pg_roundtrip(&data_to, "Polygon").await;
        assert_eq!(data_to, data_from);
    }
}
