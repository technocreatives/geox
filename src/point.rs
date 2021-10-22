use std::{convert::TryFrom, ops::Deref};

use sqlx::{
    postgres::{PgTypeInfo, PgValueRef},
    Postgres,
};

use crate::Geometry;

#[derive(Clone, Debug)]
pub struct Point(geo::Point<f64>);

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

impl<'de> sqlx::Decode<'de, Postgres> for Point {
    fn decode(value: PgValueRef<'de>) -> Result<Self, sqlx::error::BoxDynError> {
        let geometry = Geometry::decode(value)?;
        let point = geo::Point::<f64>::try_from(geometry.0)?;
        Ok(Point(point))
    }
}
