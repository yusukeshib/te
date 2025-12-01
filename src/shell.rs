pub fn generate_init_script(shell: &str) -> Option<String> {
    match shell {
        "zsh" => Some(generate_zsh_script()),
        "bash" => Some(generate_bash_script()),
        "fish" => Some(generate_fish_script()),
        _ => None,
    }
}

fn generate_zsh_script() -> String {
    r#"# te shell integration for zsh

# Function to run te and execute the resulting command
te-run() {
    local result
    result=$(te "$@")
    if [ $? -eq 0 ] && [ -n "$result" ]; then
        eval "$result"
    fi
}

# Widget to invoke te with current buffer content
te-widget() {
    if [ -n "$BUFFER" ]; then
        local result
        result=$(te $BUFFER)
        if [ $? -eq 0 ] && [ -n "$result" ]; then
            BUFFER="$result"
            zle end-of-line
        fi
    fi
}

# Register the widget
zle -N te-widget

# Bind Ctrl+T to the widget (you can customize this)
bindkey '^T' te-widget
"#.to_string()
}

fn generate_bash_script() -> String {
    r#"# te shell integration for bash

# Function to run te and execute the resulting command
te-run() {
    local result
    result=$(te "$@")
    if [ $? -eq 0 ] && [ -n "$result" ]; then
        eval "$result"
    fi
}
"#.to_string()
}

fn generate_fish_script() -> String {
    r#"# te shell integration for fish

# Function to run te and execute the resulting command
function te-run
    set -l result (te $argv)
    if test $status -eq 0 -a -n "$result"
        eval $result
    end
end
"#.to_string()
}
