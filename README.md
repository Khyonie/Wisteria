# Wisteria
Wisteria is a Java project manager and builder buildt in Rust for Linux CLI-only environments, such as SSH 
sessions. Inspired by Rust-lang's Cargo project manager.

*Example project:*
```toml
[project]
name = "MyJavaProject"
description = "My Java project."
version = "0.1.0"

# Declare dependencies and how to find them
[dependencies.maven]
gson = { group_id = "com.google.code.gson", artifact_id = "gson" }

[dependencies.github]
example = { repository = "Example/Example4j" }

# Configurations define the working parts of the project environment
[configuration.main]
sources = [ "src/main/" ]
targets = [ "build/{configuration}/{project_name}-{version}.jar" ]
dependencies = [
  { name = "gson", scope = "compile" },
  { name = "example", scope = "compile", package = "shade" },
]
```

# Building
Build with Cargo:
```shell
cargo build --release
```

# Usage
```shell
wisteria <new | update | switch | refresh | clean | migrate | info>
```
By default, Wisteria will detect a `project.toml` in the current directory. Specific project files can be specified
with the `--project` flag.

### Options
`--project`: Use a specific project file.

`--minimal`: Use a minimal project.toml template when creating a new project.

`--norefresh`: Skips automatically refreshing the project configuration when switching.

## Implicit tasks
wisteria reads the current project configuration, and derives tasks based on what has been defined.

### Build
```shell
wisteria build
```
Builds the current project configuration and writes it to all configured targets.

### Run
```shell
wisteria run -- <args...>
```
Builds and runs the current project configuration, passing any given args after `--` as program arguments.

### Javadoc
```shell
wisteria javadoc
```
Generates javadoc files into the configured directory.

# Project File
The project file is the beating heart of your Java project, defining your dependencies, inputs, outputs, and other
useful settings.

### Project Header
Top-level information about your project. Settings from here can be used in configurations.
|Key|Required|Description|Configuration Variable|
|---|--------|-----------|----------------------|
|name|true|Your project name.|`{project_name}`|
|description|true|Your project's description||
|version|true|Your project's version. [Semantic versioning](https://semver.org) is encouraged.|`{version}`|
|natures|true|What environments your project will be compatible with.||
|authors|false|Main contributor(s) to your project.||
|sourcepage|false|Where to find the source code of your project.||

## Dependency Declarations
Dependencies are declared to later be referenced by configurations. You write the declaration, and Wisteria will handle
retrieval and updating.

### Maven artifacts
```toml
[dependencies.maven]
gson = { group_id = "com.google.code.gson", artifact_id = "gson" }
```
Maven artifacts are retrieved from Maven central by default. The latest release version will be downloaded unless specified with
`version`.
Classifiers can also be specified for more complicated repositories.

Other Maven repository URLs can be specified with `url = ...`

### Github releases
```toml
[dependencies.github]
lilac = { repository = "Khyonie/Lilac" }
```
The latest release will be retrieved by default. This can be specified by supplying `release-type = "latest"/"any"/"prerelease"`,
and specific tags can be specified with `tag = "v1.0.0"`. 

### Local libraries
Locally stored dependencies can be specified in two ways: reading everything in a folder, or referencing the file directly.
```toml
[dependencies.file]
proprietary-lib = { path = "lib/Proprietary4j.jar" }
```
```toml
[dependencies.folder]
local-libs = { path = "lib/", recursive = true" }
```

# Updating dependencies
After declaring your dependencies, run: 
```shell
wisteria update all
```
This will download the matching dependencies. Unless specified, these will be their latest versions. 
If reproducibility and breaking changes are concerns, consider explicitly setting the version/tag for each dependency you care about.

To update one specific dependency:
```shell
wisteria update gson
```
*Downloaded Dependencies will be written to `.wisteria/cache/`.*














