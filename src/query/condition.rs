//! Query conditions for WHERE clauses

use crate::entity::ColumnTrait;
use crate::value::IntoValue;
use crate::value::Value;

/// A condition for filtering queries (WHERE clauses)
///
/// Conditions are used to filter query results. They can be created using
/// static methods like [`Condition::eq`], [`Condition::gt`], etc., and
/// combined using [`Condition::and`] and [`Condition::or`].
///
/// # Example
///
/// ```ignore
/// use tursorm::Condition;
///
/// // Simple equality
/// let cond = Condition::eq(UserColumn::Id, 1);
///
/// // Combined conditions
/// let cond = Condition::eq(UserColumn::Status, "active")
///     .and(Condition::gt(UserColumn::Age, 18));
/// ```
#[derive(Clone, Debug)]
pub struct Condition {
    pub(crate) sql:    String,
    pub(crate) values: Vec<Value>,
}

impl Condition {
    /// Create an equality condition: column = value
    pub fn eq<C: ColumnTrait, V: IntoValue>(column: C, value: V) -> Self {
        Self { sql: format!("{} = ?", column.name()), values: vec![value.into_value()] }
    }

    /// Create a not-equal condition: column != value
    pub fn ne<C: ColumnTrait, V: IntoValue>(column: C, value: V) -> Self {
        Self { sql: format!("{} != ?", column.name()), values: vec![value.into_value()] }
    }

    /// Create a greater-than condition: column > value
    pub fn gt<C: ColumnTrait, V: IntoValue>(column: C, value: V) -> Self {
        Self { sql: format!("{} > ?", column.name()), values: vec![value.into_value()] }
    }

    /// Create a greater-than-or-equal condition: column >= value
    pub fn gte<C: ColumnTrait, V: IntoValue>(column: C, value: V) -> Self {
        Self { sql: format!("{} >= ?", column.name()), values: vec![value.into_value()] }
    }

    /// Create a less-than condition: column < value
    pub fn lt<C: ColumnTrait, V: IntoValue>(column: C, value: V) -> Self {
        Self { sql: format!("{} < ?", column.name()), values: vec![value.into_value()] }
    }

    /// Create a less-than-or-equal condition: column <= value
    pub fn lte<C: ColumnTrait, V: IntoValue>(column: C, value: V) -> Self {
        Self { sql: format!("{} <= ?", column.name()), values: vec![value.into_value()] }
    }

