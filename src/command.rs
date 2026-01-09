use anyhow::Result;

pub struct Command {
    components: Vec<String>,
}

/// Quotes a string so it can be safely passed as a single shell argument.
///
/// This helper chooses a quoting style and escapes only the characters required
/// for correct shell parsing, allowing intentional use of features like
/// variable expansion and command substitution.
///
/// Quoting strategy:
/// - If the string contains any whitespace or the characters `"`, `'`, `\`,
///   newline (`\n`), carriage return (`\r`), or tab (`\t`), it is wrapped in
///   quotes; otherwise it is returned unchanged.
/// - The function counts both single (`'`) and double (`"`) quotes and chooses
///   the quote style that minimizes escaping:
///   - If there are more double quotes than single quotes, the string is
///     wrapped in single quotes.
///   - Otherwise, the string is wrapped in double quotes.
///
/// Escaping rules:
/// - In single-quoted mode, the string is wrapped in `'...'`. Any literal
///   single quote inside is represented by closing the quote, adding an
///   escaped single quote, and reopening the quote (e.g. `abc'def` becomes
///   `'abc'\''def'`). Backslashes and all other characters are left as-is.
/// - In double-quoted mode, the string is wrapped in `"..."`. Within the
///   double quotes, backslashes (`\`) and double quotes (`"`) are prefixed
///   with a backslash; all other characters are left unchanged.
///
/// Special characters:
/// - Dollar signs (`$`) and backticks (`` ` ``) are intentionally *not*
///   escaped in either mode so that shell variable expansion and command
///   substitution can still occur when using the resulting string.
/// - Newlines (`\n`), tabs (`\t`), and other whitespace are preserved
///   literally inside the chosen quotes; their presence is what triggers
///   quoting in the first place.
fn quote_if_needed(s: &str) -> String {
    let needs_quoting = s
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\' | '\n' | '\r' | '\t'));

    if needs_quoting {
        // Choose quote style based on which quote char appears more
        let double_quotes = s.chars().filter(|&c| c == '"').count();
        let single_quotes = s.chars().filter(|&c| c == '\'').count();

        if double_quotes > single_quotes {
            // Use single quotes; to include a single quote in a single-quoted
            // shell string, close the quote, add an escaped quote, and reopen.
            // E.g., abc'def becomes 'abc'\''def'
            // Note: In single quotes, backslashes are literal (no escaping needed)
            let mut quoted = String::with_capacity(s.len() + 2);
            quoted.push('\'');
            for ch in s.chars() {
                match ch {
                    '\'' => quoted.push_str("'\\''"),
                    _ => quoted.push(ch),
                }
            }
            quoted.push('\'');
            quoted
        } else {
            // Use double quotes, escape backslashes and double quotes
            let mut escaped = String::with_capacity(s.len());
            for ch in s.chars() {
                match ch {
                    '\\' => escaped.push_str("\\\\"),
                    '"' => escaped.push_str("\\\""),
                    _ => escaped.push(ch),
                }
            }
            format!("\"{}\"", escaped)
        }
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

    /// Convert command to a shell-safe string with proper quoting
    pub fn to_shell_string(&self) -> String {
        self.components
            .iter()
            .map(|c| quote_if_needed(c))
            .collect::<Vec<_>>()
            .join(" ")
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
        // Simple strings without spaces - no quoting needed
        assert_eq!(quote_if_needed("kubectl"), "kubectl");
        assert_eq!(quote_if_needed("--name"), "--name");
        assert_eq!(quote_if_needed("myapp"), "myapp");

        // Empty string - no quoting needed
        assert_eq!(quote_if_needed(""), "");

        // String with spaces - use double quotes (default)
        assert_eq!(quote_if_needed("hello world"), "\"hello world\"");

        // String with double quotes (2 > 0 single) - use single quotes
        // Single quotes preserve the double quotes literally
        assert_eq!(quote_if_needed("say \"hello\""), "'say \"hello\"'");

        // String with more double quotes than single - use single quotes
        assert_eq!(
            quote_if_needed("say \"hello\" and \"world\""),
            "'say \"hello\" and \"world\"'"
        );

        // String with single quote - use double quotes
        assert_eq!(quote_if_needed("it's fine"), "\"it's fine\"");

        // String with more single quotes than double - use double quotes
        assert_eq!(quote_if_needed("it's Bob's day"), "\"it's Bob's day\"");

        // More double quotes (2) than single (1) - use single quotes with '\'' escape
        assert_eq!(quote_if_needed("it's \"ok\""), "'it'\\''s \"ok\"'");

        // Equal single and double quotes (1 each) - prefer double quotes
        assert_eq!(quote_if_needed("it's x\""), "\"it's x\\\"\"");

        // String with only backslashes - needs quoting and escaping
        assert_eq!(quote_if_needed("path\\to\\file"), "\"path\\\\to\\\\file\"");

        // String with dollar sign - quoted due to space, but $ not escaped (allow variable expansion)
        assert_eq!(quote_if_needed("test $HOME"), "\"test $HOME\"");

        // String with backtick - quoted due to space, but ` not escaped (allow command substitution)
        assert_eq!(quote_if_needed("run `cmd`"), "\"run `cmd`\"");

        // Dollar sign alone - no quoting needed
        assert_eq!(quote_if_needed("$HOME"), "$HOME");

        // Backtick alone - no quoting needed
        assert_eq!(quote_if_needed("`cmd`"), "`cmd`");

        // String with newline - needs quoting (preserved in quotes)
        assert_eq!(quote_if_needed("line1\nline2"), "\"line1\nline2\"");

        // String with tab - needs quoting (preserved in quotes)
        assert_eq!(quote_if_needed("col1\tcol2"), "\"col1\tcol2\"");

        // Single quote inside single-quoted string uses '\'' technique
        // When we have more double quotes than single quotes, we use single quotes
        // and escape single quotes with '\''
        assert_eq!(
            quote_if_needed("say \"hello\" it's \"great\""),
            "'say \"hello\" it'\\''s \"great\"'"
        );

        // String with newline AND more double quotes - uses single quotes
        // (single quotes can contain literal newlines)
        assert_eq!(
            quote_if_needed("line1\nline2 \"quoted\""),
            "'line1\nline2 \"quoted\"'"
        );

        // String with tab AND more double quotes - uses single quotes
        assert_eq!(
            quote_if_needed("col1\tcol2 \"data\""),
            "'col1\tcol2 \"data\"'"
        );
    }

    #[test]
    fn test_to_shell_string() {
        // Simple command roundtrip
        let cmd: Command = "kubectl get pods -n default".try_into().unwrap();
        assert_eq!(cmd.to_shell_string(), "kubectl get pods -n default");

        // Command with quoted value containing spaces
        let cmd: Command = "echo \"hello world\"".try_into().unwrap();
        assert_eq!(cmd.to_shell_string(), "echo \"hello world\"");

        // Command with --flag=value syntax (now kept as single token)
        let cmd: Command = "docker run --name=myapp image".try_into().unwrap();
        assert_eq!(cmd.to_shell_string(), "docker run --name=myapp image");
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
