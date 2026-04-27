use leptos::*;

const DEFAULT_FIELD_POLICY_JSON: &str = "{\n  \"columns\": {},\n  \"default\": \"keep\"\n}";
const IDLE_SUMMARY: &str = "Awaiting submission.";
const IDLE_REVIEW_QUEUE: &str = "No review items yet.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    CsvText,
    XlsxBase64,
}

impl InputMode {
    fn from_select_value(value: &str) -> Self {
        match value {
            "xlsx-base64" => Self::XlsxBase64,
            _ => Self::CsvText,
        }
    }

    fn select_value(self) -> &'static str {
        match self {
            Self::CsvText => "csv-text",
            Self::XlsxBase64 => "xlsx-base64",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
        }
    }

    fn payload_hint(self) -> &'static str {
        match self {
            Self::CsvText => "Paste CSV rows here",
            Self::XlsxBase64 => "Paste base64-encoded XLSX content here",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TabularFlowState {
    input_mode: InputMode,
    payload: String,
    field_policy_json: String,
    result_output: String,
    summary: String,
    review_queue: String,
    error_banner: Option<String>,
}

impl Default for TabularFlowState {
    fn default() -> Self {
        Self {
            input_mode: InputMode::CsvText,
            payload: String::new(),
            field_policy_json: DEFAULT_FIELD_POLICY_JSON.to_string(),
            result_output: String::new(),
            summary: IDLE_SUMMARY.to_string(),
            review_queue: IDLE_REVIEW_QUEUE.to_string(),
            error_banner: None,
        }
    }
}

impl TabularFlowState {
    fn clear_generated_state(&mut self) {
        self.result_output.clear();
        self.summary = IDLE_SUMMARY.to_string();
        self.review_queue = IDLE_REVIEW_QUEUE.to_string();
        self.error_banner = None;
    }

    fn submit(&mut self) {
        if self.payload.trim().is_empty() {
            self.clear_generated_state();
            self.error_banner = Some(format!(
                "{} payload is required before submitting.",
                self.input_mode.label()
            ));
            return;
        }

        if self.field_policy_json.trim().is_empty() {
            self.clear_generated_state();
            self.error_banner =
                Some("Field policy JSON is required before submitting.".to_string());
            return;
        }

        self.error_banner = None;
        self.result_output = format!(
            "// rewritten output preview\n// mode: {}\n{}",
            self.input_mode.label(),
            self.payload.trim()
        );
        self.summary = format!(
            "Shell submission captured for {} with {} payload characters.",
            self.input_mode.label(),
            self.payload.chars().count()
        );
        self.review_queue = "Review queue preview:\n- No flagged cells yet. Wire runtime review output in a later slice.".to_string();
    }
}

#[component]
pub fn App() -> impl IntoView {
    let state = create_rw_signal(TabularFlowState::default());

    let on_mode_change = move |event| {
        let next_mode = InputMode::from_select_value(&event_target_value(&event));
        state.update(|state| {
            state.input_mode = next_mode;
            state.clear_generated_state();
        });
    };

    let on_payload_input = move |event| {
        let next_payload = event_target_value(&event);
        state.update(|state| {
            state.payload = next_payload;
            state.clear_generated_state();
        });
    };

    let on_field_policy_input = move |event| {
        let next_policy = event_target_value(&event);
        state.update(|state| {
            state.field_policy_json = next_policy;
            state.clear_generated_state();
        });
    };

    let on_submit = move |_| {
        state.update(TabularFlowState::submit);
    };

    view! {
        <main class="tabular-flow-shell">
            <h1>"med-de-id browser tool"</h1>
            <p>"Bounded tabular deidentification flow"</p>

            <section>
                <h2>"Input"</h2>
                <label>
                    "Input mode"
                    <select on:change=on_mode_change prop:value=move || state.get().input_mode.select_value()>
                        <option value="csv-text">"CSV text"</option>
                        <option value="xlsx-base64">"XLSX base64"</option>
                    </select>
                </label>

                <label>
                    "Payload"
                    <textarea
                        on:input=on_payload_input
                        prop:value=move || state.get().payload
                        placeholder=move || state.get().input_mode.payload_hint()
                        rows="12"
                    />
                </label>

                <label>
                    "Field policy JSON"
                    <textarea
                        on:input=on_field_policy_input
                        prop:value=move || state.get().field_policy_json
                        rows="10"
                    />
                </label>

                <button on:click=on_submit type="button">"Submit"</button>
            </section>

            <Show when=move || state.get().error_banner.is_some()>
                <section aria-live="polite" class="error-banner">
                    <h2>"Error"</h2>
                    <p>{move || state.get().error_banner.unwrap_or_default()}</p>
                </section>
            </Show>

            <section>
                <h2>"Rewritten output"</h2>
                <pre>{move || state.get().result_output}</pre>
            </section>

            <section>
                <h2>"Summary"</h2>
                <p>{move || state.get().summary}</p>
            </section>

            <section>
                <h2>"Review queue"</h2>
                <pre>{move || state.get().review_queue}</pre>
            </section>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InputMode, TabularFlowState, DEFAULT_FIELD_POLICY_JSON, IDLE_REVIEW_QUEUE, IDLE_SUMMARY,
    };

    #[test]
    fn tabular_flow_state_defaults_to_csv_shell() {
        let state = TabularFlowState::default();

        assert_eq!(state.input_mode, InputMode::CsvText);
        assert!(state.payload.is_empty());
        assert_eq!(state.field_policy_json, DEFAULT_FIELD_POLICY_JSON);
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(state.error_banner.is_none());
    }

    #[test]
    fn submit_requires_payload_before_previewing_results() {
        let mut state = TabularFlowState::default();
        state.submit();

        assert_eq!(
            state.error_banner.as_deref(),
            Some("CSV text payload is required before submitting.")
        );
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
    }

    #[test]
    fn submit_generates_shell_preview_for_xlsx_base64_mode() {
        let mut state = TabularFlowState {
            input_mode: InputMode::XlsxBase64,
            payload: "UEsDBBQAAAAIA...".to_string(),
            ..TabularFlowState::default()
        };

        state.submit();

        assert!(state.error_banner.is_none());
        assert!(state.result_output.contains("mode: XLSX base64"));
        assert!(state
            .summary
            .contains("Shell submission captured for XLSX base64"));
        assert!(state.review_queue.contains("Review queue preview"));
    }

    #[test]
    fn submit_requires_non_blank_field_policy_before_previewing_results() {
        let mut state = TabularFlowState {
            payload: "patient_id,name\n1,Alice".to_string(),
            field_policy_json: "   \n\t".to_string(),
            ..TabularFlowState::default()
        };

        state.submit();

        assert_eq!(
            state.error_banner.as_deref(),
            Some("Field policy JSON is required before submitting.")
        );
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
    }

    #[test]
    fn clearing_inputs_resets_stale_generated_preview_state() {
        let mut state = TabularFlowState {
            payload: "patient_id,name\n1,Alice".to_string(),
            ..TabularFlowState::default()
        };
        state.submit();

        assert!(!state.result_output.is_empty());
        assert_ne!(state.summary, IDLE_SUMMARY);
        assert_ne!(state.review_queue, IDLE_REVIEW_QUEUE);

        state.clear_generated_state();

        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(state.error_banner.is_none());
    }
}
