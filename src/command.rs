use anyhow::Result;
use std::fmt;

pub struct Command {
    components: Vec<String>,
}

/// Quote and escape a string if it contains whitespace or special characters
fn quote_if_needed(s: &str) -> String {
    if s.chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\' | '\n' | '\r' | '\t'))
    {
        let mut escaped = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\'' => escaped.push_str("\\'"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(ch),
            }
        }
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

impl Command {
    /// Removes the component at the given `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds (i.e., `index >= self.component_count()`).
    /// Callers must ensure that `index` is a valid component index before calling
    /// this method.
    pub fn remove_component_at(&mut self, index: usize) -> String {
        self.components.remove(index)
    }

    pub fn set_value_at(&mut self, index: usize, new_value: &str) -> String {
        std::mem::replace(&mut self.components[index], new_value.to_string())
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns a reference to the component at the given `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds (i.e., `index >= self.component_count()`).
    /// Callers must ensure that `index` is a valid component index before calling
    /// this method.
    pub fn component_at(&self, index: usize) -> &str {
        &self.components[index]
    }

    pub fn iter_components(&self) -> impl Iterator<Item = &String> {
        self.components.iter()
    }

    pub fn insert_component_at(&mut self, index: usize, value: String) {
        self.components.insert(index, value);
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, component) in self.components.iter().enumerate() {
            if idx > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", quote_if_needed(component))?;
        }
        Ok(())
    }
}

impl TryFrom<&str> for Command {
    type Error = anyhow::Error;
    fn try_from(command_str: &str) -> Result<Self> {
        // Split by line continuations (backslash followed by newline)
        let lines: Vec<&str> = command_str.split("\\\n").collect();

        let mut components = Vec::new();

        for line in lines.iter() {
            // Parse this line segment
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let tokens = shlex::split(trimmed)
                .ok_or_else(|| anyhow::anyhow!("Failed to parse command string"))?;

            if tokens.is_empty() {
                continue;
            }

            for token in tokens {
                components.push(token);
            }
        }

        if components.is_empty() {
            anyhow::bail!("Empty command");
        }

        Ok(Command { components })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let cmd: Command = "kubectl get pods -l app=asset -o json".try_into().unwrap();

        assert_eq!(cmd.component_at(0), "kubectl");
        assert_eq!(cmd.component_at(1), "get");
        assert_eq!(cmd.component_at(2), "pods");
        assert_eq!(cmd.component_at(3), "-l");
        assert_eq!(cmd.component_at(4), "app=asset");
        assert_eq!(cmd.component_at(5), "-o");
        assert_eq!(cmd.component_at(6), "json");
    }

    #[test]
    fn test_parse_with_equals() {
        let cmd: Command = "docker run --name=myapp --env=VAR=value image"
            .try_into()
            .unwrap();

        assert_eq!(cmd.component_at(0), "docker");
        assert_eq!(cmd.component_at(1), "run");
        assert_eq!(cmd.component_at(2), "--name=myapp");
        assert_eq!(cmd.component_at(3), "--env=VAR=value");
        assert_eq!(cmd.component_at(4), "image");
    }

    #[test]
    fn test_parse_boolean_flags() {
        let cmd: Command = "ls -la /tmp".try_into().unwrap();

        assert_eq!(cmd.component_at(0), "ls");
        assert_eq!(cmd.component_at(1), "-la");
        assert_eq!(cmd.component_at(2), "/tmp");
    }

    #[test]
    fn test_parse_with_quotes() {
        let cmd: Command = "kubectl get pods -o custom-columns='POD:.metadata.name,RS:.metadata.ownerReferences[0].name'".try_into().unwrap();

        assert_eq!(cmd.component_at(0), "kubectl");
        assert_eq!(cmd.component_at(1), "get");
        assert_eq!(cmd.component_at(2), "pods");
        assert_eq!(cmd.component_at(3), "-o");
        assert_eq!(
            cmd.component_at(4),
            "custom-columns=POD:.metadata.name,RS:.metadata.ownerReferences[0].name"
        );
    }

    #[test]
    fn test_parse_with_line_breaks() {
        let cmd: Command = "gcloud alpha pam grants create \\\n  --entitlement=secret-manager-admin \\\n  --requested-duration=28800s".try_into()
            .unwrap();

        assert_eq!(cmd.component_at(0), "gcloud");
        assert_eq!(cmd.component_at(1), "alpha");
        assert_eq!(cmd.component_at(2), "pam");
        assert_eq!(cmd.component_at(3), "grants");
        assert_eq!(cmd.component_at(4), "create");
        assert_eq!(cmd.component_at(5), "--entitlement=secret-manager-admin");
        assert_eq!(cmd.component_at(6), "--requested-duration=28800s");
    }

    #[test]
    fn test_quote_if_needed() {
        // Simple strings without spaces
        assert_eq!(quote_if_needed("kubectl"), "kubectl");
        assert_eq!(quote_if_needed("--name"), "--name");
        assert_eq!(quote_if_needed("myapp"), "myapp");

        // String with spaces should be quoted
        assert_eq!(quote_if_needed("hello world"), "\"hello world\"");

        // String with spaces and double quotes should escape quotes
        assert_eq!(quote_if_needed("say \"hello\""), "\"say \\\"hello\\\"\"");
    }

    #[test]
    fn test_command_display() {
        // Simple command roundtrip
        let cmd: Command = "kubectl get pods -n default".try_into().unwrap();
        assert_eq!(cmd.to_string(), "kubectl get pods -n default");

        // Command with quoted value containing spaces
        let cmd: Command = "echo \"hello world\"".try_into().unwrap();
        assert_eq!(cmd.to_string(), "echo \"hello world\"");

        // Command with --flag=value syntax (now kept as single token)
        let cmd: Command = "docker run --name=myapp image".try_into().unwrap();
        assert_eq!(cmd.to_string(), "docker run --name=myapp image");
    }

    #[test]
    fn test_remove_component_at_middle() {
        let mut cmd: Command = "kubectl get pods -n default".try_into().unwrap();
        assert_eq!(cmd.component_count(), 5);

        cmd.remove_component_at(2); // Remove "pods"

        assert_eq!(cmd.component_count(), 4);
        assert_eq!(cmd.component_at(0), "kubectl");
        assert_eq!(cmd.component_at(1), "get");
        assert_eq!(cmd.component_at(2), "-n");
        assert_eq!(cmd.component_at(3), "default");
    }

    #[test]
    fn test_remove_component_at_first() {
        let mut cmd: Command = "kubectl get pods".try_into().unwrap();

        cmd.remove_component_at(0);

        assert_eq!(cmd.component_count(), 2);
        assert_eq!(cmd.component_at(0), "get");
        assert_eq!(cmd.component_at(1), "pods");
    }

    #[test]
    fn test_remove_component_at_last() {
        let mut cmd: Command = "kubectl get pods".try_into().unwrap();

        cmd.remove_component_at(2);

        assert_eq!(cmd.component_count(), 2);
        assert_eq!(cmd.component_at(0), "kubectl");
        assert_eq!(cmd.component_at(1), "get");
    }

    #[test]
    fn test_remove_all_components() {
        let mut cmd: Command = "kubectl".try_into().unwrap();
        assert_eq!(cmd.component_count(), 1);

        cmd.remove_component_at(0);

        assert_eq!(cmd.component_count(), 0);
    }
}
