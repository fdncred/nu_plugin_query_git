use chrono::{TimeZone, Utc};
use gitql_core::object::{GitQLObject, Row};
use nu_protocol::{Record, Value as NuValue};

pub fn render_objects(groups: &mut GitQLObject) -> NuValue {
    if groups.len() > 1 {
        groups.flat();
    }

    if groups.is_empty() || groups.groups[0].is_empty() {
        return NuValue::test_string("No data to display".to_string());
    }

    let gql_group = &groups.groups[0];
    print_group_as_table(&groups.titles, &gql_group.rows)
}

fn print_group_as_table(titles: &[String], rows: &[Row]) -> NuValue {
    let mut table_row_val: Vec<NuValue> = Vec::with_capacity(rows.len());

    for row in rows {
        let mut rec = Record::new();
        for (column_name, column_value) in titles.iter().zip(&row.values) {
            // eprintln!("{column_name:#?} - {:#?}", column_value.as_text());
            // let dt = column_value.data_type();
            // if let Some(int_val) = column_value.as_any().downcast_ref::<IntValue>() {
            //     rec.insert(column_name, NuValue::Int(int_val.value));
            // }

            // use super::array::ArrayValue;
            // use super::boolean::BoolValue;
            // use super::date::DateValue;
            // use super::datetime::DateTimeValue;
            // use super::float::FloatValue;
            // use super::integer::IntValue;
            // use super::null::NullValue;
            // use super::range::RangeValue;
            // use super::text::TextValue;
            // use super::time::TimeValue;
            // }
            match column_value {
                v if v.is_array() => {
                    if let Some(array_val) = v.as_array() {
                        let array_str = array_val
                            // .values
                            .iter()
                            .map(|v| v.literal())
                            .collect::<Vec<_>>()
                            .join(",");
                        rec.insert(column_name, NuValue::test_string(array_str));
                    }
                }
                v if v.is_bool() => {
                    if let Some(boolean) = v.as_bool() {
                        rec.insert(column_name, NuValue::test_bool(boolean));
                    }
                }
                v if v.is_date() => {
                    if let Some(date) = v.as_date() {
                        rec.insert(column_name, NuValue::test_string(date.to_string()));
                    }
                }
                v if v.is_date_time() => {
                    if let Some(date_time) = v.as_date_time() {
                        let timestamp = date_time * 1_000_000_000;
                        let dt = Utc.timestamp_nanos(timestamp);
                        rec.insert(column_name, NuValue::test_date(dt.into()));
                    }
                }
                v if v.is_float() => {
                    if let Some(float) = v.as_float() {
                        rec.insert(column_name, NuValue::test_float(float));
                    }
                }
                v if v.as_range().is_some() => {
                    if let Some(range) = v.as_range() {
                        rec.insert(
                            column_name,
                            NuValue::test_string(format!("{}..{}", range.0, range.1)),
                        );
                    }
                }
                v if v.is_null() => {
                    rec.insert(column_name, NuValue::test_nothing());
                }
                v if v.is_int() => {
                    if let Some(int_value) = v.as_int() {
                        rec.insert(column_name, NuValue::test_int(int_value));
                    }
                }
                v if v.is_text() => {
                    if let Some(text) = v.as_text() {
                        rec.insert(column_name, NuValue::test_string(text));
                    }
                }
                v if v.is_time() => {
                    if let Some(time) = v.as_time() {
                        rec.insert(column_name, NuValue::test_string(time));
                    }
                }

                _ => {}
            }
        }
        table_row_val.push(NuValue::test_record(rec));
    }

    NuValue::test_list(table_row_val)
}

pub fn render_groups_to_json(groups: &mut GitQLObject) -> Option<String> {
    let mut elements: Vec<serde_json::Value> = vec![];

    if let Some(group) = groups.groups.first() {
        let titles = &groups.titles;
        for row in &group.rows {
            let mut object = serde_json::Map::new();
            for (i, value) in row.values.iter().enumerate() {
                object.insert(
                    titles[i].clone(),
                    serde_json::Value::String(value.literal()),
                );
            }
            elements.push(serde_json::Value::Object(object));
        }
    }

    serde_json::to_string(&serde_json::Value::Array(elements)).ok()
}

pub fn render_groups_to_csv(groups: &mut GitQLObject) -> Option<String> {
    let mut writer = csv::Writer::from_writer(vec![]);
    let _ = writer.write_record(groups.titles.clone());
    let row_len = groups.titles.len();
    if let Some(group) = groups.groups.first() {
        for row in &group.rows {
            let mut values_row: Vec<String> = Vec::with_capacity(row_len);
            for value in &row.values {
                values_row.push(value.literal());
            }
            let _ = writer.write_record(values_row);
        }
    }

    writer
        .into_inner()
        .ok()
        .and_then(|writer_content| String::from_utf8(writer_content).ok())
}

pub fn render_groups_to_yaml(groups: &mut GitQLObject) -> Option<String> {
    let mut elements: Vec<serde_yaml::Value> = Vec::new();

    if let Some(group) = groups.groups.first() {
        let titles = &groups.titles;
        for row in &group.rows {
            let mut object = serde_yaml::Mapping::new();
            for (i, value) in row.values.iter().enumerate() {
                object.insert(
                    serde_yaml::Value::String(titles[i].clone()),
                    serde_yaml::Value::String(value.literal()),
                );
            }
            elements.push(serde_yaml::Value::Mapping(object));
        }
    }

    serde_yaml::to_string(&elements).ok()
}
