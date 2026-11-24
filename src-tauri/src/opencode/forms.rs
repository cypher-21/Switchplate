use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub type FormAnswer = Map<String, Value>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormId {
    pub id: String,
    #[serde(alias = "sessionID")]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeForm {
    #[serde(flatten)]
    pub key: FormId,
    pub title: String,
    pub fields: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FormCreated {
    pub data: CreatedData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatedData {
    pub form: OpenCodeForm,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FormSettled {
    pub data: FormId,
}

#[cfg(test)]
mod tests {
    use crate::opencode::events::OpenCodeEvent;
    use serde_json::json;

    #[test]
    fn question_forms_keep_the_server_field_keys_and_values() {
        let fields = json!([
            { "key": "q0", "type": "string", "options": [{ "value": "email", "label": "Email" }] },
            { "key": "q1", "type": "multiselect", "custom": true }
        ]);
        let created = serde_json::from_value::<OpenCodeEvent>(json!({
            "type": "form.created",
            "data": { "form": { "id": "frm_1", "sessionID": "ses_1", "title": "Questions", "fields": fields } }
        })).unwrap();
        let OpenCodeEvent::FormCreated(created) = created else {
            panic!("expected a form creation event");
        };
        assert_eq!(created.data.form.key.session_id, "ses_1");
        assert_eq!(
            serde_json::to_value(&created.data.form.fields).unwrap(),
            fields
        );
        let settled = serde_json::from_value::<OpenCodeEvent>(json!({
            "type": "form.replied", "data": { "id": "frm_1", "sessionID": "ses_1", "answer": { "q0": "email" } }
        })).unwrap();
        let OpenCodeEvent::FormSettled(settled) = settled else {
            panic!("expected a form settlement event");
        };
        assert_eq!(created.data.form.key, settled.data);
    }
}
