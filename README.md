# Wisteria
Wisteria is a Java project manager and builder for Linux CLI-only environments, such as SSH sessions. Inspired by Rust-lang's Cargo project manager.

For example:
```toml
# Your basic project information, such as the name, current version,
# description, and what environments your project should be compatible
# with.
[project]
name = "WisteriaProject"
version = "0.1.0"
description = "An example of a project file."
natures = [ "eclipse", "maven" ] 

# Dependencies to be made available to configurations.
[dependencies.maven]
gson = { group_id = "com.google.code.gson", artifact_id = "gson" }

# Configurations define where to find source files, what dependencies to
# include, where to write completed builds, among other options.
# See the more in-depth configuration section for details.
[configuration.main]
sources = [ "src/" ]
dependencies = [  ] 
targets = [ "targets/{configuration}/{project_name}-{configuration}-{version}.jar" ]
```

## Usage:
### Create a new project
`-$ wisteria create <your project name>` 

Creates a folder in the current directory to contain the new project, with a source folder and a `project.toml` file.

Optionally, add the `--minimal` flag to create the project using a minimal `project.toml`.

### Build the current project configuration
`-$ wisteria build`

Compiles all .java source files inside the source folder(s) defined in the current project configuration, and packages them as a .jar file defined by the `targets` section of the current project configuration.

For a configuration to be valid for building, it must define `sources` and `targets`. Additionally if `entry` is specified, the resulting .jar file will be executable.

### Build and run the current project configuration
`-$ wisteria run`

Like build, this compiles all .java source files defined in the current project configuration. The program is then executed.

- To pass command-line JVM arguments to the program, write `--` as an argument. Any arguments written after will be passed to the program as-is.

Example: `-$ wisteria run -- -Xms4g -Xmx8g`

For a configuration te be valid for running, it must define `sources` and `entry`.

### Switch the project configuration
`-$ wisteria switch <configuration>`

Switches the project configuration to a different configuration defined in `project.toml`, and refreshes the workspace. See below for more details.

### Apply project settings to the workspace
`-$ wisteria refresh`

The current `project.toml` settings will apply to the project workspace by generating files based on the natures in `project.natures`. Useful to apply changes to the workspace without switching the current project configuration.
- "eclipse" nature generates `.project` and `.classpath` files.
- "maven" nature generates `pom.xml`.

Applicable dependencies may be updated with this action as well.

### Update dependencies
`-$ wisteria update [dependency | all]`

Downloads a specific Maven or Github dependency as defined under a `[dependencies.<source>]` table in `project.toml` and reconfigures the classpath to use the new file. Unless otherwise defined, the version selected will be the latest stable release.

If "all" is specified as the dependency, all applicable dependencies will be updated.

### Migrate a Wisteria 2 project
`-$ wisteria migrate wisteria2`

Converts a Wisteria 2 `project.toml` to the current format in place. Before writing the converted file, Wisteria writes a backup next to it using a name like `project.toml.wisteria2.bak`. If that backup already exists, the next available numbered backup is used.

The converter maps v2 `project.libraries` entries to local dependency groups, maps each `[task.<name>]` table to `[configuration.<name>]`, and converts common javac `arguments` into structured `compiler_flags` when possible. Use `--project <project file>` to migrate a project file with a different path.

## project.toml:
Projects are defined inside of a `project.toml` file at the root of the project hierarchy.

### `[project]`
Contains basic information about your project, such as the name, version, and description. The name and version can be referenced as `{project_name}` and `{version}` in a configuration's `targets`.

Also found here are a project's `natures`, which are "eclipse" and "maven" by default. Natures define what environments a project should be compatible with.

### `[dependencies.<source>]`
Declares the dependencies in use by your project and exposes them to be used by a configuration. Dependency declarations are grouped by source type, and each dependency can then be referenced by name from a configuration.
```toml
[dependencies.archive]
# Resolves the specified file
local-library = { path = "path/to/library.jar" }

[dependencies.folder]
# Adds all .jar files inside the given directory
project-libraries = { path = "lib/", recursive = true }

[dependencies.url]
# Downloads a file from a URL
remote-library = { url = "https://lib.example.com/snapshots/libexample.jar" }

[dependencies.maven]
# Downloads a file from Maven central (or another repository)
# If "version" is not specified, the latest stable version is downloaded
maven-library = { group_id = "com.example", artifact_id = "libexample" }

[dependencies.github]
# Downloads a release asset from a Github repository
# If "tag" is not specified, the latest non-prerelease tag asset is downloaded
github-library = { username = "Example", repository = "LibExample" }

# Use a pinned release tag
pinned-github-library = { username = "Example", repository = "LibExample", tag = "v1.2.3" }

# Or opt into prerelease tags when "tag" is omitted
preview-github-library = { username = "Example", repository = "LibExample", release_type = "prerelease" }
```

Recognized dependency groups are `archive`, `folder`, `url`, `maven`, `github`, `local_repository`, and `script`. Legacy flat dependency declarations with `type = "..."` are migrated in memory while loading, but new project files should use grouped tables.

### `[configuration.<config>]`
Defines the workspace settings that make up how a project should be interacted with and built.

```toml
# Configurations are referenced using the name given in the header, ex "main" or "testing"
[configuration.main]
sources = [ "src/api/", "src/app/" ] # Defines where to look for source files
dependencies = [ "local-library", "maven-library" ] # Adds the given dependencies to the classpath, resolving them automatically
targets = [ "targets/{configuration}/{project_name}-{configuration}-{version}.jar" ] # Defines where the final packaged .jar(s) will be written to

# Optional settings
entry = "com.example.App" # Defines the entry point of the .jar file, making it executable
shaded = [ "maven-library" ] # Shades the given library into the final .jar after packaging
includes = [ "metadata.json" ] # Defines what non-java files should be added into the final .jar
java_version = 14 # Defines the minimum java version the final .jar will be compatible with

# Multiple configurations can be added, and settings can be copied from one to another using "inherit"
[configuration.testing]
# Inherit the configuration from "main".
# After inheriting, any sources or dependencies are appended to the original configuration's definition.
inherit = "main"
sources = [ "src/testing/" ] # Now includes "src/api/", "src/main/", and "src/testing/"
dependencies = [ "gson" ] # Same as above
```
