# Filesystem-Based Skill Loading

This example walkthrough demonstrates how to load domain-specific knowledge and instructions from directories containing `SKILL.md` files with YAML frontmatter, following the [official Agent Skills specification](https://agentskills.io/home).

An **Agent Skill** is a standardized format to give AI agents new capabilities and expertise. It typically consists of a directory containing a `SKILL.md` file with instructions and metadata, and optionally scripts, references, and assets.

## Loading Skills in Rust

You configure an agent to load skills from the filesystem by specifying paths in the `skills_paths` vector inside `AgentConfig`.

> [!IMPORTANT]
> The `skills_paths` property accepts a list of paths. Each path can be:
> - A directory that *contains* skill folders (each containing a `SKILL.md` file). The agent will discover all skills under that directory.
> - A direct path to a specific skill folder (containing a `SKILL.md` file) to load just that single skill.

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Set path to directory containing skill folders
    let skills_directory = "/path/to/skills".to_string();
    config.skills_paths = vec![skills_directory];

    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("List your available skills and explain what they do.").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```
