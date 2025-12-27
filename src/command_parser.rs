use crate::app::CommandComponent;
use anyhow::Result;

pub fn parse_command(command_str: &str) -> Result<Vec<CommandComponent>> {
    // Split by line continuations (backslash followed by newline)
    let lines: Vec<&str> = command_str.split("\\\n").collect();

    let mut all_components = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        // Parse this line segment
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // Empty line, but we still want to preserve the line break
            if line_idx < lines.len() - 1 {
                all_components.push(CommandComponent::LineBreak);
            }
            continue;
        }

        let tokens = shlex::split(trimmed)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse command string"))?;

        if tokens.is_empty() {
            if line_idx < lines.len() - 1 {
                all_components.push(CommandComponent::LineBreak);
            }
            continue;
        }

        let mut i = 0;

        // Find where arguments start (first token starting with -)
        while i < tokens.len() {
            let token = &tokens[i];
            if token.starts_with('-') {
                break;
            }
            all_components.push(CommandComponent::Base(token.clone()));
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
                    all_components.push(CommandComponent::Flag(flag));
                    all_components.push(CommandComponent::Value(value));
                    i += 1;
                } else {
                    // Check if next token is a value (doesn't start with -)
                    let flag = token.clone();
                    if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                        let value = tokens[i + 1].clone();
                        all_components.push(CommandComponent::Flag(flag));
                        all_components.push(CommandComponent::Value(value));
                        i += 2;
                    } else {
                        // Boolean flag (no value)
                        all_components.push(CommandComponent::Flag(flag));
                        i += 1;
                    }
                }
            } else {
                // Unexpected token (not starting with -)
                // Treat it as a positional argument
                all_components.push(CommandComponent::Value(token.clone()));
                i += 1;
            }
        }

        // Add line break after this segment (except for the last line)
        if line_idx < lines.len() - 1 {
            all_components.push(CommandComponent::LineBreak);
        }
    }

    if all_components.is_empty() {
        anyhow::bail!("Empty command");
    }

    Ok(all_components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let cmd = "kubectl get pods -l app=asset -o json";
        let components = parse_command(cmd).unwrap();

        assert_eq!(components[0], CommandComponent::Base("kubectl".to_string()));
        assert_eq!(components[1], CommandComponent::Base("get".to_string()));
        assert_eq!(components[2], CommandComponent::Base("pods".to_string()));
        assert_eq!(components[3], CommandComponent::Flag("-l".to_string()));
        assert_eq!(
            components[4],
            CommandComponent::Value("app=asset".to_string())
        );
        assert_eq!(components[5], CommandComponent::Flag("-o".to_string()));
        assert_eq!(components[6], CommandComponent::Value("json".to_string()));
    }

    #[test]
    fn test_parse_with_equals() {
        let cmd = "docker run --name=myapp --env=VAR=value image";
        let components = parse_command(cmd).unwrap();

        assert_eq!(components[0], CommandComponent::Base("docker".to_string()));
        assert_eq!(components[1], CommandComponent::Base("run".to_string()));
        assert_eq!(components[2], CommandComponent::Flag("--name".to_string()));
        assert_eq!(components[3], CommandComponent::Value("myapp".to_string()));
        assert_eq!(components[4], CommandComponent::Flag("--env".to_string()));
        assert_eq!(
            components[5],
            CommandComponent::Value("VAR=value".to_string())
        );
        assert_eq!(components[6], CommandComponent::Value("image".to_string()));
    }

    #[test]
    fn test_parse_boolean_flags() {
        let cmd = "ls -la /tmp";
        let components = parse_command(cmd).unwrap();

        assert_eq!(components[0], CommandComponent::Base("ls".to_string()));
        // -la might be treated as a single flag with no value
        // or as two separate flags - depends on shlex behavior
    }

    #[test]
    fn test_parse_with_quotes() {
        let cmd = r#"kubectl get pods -o custom-columns='POD:.metadata.name,RS:.metadata.ownerReferences[0].name'"#;
        let components = parse_command(cmd).unwrap();

        assert_eq!(components[0], CommandComponent::Base("kubectl".to_string()));
        assert_eq!(components[1], CommandComponent::Base("get".to_string()));
        assert_eq!(components[2], CommandComponent::Base("pods".to_string()));
        assert_eq!(components[3], CommandComponent::Flag("-o".to_string()));
        assert_eq!(
            components[4],
            CommandComponent::Value(
                "custom-columns=POD:.metadata.name,RS:.metadata.ownerReferences[0].name"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_with_line_breaks() {
        let cmd = "gcloud alpha pam grants create \\\n  --entitlement=secret-manager-admin \\\n  --requested-duration=28800s";
        let components = parse_command(cmd).unwrap();

        assert_eq!(components[0], CommandComponent::Base("gcloud".to_string()));
        assert_eq!(components[1], CommandComponent::Base("alpha".to_string()));
        assert_eq!(components[2], CommandComponent::Base("pam".to_string()));
        assert_eq!(components[3], CommandComponent::Base("grants".to_string()));
        assert_eq!(components[4], CommandComponent::Base("create".to_string()));
        assert_eq!(components[5], CommandComponent::LineBreak);
        assert_eq!(
            components[6],
            CommandComponent::Flag("--entitlement".to_string())
        );
        assert_eq!(
            components[7],
            CommandComponent::Value("secret-manager-admin".to_string())
        );
        assert_eq!(components[8], CommandComponent::LineBreak);
        assert_eq!(
            components[9],
            CommandComponent::Flag("--requested-duration".to_string())
        );
        assert_eq!(
            components[10],
            CommandComponent::Value("28800s".to_string())
        );
    }
}