    /// Create a LIKE condition: column LIKE pattern
    pub fn like<C: ColumnTrait>(column: C, pattern: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(pattern.into())] }
    }

    /// Create a NOT LIKE condition: column NOT LIKE pattern
    pub fn not_like<C: ColumnTrait>(column: C, pattern: impl Into<String>) -> Self {
        Self { sql: format!("{} NOT LIKE ?", column.name()), values: vec![Value::Text(pattern.into())] }
    }

    /// Create a contains condition (LIKE %value%)
    pub fn contains<C: ColumnTrait>(column: C, value: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(format!("%{}%", value.into()))] }
    }

    /// Create a starts-with condition (LIKE value%)
    pub fn starts_with<C: ColumnTrait>(column: C, value: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(format!("{}%", value.into()))] }
    }

    /// Create an ends-with condition (LIKE %value)
    pub fn ends_with<C: ColumnTrait>(column: C, value: impl Into<String>) -> Self {
        Self { sql: format!("{} LIKE ?", column.name()), values: vec![Value::Text(format!("%{}", value.into()))] }
    }

    /// Create an IS NULL condition
    pub fn is_null<C: ColumnTrait>(column: C) -> Self {
        Self { sql: format!("{} IS NULL", column.name()), values: vec![] }
    }

    /// Create an IS NOT NULL condition
    pub fn is_not_null<C: ColumnTrait>(column: C) -> Self {
        Self { sql: format!("{} IS NOT NULL", column.name()), values: vec![] }
    }

    /// Create an IN condition: column IN (values...)
    pub fn is_in<C: ColumnTrait, V: IntoValue>(column: C, values: Vec<V>) -> Self {
        let placeholders: Vec<&str> = values.iter().map(|_| "?").collect();
        Self {
            sql:    format!("{} IN ({})", column.name(), placeholders.join(", ")),
            values: values.into_iter().map(|v| v.into_value()).collect(),
        }
    }

    /// Create a NOT IN condition: column NOT IN (values...)
    pub fn not_in<C: ColumnTrait, V: IntoValue>(column: C, values: Vec<V>) -> Self {
        let placeholders: Vec<&str> = values.iter().map(|_| "?").collect();
        Self {
            sql:    format!("{} NOT IN ({})", column.name(), placeholders.join(", ")),
            values: values.into_iter().map(|v| v.into_value()).collect(),
        }
    }

    /// Create a BETWEEN condition: column BETWEEN low AND high
    pub fn between<C: ColumnTrait, V: IntoValue>(column: C, low: V, high: V) -> Self {
        Self { sql: format!("{} BETWEEN ? AND ?", column.name()), values: vec![low.into_value(), high.into_value()] }
    }

    /// Create a NOT BETWEEN condition: column NOT BETWEEN low AND high
    pub fn not_between<C: ColumnTrait, V: IntoValue>(column: C, low: V, high: V) -> Self {
        Self {
            sql:    format!("{} NOT BETWEEN ? AND ?", column.name()),
            values: vec![low.into_value(), high.into_value()],
        }
    }

    /// Create a raw SQL condition with values
    pub fn raw(sql: impl Into<String>, values: Vec<Value>) -> Self {
        Self { sql: sql.into(), values }
    }

    /// Combine two conditions with AND
    pub fn and(self, other: Condition) -> Self {
        let mut values = self.values;
        values.extend(other.values);
        Self { sql: format!("({}) AND ({})", self.sql, other.sql), values }
    }

    /// Combine two conditions with OR
    pub fn or(self, other: Condition) -> Self {
        let mut values = self.values;
        values.extend(other.values);
        Self { sql: format!("({}) OR ({})", self.sql, other.sql), values }
    }

    /// Negate the condition
    pub fn not(self) -> Self {
        Self { sql: format!("NOT ({})", self.sql), values: self.values }
    }

    /// Get the SQL string
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Get the values
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Take ownership of values
    pub fn into_values(self) -> Vec<Value> {
        self.values
    }
}

/// Order direction for ORDER BY clauses
///
/// Used with `Select::order_by` to specify the sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Ascending order (smallest to largest, A to Z)
    Asc,
    /// Descending order (largest to smallest, Z to A)
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

/// Order by clause for sorting query results
///
/// Represents a column and direction for ORDER BY clauses.
/// Created using [`OrderBy::asc`] or [`OrderBy::desc`].
#[derive(Clone, Debug)]
pub struct OrderBy {
    pub(crate) column:    String,
    pub(crate) direction: Order,
}

impl OrderBy {
    /// Create a new ascending order
    pub fn asc<C: ColumnTrait>(column: C) -> Self {
        Self { column: column.name().to_string(), direction: Order::Asc }
    }

    /// Create a new descending order
    pub fn desc<C: ColumnTrait>(column: C) -> Self {
        Self { column: column.name().to_string(), direction: Order::Desc }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::ColumnType;

    // Mock column for testing
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

    // Condition::eq tests
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

    // Condition::ne tests
    #[test]
    fn test_condition_ne() {
        let cond = Condition::ne(TestColumn::Id, 42);
        assert_eq!(cond.sql(), "id != ?");
        assert_eq!(cond.values()[0], Value::Integer(42));
    }

    // Condition::gt tests
    #[test]
    fn test_condition_gt() {
        let cond = Condition::gt(TestColumn::Age, 18);
        assert_eq!(cond.sql(), "age > ?");
        assert_eq!(cond.values()[0], Value::Integer(18));
    }

    // Condition::gte tests
    #[test]
    fn test_condition_gte() {
        let cond = Condition::gte(TestColumn::Age, 18);
        assert_eq!(cond.sql(), "age >= ?");
        assert_eq!(cond.values()[0], Value::Integer(18));
    }

    // Condition::lt tests
    #[test]
    fn test_condition_lt() {
        let cond = Condition::lt(TestColumn::Age, 65);
        assert_eq!(cond.sql(), "age < ?");
        assert_eq!(cond.values()[0], Value::Integer(65));
    }

    // Condition::lte tests
    #[test]
    fn test_condition_lte() {
        let cond = Condition::lte(TestColumn::Age, 65);
        assert_eq!(cond.sql(), "age <= ?");
        assert_eq!(cond.values()[0], Value::Integer(65));
    }

    // Condition::like tests
    #[test]
    fn test_condition_like() {
        let cond = Condition::like(TestColumn::Name, "%Alice%");
        assert_eq!(cond.sql(), "name LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%Alice%".to_string()));
    }

