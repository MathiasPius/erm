use sqlx::any::AnyTypeInfo;

pub trait ColumnType {
    const SQL_TYPE: sqlx::postgres::any::AnyTypeInfo;
}

impl ColumnType for f32 {
    const SQL_TYPE: sqlx::postgres::any::AnyTypeInfo = AnyTypeInfo {
        kind: sqlx::postgres::any::AnyTypeInfoKind::Double,
    };
}

impl ColumnType for f64 {
    const SQL_TYPE: sqlx::postgres::any::AnyTypeInfo = AnyTypeInfo {
        kind: sqlx::postgres::any::AnyTypeInfoKind::Double,
    };
}
