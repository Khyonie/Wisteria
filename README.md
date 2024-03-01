# Wisteria
Wisteria is a Java project manager that aims to be as simple to use and configure as possible, as well as being able to configure/handle complex projects and project structures.

For example, a project with multiple source folders can be easily configured as:
```toml
[project]
name = "MyProject"

[task]
source = [ "src/main/", "src/test/" ]
output = [ "target/{TASK_NAME}/{PROJECT_NAME}.jar" ]
```
## Usage
### *Create a new project...*
Run
```
$ wisteria new ProjectName my.personal.package`
```
... which will create a new folder to contain your project, a basic source folder structure, and a project configuration with sane defaults.

### *Or initialize an existing project...*
Run
```
$ wisteria init
```
which will derive a Wisteria project configuration from the project in the current directory.

### *Or upgrade an existing Wisteria 1.x.x project to 2.x.x...*
Run
```
$ wisteria convert
```

### *And to build the project...*
Run
```
$ wisteria build
```
which will compile the default task. If any targets are given ("`output`" in your project.toml), the classes will be packaged into a .jar for each target.
You may optionally give a specific task to build as well.

See the [Wisteria wiki](https://google.com) for all the different ways you can write new tasks and change compiler settings.