    // Condition::not_like tests
    #[test]
    fn test_condition_not_like() {
        let cond = Condition::not_like(TestColumn::Name, "%Bob%");
        assert_eq!(cond.sql(), "name NOT LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%Bob%".to_string()));
    }

    // Condition::contains tests
    #[test]
    fn test_condition_contains() {
        let cond = Condition::contains(TestColumn::Email, "@example.com");
        assert_eq!(cond.sql(), "email LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%@example.com%".to_string()));
    }

    // Condition::starts_with tests
    #[test]
    fn test_condition_starts_with() {
        let cond = Condition::starts_with(TestColumn::Name, "Al");
        assert_eq!(cond.sql(), "name LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("Al%".to_string()));
    }

    // Condition::ends_with tests
    #[test]
    fn test_condition_ends_with() {
        let cond = Condition::ends_with(TestColumn::Email, ".com");
        assert_eq!(cond.sql(), "email LIKE ?");
        assert_eq!(cond.values()[0], Value::Text("%.com".to_string()));
    }

    // Condition::is_null tests
    #[test]
    fn test_condition_is_null() {
        let cond = Condition::is_null(TestColumn::Email);
        assert_eq!(cond.sql(), "email IS NULL");
        assert!(cond.values().is_empty());
    }

    // Condition::is_not_null tests
    #[test]
    fn test_condition_is_not_null() {
        let cond = Condition::is_not_null(TestColumn::Email);
        assert_eq!(cond.sql(), "email IS NOT NULL");
        assert!(cond.values().is_empty());
    }

    // Condition::is_in tests
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

    // Condition::not_in tests
    #[test]
    fn test_condition_not_in() {
        let cond = Condition::not_in(TestColumn::Id, vec![1, 2]);
        assert_eq!(cond.sql(), "id NOT IN (?, ?)");
        assert_eq!(cond.values().len(), 2);
    }

    // Condition::between tests
    #[test]
    fn test_condition_between() {
        let cond = Condition::between(TestColumn::Age, 18, 65);
        assert_eq!(cond.sql(), "age BETWEEN ? AND ?");
        assert_eq!(cond.values().len(), 2);
        assert_eq!(cond.values()[0], Value::Integer(18));
        assert_eq!(cond.values()[1], Value::Integer(65));
    }

    // Condition::not_between tests
    #[test]
    fn test_condition_not_between() {
        let cond = Condition::not_between(TestColumn::Age, 0, 18);
        assert_eq!(cond.sql(), "age NOT BETWEEN ? AND ?");
        assert_eq!(cond.values().len(), 2);
    }

    // Condition::raw tests
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

    // Condition::and tests
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

    // Condition::or tests
    #[test]
    fn test_condition_or() {
        let cond1 = Condition::eq(TestColumn::Name, "Alice");
        let cond2 = Condition::eq(TestColumn::Name, "Bob");
        let combined = cond1.or(cond2);

        assert_eq!(combined.sql(), "(name = ?) OR (name = ?)");
        assert_eq!(combined.values().len(), 2);
    }

    // Condition::not tests
    #[test]
    fn test_condition_not() {
        let cond = Condition::eq(TestColumn::Id, 1).not();
        assert_eq!(cond.sql(), "NOT (id = ?)");
        assert_eq!(cond.values().len(), 1);
    }

    // Chained conditions
    #[test]
    fn test_condition_chained() {
        let cond = Condition::eq(TestColumn::Age, 25)
            .and(Condition::eq(TestColumn::Name, "Alice"))
            .or(Condition::eq(TestColumn::Id, 1));

        assert!(cond.sql().contains("AND"));
        assert!(cond.sql().contains("OR"));
        assert_eq!(cond.values().len(), 3);
    }

    // Accessor methods
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

    // Condition clone
    #[test]
    fn test_condition_clone() {
        let cond = Condition::eq(TestColumn::Id, 42);
        let cloned = cond.clone();
        assert_eq!(cloned.sql(), "id = ?");
        assert_eq!(cloned.values()[0], Value::Integer(42));
    }

    // Order tests
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

    // OrderBy tests
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
