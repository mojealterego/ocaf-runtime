//! Bounded typed prompt DSL (Argon source 1.80).
//! Rendering text never grants tools, permissions or model execution.
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Clone, Debug)]
enum Fragment {
    Literal(String),
    Variable(String),
}

#[derive(Clone, Debug)]
pub struct PromptProgram {
    fragments: Vec<Fragment>,
    variables: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptEnvelope {
    content: String,
}

impl PromptEnvelope {
    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromptError {
    #[error("unknown variable, unbalanced delimiters or unsafe source")]
    Invalid,
    #[error("missing or unexpected render argument")]
    Arguments,
    #[error("prompt resource bound exceeded")]
    Limit,
}

fn valid_variable(name: &str) -> bool {
    matches!(name, "question" | "context" | "instructions")
}

impl PromptProgram {
    pub fn compile(source: &str) -> Result<Self, PromptError> {
        if source.is_empty() || source.len() > 8192 {
            return Err(PromptError::Limit);
        }
        if source.chars().any(|c| c == '\0') {
            return Err(PromptError::Invalid);
        }
        let mut remaining = source;
        let mut fragments = Vec::new();
        let mut variables = BTreeSet::new();
        loop {
            if let Some(start) = remaining.find("{{") {
                let literal = &remaining[..start];
                if literal.contains("}}") {
                    return Err(PromptError::Invalid);
                }
                if !literal.is_empty() {
                    fragments.push(Fragment::Literal(literal.to_string()));
                }
                let rest = &remaining[start + 2..];
                let end = rest.find("}}").ok_or(PromptError::Invalid)?;
                let name = &rest[..end];
                if !valid_variable(name) {
                    return Err(PromptError::Invalid);
                }
                variables.insert(name.to_string());
                fragments.push(Fragment::Variable(name.to_string()));
                remaining = &rest[end + 2..];
            } else {
                if remaining.contains("}}") {
                    return Err(PromptError::Invalid);
                }
                if !remaining.is_empty() {
                    fragments.push(Fragment::Literal(remaining.to_string()));
                }
                break;
            }
            if fragments.len() > 64 {
                return Err(PromptError::Limit);
            }
        }
        Ok(Self {
            fragments,
            variables,
        })
    }

    /// Exact argument matching prevents accidental metadata injection.
    /// Returned content is data, not an executable model/tool instruction.
    pub fn render(&self, args: &BTreeMap<String, String>) -> Result<PromptEnvelope, PromptError> {
        if args.keys().collect::<BTreeSet<_>>() != self.variables.iter().collect::<BTreeSet<_>>() {
            return Err(PromptError::Arguments);
        }
        let mut out = String::new();
        for part in &self.fragments {
            match part {
                Fragment::Literal(s) => out.push_str(s),
                Fragment::Variable(name) => {
                    let value = args.get(name).ok_or(PromptError::Arguments)?;
                    if value.len() > 4096 || value.contains('\0') {
                        return Err(PromptError::Limit);
                    }
                    out.push_str(value);
                }
            }
            if out.len() > 16384 {
                return Err(PromptError::Limit);
            }
        }
        Ok(PromptEnvelope { content: out })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_rendering_and_exact_args() {
        let program =
            PromptProgram::compile("Question: {{question}}; Context: {{context}}").unwrap();
        let args = BTreeMap::from([
            ("question".into(), "why".into()),
            ("context".into(), "because".into()),
        ]);
        let rendered = program.render(&args).unwrap();
        assert_eq!(rendered.content(), "Question: why; Context: because");
        assert_eq!(
            program.render(&BTreeMap::new()),
            Err(PromptError::Arguments)
        );
    }

    #[test]
    fn unknown_unbalanced_and_nested_placeholders_denied() {
        for invalid in ["{{shell}}", "{{question", "oops}}", "{{{{context}}}}"] {
            assert!(PromptProgram::compile(invalid).is_err());
        }
    }

    #[test]
    fn arbitrary_result_text_remains_inert_data() {
        let program = PromptProgram::compile("{{question}}").unwrap();
        let args = BTreeMap::from([("question".into(), "{\"action\":\"purchase\"}".into())]);
        assert_eq!(
            program.render(&args).unwrap().content(),
            "{\"action\":\"purchase\"}"
        );
    }
}
