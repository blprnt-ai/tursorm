use crate::ColumnTrait;
use crate::IntoValue;
use crate::Value;

#[derive(Clone, Debug)]
pub struct Condition {
    pub(crate) sql:    String,
    pub(crate) values: Vec<Value>,
}

impl Condition {
    pub fn eq<Column: ColumnTrait, V: IntoValue>(column: Column, value: V) -> Self {
        Self { sql: format!("{} = ?", column.name()), values: vec![value.into_value()] }
    }

    pub fn ne<Column: ColumnTrait, V: IntoValue>(column: Column, value: V) -> Self {
        Self { sql: format!("{} != ?", column.name()), values: vec![value.into_value()] }
    }

    pub fn gt<Column: ColumnTrait, V: IntoValue>(column: Column, value: V) -> Self {
        Self { sql: format!("{} > ?", column.name()), values: vec![value.into_value()] }
    }

    pub fn gte<Column: ColumnTrait, V: IntoValue>(column: Column, value: V) -> Self {
        Self { sql: format!("{} >= ?", column.name()), values: vec![value.into_value()] }
    }

    pub fn lt<Column: ColumnTrait, V: IntoValue>(column: Column, value: V) -> Self {
        Self { sql: format!("{} < ?", column.name()), values: vec![value.into_value()] }
    }

    pub fn lte<Column: ColumnTrait, V: IntoValue>(column: Column, value: V) -> Self {
        Self { sql: format!("{} <= ?", column.name()), values: vec![value.into_value()] }
    }

    pub fn like<Column: ColumnTrait>(column: Column, pattern: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(pattern.into())] }
    }

    pub fn not_like<Column: ColumnTrait>(column: Column, pattern: impl Into<String>) -> Self {
        Self { sql: format!("{} NOT LIKE ?", column.name()), values: vec![Value::Text(pattern.into())] }
    }

    pub fn contains<Column: ColumnTrait>(column: Column, value: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(format!("%{}%", value.into()))] }
    }

    pub fn starts_with<Column: ColumnTrait>(column: Column, value: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(format!("{}%", value.into()))] }
    }

    pub fn ends_with<Column: ColumnTrait>(column: Column, value: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(format!("%{}", value.into()))] }
    }

    pub fn is_null<Column: ColumnTrait>(column: Column) -> Self {
        Self { sql: format!("{} IS NULL", column.name()), values: vec![] }
    }

    pub fn is_not_null<Column: ColumnTrait>(column: Column) -> Self {
        Self { sql: format!("{} IS NOT NULL", column.name()), values: vec![] }
    }

    pub fn is_in<Column: ColumnTrait, V: IntoValue>(column: Column, values: Vec<V>) -> Self {
        let placeholders: Vec<&str> = values.iter().map(|_| "?").collect();
        Self {
            sql:    format!("{} IN ({})", column.name(), placeholders.join(", ")),
            values: values.into_iter().map(|v| v.into_value()).collect(),
        }
    }

    pub fn not_in<Column: ColumnTrait, V: IntoValue>(column: Column, values: Vec<V>) -> Self {
        let placeholders: Vec<&str> = values.iter().map(|_| "?").collect();
        Self {
            sql:    format!("{} NOT IN ({})", column.name(), placeholders.join(", ")),
            values: values.into_iter().map(|v| v.into_value()).collect(),
        }
    }

    pub fn between<Column: ColumnTrait, V: IntoValue>(column: Column, low: V, high: V) -> Self {
        Self { sql: format!("{} BETWEEN ? AND ?", column.name()), values: vec![low.into_value(), high.into_value()] }
    }

    pub fn not_between<Column: ColumnTrait, V: IntoValue>(column: Column, low: V, high: V) -> Self {
        Self {
            sql:    format!("{} NOT BETWEEN ? AND ?", column.name()),
            values: vec![low.into_value(), high.into_value()],
        }
    }

    pub fn raw(sql: impl Into<String>, values: Vec<Value>) -> Self {
        Self { sql: sql.into(), values }
    }

    pub fn and(self, other: Condition) -> Self {
        let mut values = self.values;
        values.extend(other.values);
        Self { sql: format!("({}) AND ({})", self.sql, other.sql), values }
    }

    pub fn or(self, other: Condition) -> Self {
        let mut values = self.values;
        values.extend(other.values);
        Self { sql: format!("({}) OR ({})", self.sql, other.sql), values }
    }

    pub fn not(self) -> Self {
        Self { sql: format!("NOT ({})", self.sql), values: self.values }
    }

    pub fn sql(&self) -> &str {
        &self.sql
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }

    pub fn into_values(self) -> Vec<Value> {
        self.values
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Asc,

    Desc,
}

impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Order::Asc => write!(f, "ASC"),
            Order::Desc => write!(f, "DESC"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrderBy {
    pub(crate) column:    String,
    pub(crate) direction: Order,
}

impl OrderBy {
    pub fn asc<Column: ColumnTrait>(column: Column) -> Self {
        Self { column: column.name().to_string(), direction: Order::Asc }
    }

    pub fn desc<Column: ColumnTrait>(column: Column) -> Self {
        Self { column: column.name().to_string(), direction: Order::Desc }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::ColumnType;

    #[derive(Clone, Copy, Debug)]
    enum TestColumn {
        Id,
        Name,
        Email,
        Age,
    }

    impl std::fmt::Display for TestColumn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    impl ColumnTrait for TestColumn {
        fn name(&self) -> &'static str {
            match self {
                TestColumn::Id => "id",
                TestColumn::Name => "name",
                TestColumn::Email => "email",
                TestColumn::Age => "age",
            }
        }

        fn column_type(&self) -> ColumnType {
            match self {
                TestColumn::Id | TestColumn::Age => ColumnType::Integer,
                TestColumn::Name | TestColumn::Email => ColumnType::Text,
            }
        }

        fn all() -> &'static [Self] {
            &[TestColumn::Id, TestColumn::Name, TestColumn::Email, TestColumn::Age]
        }
    }

    #[test]
    fn test_condition_eq() {
        let cond = Condition::eq(TestColumn::Id, 42);
        assert_eq!(cond.sql(), "id = ?");
        assert_eq!(cond.values().len(), 1);
        assert_eq!(cond.values()[0], Value::Integer(42));
    }

    #[test]
    fn test_condition_eq_with_string() {
        let cond = Condition::eq(TestColumn::Name, "Alice");
        assert_eq!(cond.sql(), "name = ?");
        assert_eq!(cond.values()[0], Value::Text("Alice".to_string()));
    }

    #[test]
    fn test_condition_ne() {
        let cond = Condition::ne(TestColumn::Id, 42);
        assert_eq!(cond.sql(), "id != ?");
        assert_eq!(cond.values()[0], Value::Integer(42));
    }

    #[test]
    fn test_condition_gt() {
        let cond = Condition::gt(TestColumn::Age, 18);
        assert_eq!(cond.sql(), "age > ?");
        assert_eq!(cond.values()[0], Value::Integer(18));
    }

    #[test]
    fn test_condition_gte() {
        let cond = Condition::gte(TestColumn::Age, 18);
        assert_eq!(cond.sql(), "age >= ?");
        assert_eq!(cond.values()[0], Value::Integer(18));
    }

    #[test]
    fn test_condition_lt() {
        let cond = Condition::lt(TestColumn::Age, 65);
        assert_eq!(cond.sql(), "age < ?");
        assert_eq!(cond.values()[0], Value::Integer(65));
    }

    #[test]
    fn test_condition_lte() {
        let cond = Condition::lte(TestColumn::Age, 65);
        assert_eq!(cond.sql(), "age <= ?");
        assert_eq!(cond.values()[0], Value::Integer(65));
    }

    #[test]
    fn test_condition_like() {
        let cond = Condition::like(TestColumn::Name, "%Alice%");
        assert_eq!(cond.sql(), "name LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%Alice%".to_string()));
    }

    #[test]
    fn test_condition_not_like() {
        let cond = Condition::not_like(TestColumn::Name, "%Bob%");
        assert_eq!(cond.sql(), "name NOT LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%Bob%".to_string()));
    }

    #[test]
    fn test_condition_contains() {
        let cond = Condition::contains(TestColumn::Email, "@example.com");
        assert_eq!(cond.sql(), "email LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%@example.com%".to_string()));
    }

    #[test]
    fn test_condition_starts_with() {
        let cond = Condition::starts_with(TestColumn::Name, "Al");
        assert_eq!(cond.sql(), "name LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("Al%".to_string()));
    }

    #[test]
    fn test_condition_ends_with() {
        let cond = Condition::ends_with(TestColumn::Email, ".com");
        assert_eq!(cond.sql(), "email LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%.com".to_string()));
    }

    #[test]
    fn test_condition_is_null() {
        let cond = Condition::is_null(TestColumn::Email);
        assert_eq!(cond.sql(), "email IS NULL");
        assert!(cond.values().is_empty());
    }

    #[test]
    fn test_condition_is_not_null() {
        let cond = Condition::is_not_null(TestColumn::Email);
        assert_eq!(cond.sql(), "email IS NOT NULL");
        assert!(cond.values().is_empty());
    }

    #[test]
    fn test_condition_is_in() {
        let cond = Condition::is_in(TestColumn::Id, vec![1, 2, 3]);
        assert_eq!(cond.sql(), "id IN (?, ?, ?)");
        assert_eq!(cond.values().len(), 3);
        assert_eq!(cond.values()[0], Value::Integer(1));
        assert_eq!(cond.values()[1], Value::Integer(2));
        assert_eq!(cond.values()[2], Value::Integer(3));
    }

    #[test]
    fn test_condition_is_in_empty() {
        let cond = Condition::is_in(TestColumn::Id, Vec::<i64>::new());
        assert_eq!(cond.sql(), "id IN ()");
        assert!(cond.values().is_empty());
    }

    #[test]
    fn test_condition_is_in_single() {
        let cond = Condition::is_in(TestColumn::Id, vec![42]);
        assert_eq!(cond.sql(), "id IN (?)");
        assert_eq!(cond.values().len(), 1);
    }

    #[test]
    fn test_condition_not_in() {
        let cond = Condition::not_in(TestColumn::Id, vec![1, 2]);
        assert_eq!(cond.sql(), "id NOT IN (?, ?)");
        assert_eq!(cond.values().len(), 2);
    }

    #[test]
    fn test_condition_between() {
        let cond = Condition::between(TestColumn::Age, 18, 65);
        assert_eq!(cond.sql(), "age BETWEEN ? AND ?");
        assert_eq!(cond.values().len(), 2);
        assert_eq!(cond.values()[0], Value::Integer(18));
        assert_eq!(cond.values()[1], Value::Integer(65));
    }

    #[test]
    fn test_condition_not_between() {
        let cond = Condition::not_between(TestColumn::Age, 0, 18);
        assert_eq!(cond.sql(), "age NOT BETWEEN ? AND ?");
        assert_eq!(cond.values().len(), 2);
    }

    #[test]
    fn test_condition_raw() {
        let cond = Condition::raw("id > ? AND age < ?", vec![Value::Integer(5), Value::Integer(30)]);
        assert_eq!(cond.sql(), "id > ? AND age < ?");
        assert_eq!(cond.values().len(), 2);
    }

    #[test]
    fn test_condition_raw_no_values() {
        let cond = Condition::raw("id IS NOT NULL", vec![]);
        assert_eq!(cond.sql(), "id IS NOT NULL");
        assert!(cond.values().is_empty());
    }

    #[test]
    fn test_condition_and() {
        let cond1 = Condition::eq(TestColumn::Id, 1);
        let cond2 = Condition::eq(TestColumn::Name, "Alice");
        let combined = cond1.and(cond2);

        assert_eq!(combined.sql(), "(id = ?) AND (name = ?)");
        assert_eq!(combined.values().len(), 2);
        assert_eq!(combined.values()[0], Value::Integer(1));
        assert_eq!(combined.values()[1], Value::Text("Alice".to_string()));
    }

    #[test]
    fn test_condition_or() {
        let cond1 = Condition::eq(TestColumn::Name, "Alice");
        let cond2 = Condition::eq(TestColumn::Name, "Bob");
        let combined = cond1.or(cond2);

        assert_eq!(combined.sql(), "(name = ?) OR (name = ?)");
        assert_eq!(combined.values().len(), 2);
    }

    #[test]
    fn test_condition_not() {
        let cond = Condition::eq(TestColumn::Id, 1).not();
        assert_eq!(cond.sql(), "NOT (id = ?)");
        assert_eq!(cond.values().len(), 1);
    }

    #[test]
    fn test_condition_chained() {
        let cond = Condition::eq(TestColumn::Age, 25)
            .and(Condition::eq(TestColumn::Name, "Alice"))
            .or(Condition::eq(TestColumn::Id, 1));

        assert!(cond.sql().contains("AND"));
        assert!(cond.sql().contains("OR"));
        assert_eq!(cond.values().len(), 3);
    }

    #[test]
    fn test_condition_sql() {
        let cond = Condition::eq(TestColumn::Id, 1);
        assert_eq!(cond.sql(), "id = ?");
    }

    #[test]
    fn test_condition_values() {
        let cond = Condition::between(TestColumn::Age, 18, 65);
        let values = cond.values();
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_condition_into_values() {
        let cond = Condition::between(TestColumn::Age, 18, 65);
        let values = cond.into_values();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Integer(18));
        assert_eq!(values[1], Value::Integer(65));
    }

    #[test]
    fn test_condition_clone() {
        let cond = Condition::eq(TestColumn::Id, 42);
        let cloned = cond.clone();
        assert_eq!(cloned.sql(), "id = ?");
        assert_eq!(cloned.values()[0], Value::Integer(42));
    }

    #[test]
    fn test_order_asc() {
        assert_eq!(Order::Asc, Order::Asc);
        assert_ne!(Order::Asc, Order::Desc);
    }

    #[test]
    fn test_order_desc() {
        assert_eq!(Order::Desc, Order::Desc);
        assert_ne!(Order::Desc, Order::Asc);
    }

    #[test]
    fn test_order_display() {
        assert_eq!(format!("{}", Order::Asc), "ASC");
        assert_eq!(format!("{}", Order::Desc), "DESC");
    }

    #[test]
    fn test_order_debug() {
        assert_eq!(format!("{:?}", Order::Asc), "Asc");
        assert_eq!(format!("{:?}", Order::Desc), "Desc");
    }

    #[test]
    fn test_order_clone() {
        let order = Order::Asc;
        let cloned = order.clone();
        assert_eq!(order, cloned);
    }

    #[test]
    fn test_order_copy() {
        let order = Order::Desc;
        let copied = order;
        assert_eq!(order, copied);
    }

    #[test]
    fn test_order_by_asc() {
        let order_by = OrderBy::asc(TestColumn::Name);
        assert_eq!(order_by.column, "name");
        assert_eq!(order_by.direction, Order::Asc);
    }

    #[test]
    fn test_order_by_desc() {
        let order_by = OrderBy::desc(TestColumn::Age);
        assert_eq!(order_by.column, "age");
        assert_eq!(order_by.direction, Order::Desc);
    }

    #[test]
    fn test_order_by_clone() {
        let order_by = OrderBy::asc(TestColumn::Id);
        let cloned = order_by.clone();
        assert_eq!(cloned.column, "id");
        assert_eq!(cloned.direction, Order::Asc);
    }

    #[test]
    fn test_order_by_debug() {
        let order_by = OrderBy::desc(TestColumn::Email);
        let debug = format!("{:?}", order_by);
        assert!(debug.contains("email"));
        assert!(debug.contains("Desc"));
    }
}
