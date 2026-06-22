use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    domain::events::{PermissionOption, RunEvent},
    infrastructure::acp::util::clean_tool_title,
    ports::permission::PermissionDecisionPort,
};

pub async fn request_permission<P, F>(
    params: Value,
    run_id: &str,
    auto_allow: bool,
    permission_decisions: &P,
    emit: F,
) -> Result<Value>
where
    P: PermissionDecisionPort,
    F: Fn(RunEvent),
{
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("permission request missing options"))?;
    let tool_call = params.get("toolCall").cloned().unwrap_or(Value::Null);
    let title = clean_tool_title(tool_call.get("title").and_then(Value::as_str));
    let mapped_options = options
        .iter()
        .map(|option| PermissionOption {
            name: option
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            kind: option
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            option_id: option
                .get("optionId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
        .collect::<Vec<_>>();
    let permission_id = Uuid::new_v4().to_string();
    let selected = if auto_allow {
        select_permission_option(options, true)
            .ok_or_else(|| anyhow!("No allow permission option was offered by the agent."))?
            .to_owned()
    } else {
        let receiver = permission_decisions
            .create_waiter(run_id.to_string(), permission_id.clone())
            .await;
        emit(RunEvent::Permission {
            permission_id: Some(permission_id.clone()),
            title: title.clone(),
            input: tool_call.get("rawInput").cloned(),
            options: mapped_options.clone(),
            selected: None,
            requires_response: true,
        });
        let decision = receiver
            .await
            .map_err(|_| anyhow!("permission response channel closed"))?;
        let selected = options
            .iter()
            .find(|option| {
                option.get("optionId").and_then(Value::as_str) == Some(decision.option_id.as_str())
            })
            .ok_or_else(|| anyhow!("permission response selected an unknown option"))?;
        selected.to_owned()
    };
    let selected_name = selected
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let option_id = selected
        .get("optionId")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("selected permission option is missing optionId"))?;
    emit(RunEvent::Permission {
        permission_id: if auto_allow {
            None
        } else {
            Some(permission_id)
        },
        title,
        input: tool_call.get("rawInput").cloned(),
        options: mapped_options,
        selected: Some(selected_name),
        requires_response: false,
    });
    Ok(json!({"outcome": {"outcome": "selected", "optionId": option_id}}))
}

fn select_permission_option(options: &[Value], auto_allow: bool) -> Option<&Value> {
    if auto_allow {
        for desired in ["allow_once", "allow_always"] {
            if let Some(option) = options
                .iter()
                .find(|option| option.get("kind").and_then(Value::as_str) == Some(desired))
            {
                return Some(option);
            }
        }
    }
    options.iter().find(|option| {
        option
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.starts_with("allow"))
    })
}

#[cfg(test)]
mod tests {
    use super::select_permission_option;
    use serde_json::json;

    #[test]
    fn auto_allow_prefers_allow_once_before_allow_always() {
        let options = vec![
            json!({"kind": "allow_always", "optionId": "always"}),
            json!({"kind": "allow_once", "optionId": "once"}),
        ];

        let selected = select_permission_option(&options, true).expect("permission option");

        assert_eq!(
            selected.get("optionId").and_then(|value| value.as_str()),
            Some("once")
        );
    }

    #[test]
    fn manual_mode_still_selects_first_allow_option_as_fallback() {
        let options = vec![
            json!({"kind": "reject_once", "optionId": "reject"}),
            json!({"kind": "allow_always", "optionId": "always"}),
        ];

        let selected = select_permission_option(&options, false).expect("permission option");

        assert_eq!(
            selected.get("optionId").and_then(|value| value.as_str()),
            Some("always")
        );
    }
}
