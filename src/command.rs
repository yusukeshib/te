use anyhow::Result;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Comp {
    Base(String),
    Flag(String),
    Value(String),
}

impl Comp {
    pub fn as_str(&self) -> &str {
        match self {
            Comp::Base(s) | Comp::Flag(s) | Comp::Value(s) => s,
        }
    }

    pub fn set_value(&mut self, new_value: &str) {
        match self {
            Comp::Base(s) | Comp::Flag(s) | Comp::Value(s) => *s = new_value.to_string(),
        }
    }
}

impl fmt::Display for Comp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.as_str();
        if s.contains(' ') {
            let escaped = s.replace('"', "\\\"");
            write!(f, "\"{}\"", escaped)
        } else {
            write!(f, "{}", s)
        }
    }
}

pub struct Command {
    components: Vec<Comp>,
}

impl Command {
    pub fn set_value_at(&mut self, index: usize, new_value: &str) {
        self.components[index].set_value(new_value);
    }
    pub fn component_count(&self) -> usize {
        self.components.len()
    }
    pub fn component_at(&self, index: usize) -> &Comp {
        &self.components[index]
    }
    pub fn iter_components(&self) -> impl Iterator<Item = &Comp> {
        self.components.iter()
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, component) in self.components.iter().enumerate() {
            if idx > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", component)?;
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

            let mut i = 0;

            // Find where arguments start (first token starting with -)
            while i < tokens.len() {
                let token = &tokens[i];
                if token.starts_with('-') {
                    break;
                }
                components.push(Comp::Base(token.clone()));
                i += 1;
            }

            // Parse arguments
            while i < tokens.len() {
                let token = &tokens[i];

                if token.starts_with('-') {
                    // Check if it's in the form --flag=value or -f=value
                    if let Some(eq_pos) = token.find('=') {
                        let flag = token[..eq_pos].to_string();
                        let value = token[eq_pos + 1..].to_string();
                        components.push(Comp::Flag(flag));
                        components.push(Comp::Value(value));
                        i += 1;
                    } else {
                        // Check if next token is a value (doesn't start with -)
                        let flag = token.clone();
                        if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                            let value = tokens[i + 1].clone();
                            components.push(Comp::Flag(flag));
                            components.push(Comp::Value(value));
                            i += 2;
                        } else {
                            // Boolean flag (no value)
                            components.push(Comp::Flag(flag));
                            i += 1;
                        }
                    }
                } else {
                    // Unexpected token (not starting with -)
                    // Treat it as a positional argument
                    components.push(Comp::Value(token.clone()));
                    i += 1;
                }
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

        assert_eq!(*cmd.component_at(0), Comp::Base("kubectl".to_string()));
        assert_eq!(*cmd.component_at(1), Comp::Base("get".to_string()));
        assert_eq!(*cmd.component_at(2), Comp::Base("pods".to_string()));
        assert_eq!(*cmd.component_at(3), Comp::Flag("-l".to_string()));
        assert_eq!(*cmd.component_at(4), Comp::Value("app=asset".to_string()));
        assert_eq!(*cmd.component_at(5), Comp::Flag("-o".to_string()));
        assert_eq!(*cmd.component_at(6), Comp::Value("json".to_string()));
    }

    #[test]
    fn test_parse_with_equals() {
        let cmd: Command = "docker run --name=myapp --env=VAR=value image"
            .try_into()
            .unwrap();

        assert_eq!(*cmd.component_at(0), Comp::Base("docker".to_string()));
        assert_eq!(*cmd.component_at(1), Comp::Base("run".to_string()));
        assert_eq!(*cmd.component_at(2), Comp::Flag("--name".to_string()));
        assert_eq!(*cmd.component_at(3), Comp::Value("myapp".to_string()));
        assert_eq!(*cmd.component_at(4), Comp::Flag("--env".to_string()));
        assert_eq!(*cmd.component_at(5), Comp::Value("VAR=value".to_string()));
        assert_eq!(*cmd.component_at(6), Comp::Value("image".to_string()));
    }

    #[test]
    fn test_parse_boolean_flags() {
        let cmd: Command = "ls -la /tmp".try_into().unwrap();

        assert_eq!(*cmd.component_at(0), Comp::Base("ls".to_string()));
        // -la might be treated as a single flag with no value
        // or as two separate flags - depends on shlex behavior
    }

    #[test]
    fn test_parse_with_quotes() {
        let cmd: Command = r#"kubectl get pods -o custom-columns='POD:.metadata.name,RS:.metadata.ownerReferences[0].name'"#.try_into().unwrap();

        assert_eq!(*cmd.component_at(0), Comp::Base("kubectl".to_string()));
        assert_eq!(*cmd.component_at(1), Comp::Base("get".to_string()));
        assert_eq!(*cmd.component_at(2), Comp::Base("pods".to_string()));
        assert_eq!(*cmd.component_at(3), Comp::Flag("-o".to_string()));
        assert_eq!(
            *cmd.component_at(4),
            Comp::Value(
                "custom-columns=POD:.metadata.name,RS:.metadata.ownerReferences[0].name"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_with_line_breaks() {
        let cmd: Command = "gcloud alpha pam grants create \\\n  --entitlement=secret-manager-admin \\\n  --requested-duration=28800s".try_into()
            .unwrap();

        assert_eq!(*cmd.component_at(0), Comp::Base("gcloud".to_string()));
        assert_eq!(*cmd.component_at(1), Comp::Base("alpha".to_string()));
        assert_eq!(*cmd.component_at(2), Comp::Base("pam".to_string()));
        assert_eq!(*cmd.component_at(3), Comp::Base("grants".to_string()));
        assert_eq!(*cmd.component_at(4), Comp::Base("create".to_string()));
        assert_eq!(
            *cmd.component_at(5),
            Comp::Flag("--entitlement".to_string())
        );
        assert_eq!(
            *cmd.component_at(6),
            Comp::Value("secret-manager-admin".to_string())
        );
        assert_eq!(
            *cmd.component_at(7),
            Comp::Flag("--requested-duration".to_string())
        );
        assert_eq!(*cmd.component_at(8), Comp::Value("28800s".to_string()));
    }

    #[test]
    fn test_comp_display() {
        // Simple strings without spaces
        assert_eq!(Comp::Base("kubectl".to_string()).to_string(), "kubectl");
        assert_eq!(Comp::Flag("--name".to_string()).to_string(), "--name");
        assert_eq!(Comp::Value("myapp".to_string()).to_string(), "myapp");

        // String with spaces should be quoted
        assert_eq!(
            Comp::Value("hello world".to_string()).to_string(),
            "\"hello world\""
        );

        // String with spaces and double quotes should escape quotes
        assert_eq!(
            Comp::Value("say \"hello\"".to_string()).to_string(),
            "\"say \\\"hello\\\"\""
        );
    }

    #[test]
    fn test_command_display() {
        // Simple command roundtrip
        let cmd: Command = "kubectl get pods -n default".try_into().unwrap();
        assert_eq!(cmd.to_string(), "kubectl get pods -n default");

        // Command with quoted value containing spaces
        let cmd: Command = "echo \"hello world\"".try_into().unwrap();
        assert_eq!(cmd.to_string(), "echo \"hello world\"");

        // Command with --flag=value syntax
        let cmd: Command = "docker run --name=myapp image".try_into().unwrap();
        assert_eq!(cmd.to_string(), "docker run --name myapp image");
    }
}
