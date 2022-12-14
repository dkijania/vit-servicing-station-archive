use crate::db::DbConnection;

embed_migrations!("../resources/migrations");

pub fn initialize_db_with_migration(db_conn: &DbConnection) {
    embedded_migrations::run(db_conn).unwrap();
}
