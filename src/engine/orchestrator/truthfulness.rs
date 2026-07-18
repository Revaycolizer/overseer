//! Deterministic truthfulness guard.
//!
//! The assistant must never claim a side effect (a file created/written/saved, a
//! command executed) unless a real tool actually returned success during the run.
//! Local models frequently hallucinate "I created the file" as plain text without
//! ever emitting a tool call — especially when no capable tool exists. This guard
//! rewrites such unverified claims into an honest limitation/clarification.
//!
//! It is a pure function of `(response, executed_action)`, so it is fully
//! deterministic and unit-testable, and it never weakens kernel enforcement.

use crate::engine::orchestrator::core::response::StructuredResponse;

/// Explicit file/command completion phrases that assert a side effect happened.
const COMPLETION_PHRASES: &[&str] = &[
    "created the file",
    "created a file",
    "file has been created",
    "file was created",
    "successfully created the file",
    "saved the file",
    "wrote the file",
    "written to the file",
    "the file is saved",
    "file has been saved",
    "the file is now",
    "generated the file",
    "created the document",
    "saved to disk",
    "executed the command",
    "ran the command",
    "command executed successfully",
    "command completed successfully",
    "has been written to disk",
];

/// Does `text` assert a filesystem/command side effect completed?
///
/// Conservative on purpose: it targets file/command completions and avoids
/// creative uses of "wrote"/"created" (e.g. "I wrote a poem") by requiring a
/// filesystem/path indicator alongside the verb.
pub fn claims_side_effect(text: &str) -> bool {
    let t = text.to_lowercase();

    if COMPLETION_PHRASES.iter().any(|p| t.contains(p)) {
        return true;
    }

    let has_verb = t.contains("created")
        || t.contains("saved")
        || t.contains("wrote")
        || t.contains("written");
    let has_fs_indicator = t.contains("/tmp/")
        || t.contains("/home/")
        || t.contains(".txt")
        || t.contains(".md")
        || t.contains(".pdf")
        || t.contains(".docx")
        || t.contains(".csv")
        || t.contains(".xlsx")
        || t.contains("to disk")
        || t.contains(" file ")
        || t.ends_with(" file")
        || t.contains(" file.")
        || t.contains(" file,");

    has_verb && has_fs_indicator
}

/// The honest correction shown when a side-effect claim could not be verified.
pub fn honest_correction() -> String {
    "I can't confirm that action actually happened — no tool ran successfully for \
     this request, so nothing was created, written, or changed on disk. I may not \
     have a tool capable of this action. If you need a file written to a specific \
     path, that capability isn't available yet."
        .to_string()
}

/// Enforce truthfulness on a finished orchestration response.
///
/// `executed_action` MUST be true only when a real action tool returned success
/// during this run (derived from the execution trace). When it is false, any
/// message that claims a side effect is replaced with [`honest_correction`].
///
/// Returns the (possibly rewritten) response in the same shape it arrived in
/// (structured JSON when parseable, otherwise plain text).
pub fn enforce_truthful_response(response: &str, executed_action: bool) -> String {
    if executed_action {
        return response.to_string();
    }

    // Structured response path (the normal orchestrate output).
    if let Ok(mut sr) = serde_json::from_str::<StructuredResponse>(response) {
        // Only treat it as structured if it actually carries messages; otherwise
        // fall through to the plain-text path (some callers pass raw strings that
        // happen to deserialize into an all-default StructuredResponse).
        if !sr.messages.is_empty() {
            let mut changed = false;
            for part in sr.messages.iter_mut() {
                if claims_side_effect(&part.message) {
                    part.message = honest_correction();
                    changed = true;
                }
            }
            if changed {
                sr.status = "clarification".to_string();
                return serde_json::to_string(&sr).unwrap_or_else(|_| response.to_string());
            }
            return response.to_string();
        }
    }

    // Plain-text path.
    if claims_side_effect(response) {
        honest_correction()
    } else {
        response.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structured(message: &str) -> String {
        format!(
            r#"{{"status":"completed","messages":[{{"role":"assistant","message":{}}}]}}"#,
            serde_json::to_string(message).unwrap()
        )
    }

    #[test]
    fn detects_file_creation_claims() {
        assert!(claims_side_effect("I created the file successfully."));
        assert!(claims_side_effect("proof.txt has been created."));
        assert!(claims_side_effect("Saved to /tmp/overseer-kernel-test/proof.txt"));
        assert!(claims_side_effect("The file /home/x/a.md was written."));
        assert!(claims_side_effect("I ran the command and it completed."));
    }

    #[test]
    fn ignores_creative_and_non_side_effect_text() {
        assert!(!claims_side_effect("I wrote a short poem for you."));
        assert!(!claims_side_effect("Here is the plan you asked for."));
        assert!(!claims_side_effect("I can help you draft that message."));
    }

    #[test]
    fn no_success_claim_before_tool_success() {
        // A structured response claiming a file was created, with NO action run,
        // must be rewritten to an honest correction.
        let resp = structured("Done! I created the file /tmp/overseer-kernel-test/proof.txt for you.");
        let out = enforce_truthful_response(&resp, false);
        assert!(out.contains("can't confirm"), "must replace unverified claim: {out}");
        assert!(!out.contains("proof.txt"), "must not keep the false claim");
    }

    #[test]
    fn verified_action_is_left_untouched() {
        // When a tool actually succeeded, the claim is preserved verbatim.
        let resp = structured("I created the file /tmp/x/proof.txt.");
        let out = enforce_truthful_response(&resp, true);
        assert_eq!(out, resp);
    }

    #[test]
    fn unsupported_capability_returns_explicit_limitation() {
        // Plain-text hallucination with no capable tool -> explicit limitation.
        let out = enforce_truthful_response(
            "Sure — I have created the file at /tmp/overseer-kernel-test/proof.txt.",
            false,
        );
        assert!(out.contains("capability isn't available"), "explicit limitation: {out}");
    }

    #[test]
    fn honest_response_without_claims_is_unchanged() {
        let resp = structured("Here are three ideas for your essay.");
        let out = enforce_truthful_response(&resp, false);
        assert_eq!(out, resp);
    }
}
