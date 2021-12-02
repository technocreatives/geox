use std::ops::Deref;

#[cfg(feature = "sqlx")]
use geozero::wkb;

#[cfg(feature = "sqlx")]
use sqlx::{
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

#[cfg(feature = "serde")]
use serde::Serialize;

#[cfg(feature = "serde")]
impl Serialize for Geometry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        use geozero::ToJson;
        use std::collections::BTreeMap;
        use serde_json::Value;
        use serde::ser::{Error, SerializeMap};

        let s = self.0.to_json().map_err(Error::custom)?;
        let s: BTreeMap<String, Value> = serde_json::from_str(&s).map_err(Error::custom)?;

        let mut map = serializer.serialize_map(Some(s.len()))?;
        for (k, v) in s {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}
