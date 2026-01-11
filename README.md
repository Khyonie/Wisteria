# Wisteria🌻
Wisteria is a Java project manager and builder for Linux CLI-only environments, such as SSH sessions.

## Usage
### 🌟 Create a new project
`-$ wisteria3 create <your project name>` 

Optionally, add the `--minimal` flag to create the project using a minimal `project.toml`.

### 🔃 Switch the project configuration
`-$ wisteria3 switch <configuration>`

Switches the project configuration to match another configuration defined in `project.toml`.

### ✨ Apply project settings to the workspace
`-$ wisteria3 refresh`

The current `project.toml` settings will apply to the project workspace by generating files based on the natures in `project.natures`. Useful to apply changes to the workspace without switching the current project configuration.
- "eclipse" nature generates `.project` and `.classpath` files.
- "maven" nature generates `pom.xml`.

### ⬇️ Update dependencies
`-$ wisteria3 update [dependency | all]`

Downloads a specific Maven or Github dependency as defined in the `[dependencies]` section of `project.toml` and reconfigures the classpath to use the new file. Unless otherwise defined, the version selected will be the latest stable release.

If "all" is specified as the dependency, all applicable dependencies will be updated.
## Project.toml
Projects are defined inside of a `project.toml` file at the root of the project hierarchy.

For example:
```toml
[project] 
name = "WisteriaProject"
version = "0.1.0"
description = "An example of a project file."
natures = [ "eclipse", "maven" ] 

[dependencies]

[configuration.main]
sources = [ "src/" ]
dependencies = [  ] 
targets = [ "targets/{configuration}/{name}-{configuration}-{version}.jar
```
