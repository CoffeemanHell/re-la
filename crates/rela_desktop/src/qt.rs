//! JSON <-> Qt values, the way QJsonArray::toVariantList() turned relay data into what QML reads.
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use serde_json::Value;

/// null -> invalid QVariant, numbers -> qint64 when whole else double, arrays -> QVariantList, objects -> QVariantMap.
pub fn to_qvariant(v: &Value) -> QVariant {
    match v {
        Value::Null => QVariant::default(),
        Value::Bool(b) => QVariant::from(b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => QVariant::from(&i),
            None => QVariant::from(&n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => QVariant::from(&QString::from(s.as_str())),
        Value::Array(a) => {
            let list: QList<QVariant> = a.iter().map(to_qvariant).collect();
            QVariant::from(&list)
        }
        Value::Object(o) => {
            let mut map = QMap::<QMapPair_QString_QVariant>::default();
            for (k, v) in o {
                map.insert(QString::from(k.as_str()), to_qvariant(v));
            }
            QVariant::from(&map)
        }
    }
}

/// A JSON object as a QVariantMap (anything else gives an empty map).
pub fn to_qvariant_map(v: &Value) -> QMap<QMapPair_QString_QVariant> {
    to_qvariant(v).value().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_becomes_variant_maps_and_lists() {
        let v = to_qvariant(&json!({"user": "a", "following": null, "playing": true, "n": 3, "x": 1.5, "l": ["b"]}));
        let m: QMap<QMapPair_QString_QVariant> = v.value().unwrap();
        assert_eq!(m.get(&QString::from("user")).unwrap().value::<QString>().unwrap().to_string(), "a");
        assert!(m.get(&QString::from("following")).unwrap().is_null());
        assert!(m.get(&QString::from("playing")).unwrap().value::<bool>().unwrap());
        assert_eq!(m.get(&QString::from("n")).unwrap().value::<i64>().unwrap(), 3);
        assert_eq!(m.get(&QString::from("x")).unwrap().value::<f64>().unwrap(), 1.5);
        let l: QList<QVariant> = m.get(&QString::from("l")).unwrap().value().unwrap();
        assert_eq!(l.get(0).unwrap().value::<QString>().unwrap().to_string(), "b");
    }

    #[test]
    fn objects_become_maps_and_the_rest_empty_maps() {
        assert_eq!(to_qvariant_map(&json!({"a": 1})).len(), 1);
        assert_eq!(to_qvariant_map(&json!([1])).len(), 0);
        assert_eq!(to_qvariant_map(&json!({})).len(), 0);
    }
}
