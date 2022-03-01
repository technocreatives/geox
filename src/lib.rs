mod geometry;
mod point;
mod polygon;

pub use geometry::Geometry;
pub use point::Point;
pub use polygon::Polygon;

#[cfg(all(test, feature = "sqlx"))]
#[tokio::test]
async fn quickcheck_ma_postgres() {
    use sqlx::postgres::PgPoolOptions;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:password@localhost/postgres")
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();

    sqlx::query("CREATE TABLE test ( id SERIAL PRIMARY KEY, geom GEOMETRY(Point, 26910) )")
        .execute(&mut tx)
        .await
        .unwrap();

    let data = Geometry(geo::Geometry::Point((0., 1.).into()));
    sqlx::query("INSERT INTO test (geom) VALUE (?)")
        .bind(data)
        .execute(&mut tx)
        .await
        .unwrap();

    let row: (Geometry,) = sqlx::query_as("SELECT geom FROM test")
        .fetch_one(&pool)
        .await
        .unwrap();

    println!("{row:?}");
}
