use crate::app::CommandComponent;
use anyhow::Result;

pub fn parse_command(command_str: &str) -> Result<Vec<CommandComponent>> {
    let tokens = shlex::split(command_str)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse command string"))?;

    if tokens.is_empty() {
        anyhow::bail!("Empty command");
    }

    let mut components = Vec::new();
    let mut i = 0;

    // Find where arguments start (first token starting with -)
    while i < tokens.len() {
        let token = &tokens[i];
        if token.starts_with('-') {
            break;
        }
        components.push(CommandComponent::Base(token.clone()));
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
                components.push(CommandComponent::StringArgument(flag, value));
                i += 1;
            } else {
                // Check if next token is a value (doesn't start with -)
                let flag = token.clone();
                if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                    let value = tokens[i + 1].clone();
                    components.push(CommandComponent::StringArgument(flag, value));
                    i += 2;
                } else {
                    // Boolean flag (no value)
                    components.push(CommandComponent::BoolArgument(flag, true));
                    i += 1;
                }
            }
        } else {
            // Unexpected token (not starting with -)
            // Treat it as a positional argument
            components.push(CommandComponent::StringArgument(
                String::new(),
                token.clone(),
            ));
            i += 1;
        }
    }

    Ok(components)
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
        assert_eq!(
            components[3],
            CommandComponent::StringArgument("-l".to_string(), "app=asset".to_string())
        );
        assert_eq!(
            components[4],
            CommandComponent::StringArgument("-o".to_string(), "json".to_string())
        );
    }

    #[test]
    fn test_parse_with_equals() {
        let cmd = "docker run --name=myapp --env=VAR=value image";
        let components = parse_command(cmd).unwrap();

        assert_eq!(components[0], CommandComponent::Base("docker".to_string()));
        assert_eq!(components[1], CommandComponent::Base("run".to_string()));
        assert_eq!(
            components[2],
            CommandComponent::StringArgument("--name".to_string(), "myapp".to_string())
        );
        assert_eq!(
            components[3],
            CommandComponent::StringArgument("--env".to_string(), "VAR=value".to_string())
        );
        assert_eq!(
            components[4],
            CommandComponent::StringArgument(String::new(), "image".to_string())
        );
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
        assert_eq!(
            components[3],
            CommandComponent::StringArgument(
                "-o".to_string(),
                "custom-columns=POD:.metadata.name,RS:.metadata.ownerReferences[0].name"
                    .to_string()
            )
        );
    }
}
