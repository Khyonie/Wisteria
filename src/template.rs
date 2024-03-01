use std::{collections::HashMap, path::PathBuf};

use crate::{project::Project, source, task::Task, Flags};

// project.toml
// Wisteria project configuration
//-------------------------------------------------------------------------------- 
pub const PROJECT_TOML_TEMPLATE: &str = 
r#"# This is where you can configure your project and add/remove/configure tasks 
[project]
# The name of your project
name = "{PROJECT_NAME}"

#-------------------------------------------------------------------------------- 
# Default task settings (optional)
# All tasks inherit settings from this task. If this task does not exist, all tasks must specify at minimum "source" and "output"
# Keys with a ★ are required for the task to be able to be built/ran
[task]
# A short descriptor of this task
#description = A Java project

# ★ Source folders for your project 
#   After changing this, run wisteria update to update the classpath and reflect any changes
source = [ "src/" ]

# ★ Build targets
output = [ "target/{TASK_NAME}/{PROJECT_NAME}.jar" ]

# Libraries, individual .jar files will be added to the classpath when building and running. Folders will have their contents added recursively
# After changing this, run wisteria update to update the classpath and reflect any changes
libraries = [ "lib/" ]

# Individual .jar files to be shaded in with every target (that is, their contained classes will be stored inside each target)
# Shaded jars will be included on the compilation classpath but will not be included in an automatically generated MANIFEST.MF classpath
#shaded_jars = [ "lib/MyLittleLibrary.jar" ]

# If your project is an executable, specify the entry point (where your static void main() is)
# This setting is required to run a task as executable
#entry = my.program.EntryClass

# Files to package in when building this task
#include = []

# Options to be passed to javac when compiling
# See the Wisteria wiki for more compiler options
#[task.compiler]
#enable_preview_features = true
#extra_flags = "--Xlint:cast" 

# Minimum version of Java required for this task to build or to be run
#java_version = 8

# Javadocs for given libraries
# Add javadocs in a subsection of your task. Section names must match exactly with a library, including its relative path
# After changing this, run wisteria update to update classpath and reflect any changes
#[task.javadocs]
#"lib/MyLibrary.jar" = "https://a.link/to/javadocs"

#-------------------------------------------------------------------------------- 
# Add new tasks by writing a header with the name "task_<name>"
[task_main]

"#;

pub fn generate_project_toml(project: &Project) -> String
{
    PROJECT_TOML_TEMPLATE.replace("{PROJECT_NAME}", project.get_name())
}

pub const STARTER_TEMPLATE: &str = 
r#"package {PACKAGE};

public class {PROJECT_NAME}
{
    // Your code here
}"#;

pub fn generate_starter(project: &Project, package: &String) -> String
{
    STARTER_TEMPLATE.replace("{PACKAGE}", package).replace("{PROJECT_NAME}", project.get_name())
}

// 
// Flavor text
// Little fun lines at the end of compilation
//--------------------------------------------------------------------------------
pub const FLAVOR_TEXT: [&str; 76] = [
    "Enjoy your coffee!",
    "Enjoy your latte!",
    "Enjoy your cappuccino!",
    "How's that café wifi treating you?",
    "Shoutouts to Seycara Orchestral!",
    "Shoutouts to Waterflame!",
    "Have a great day.",
    "Trans rights are human rights!",
    "You're doing great! <3",
    "XOXOXOXOXO",
    "Breathe, calm yourself...",
    "In four, out four, in in, out four, repeat...",
    "Thanks for using Wisteria!",
    "See you again soon!",
    "Back again so soon?",
    "Welcome back!",
    "Good to see you!",
    "Just one more compilation",
    "Just a couple more lines, mom!",
    "You have a right to privacy!",
    "Protect your privacy!",
    "I use arch btw",
    "Also try Rust!",
    "Also try Kotlin!",
    "Also try Lua!",
    "Also try C!",
    "Also try C++!",
    "Also try C#!",
    "Also try Go!",
    "Also try JavaScript!",
    "Also try TypeScript!",
    "Also try Python!",
    "Also try Ubuntu!",
    "Also try Fedora!",
    "Also try Gentoo!",
    "The correct distro is the one you like!",
    "Windows users, Mac users, Linux users, we all make awesome things!",
    "Your program is finished \\(^-^)/",
    "I'm aww done, master! ^w^",
    "Wisteria 2! Better than Wisteria 1, I think",
    "Written in Rust!",
    "Licensed under GPLv3-or-later!",
    "FLOSS FTW!",
    "Protect others who can't protect themselves!",
    "Fight for human rights!",
    "Support women in tech!",
    "Support women in STEM!",
    "Fight for right to repair!",
    "Shoutouts to Louis Rossmann!",
    "Black lives matter!",
    "Your voice is heard!",
    "Look at you, fixing all those bugs!",
    "The year of the Linux desktop is not coming btw",
    "Checking vibes... [ GOOD ]",
    "You have the right to display your emotions!",
    "You can do anything, and if not right now, definitely tomorrow!",
    "Sleep is important!",
    "Research polyphasic sleep!",
    "Drink some water!",
    "Stand up, walk around for a minute, and come back!",
    "Are you following convention?",
    "When's the last time you made a backup of your code?",
    "FOSS software lives forever!",
    "Proprietary software dies, in time!",
    "Hey guys, did you know that in terms of-",
    "Curse you, Perry the Platypus!",
    "I like shorts! They are comfy and easy to wear!",
    "Lets climb out of here, together.",
    "Memes, the DNA of the soul...",
    "Was that the bite of '87?!",
    "I like your outfit!",
    "I like your hair!",
    "I like your shoes!",
    "Looking good!",
    "Use FOSS whenever possible!",
    "I added like, 50 unreproducible bugs, thought you should know :)"
];

// 
// MANIFEST.MF
// Java manifest file
//--------------------------------------------------------------------------------
pub const MANIFEST_MF_TEMPLATE: &str =
r#"Manifest-Version: 1.0
Created-By: Wisteria{ENTRY_CLASS}{CLASS_PATH}

"#;

pub fn generate_manifest(task: &Task, flags: &Flags) -> String
{
    let entry_class = match task.get_entry() {
        Some(e) => format!("\nMain-Class: {}", e),
        None => "".to_string()
    };

    let mut classpath = String::from("");

    if task.get_libraries().is_some()
    {
        let classpath_str = source::generate_classpath_exclude_shaded(task, flags);

        if !classpath_str.is_empty() && classpath_str != " "
        {
            classpath.push_str("\nClass-Path: ");
            classpath.push_str(&classpath_str);
        }
    }

    MANIFEST_MF_TEMPLATE.replace("{ENTRY_CLASS}", &entry_class).replace("{CLASS_PATH}", &classpath)
}

// 
// .classpath
// Project classpath file 
//-------------------------------------------------------------------------------- 
pub const CLASSPATH_SOURCE_TEMPLATE: &str = r#"    <classpathentry kind="src" output="target/classes" path="{SOURCE}"/>"#;
pub const CLASSPATH_LIBRARY_TEMPLATE: &str = r#"    <classpathentry kind="lib" path="{RELATIVE_LIBRARY}"/>"#;
pub const CLASSPATH_LIBRARY_WITH_JAVADOC_TEMPLATE: &str = 
r#"    <classpathentry kind="lib" path="{RELATIVE_LIBRARY}">    
        <attributes>
            <attribute name="javadoc_location" value="{JAVADOC}"/>
        </attributes>
    </classpathentry>"#;
pub const CLASSPATH_TEMPLATE: &str = 
r#"<?xml version="1.0" encoding="UTF-8"?>
<classpath>{SOURCES}{LIBRARIES}
    <classpathentry kind="con" path="org.eclipse.jdt.launching.JRE_CONTAINER"/>
    <classpathentry kind="con" path="org.eclipse.m2e.MAVEN2_CLASSPATH_CONTAINER"/>
    <classpathentry kind="output" path="target/classes"/> 
</classpath>"#;

pub fn generate_classpath(task: &Task, flags: &Flags) -> String
{
    let sources = match task.get_sources().len()
    {
        0 => "".to_string(),
        _ => {
            let mut buffer = String::new();

            for s in task.get_sources()
            {
                buffer.push_str(&format!("\n{}", CLASSPATH_SOURCE_TEMPLATE.replace("{SOURCE}", s)));
            }

            buffer
        }
    };

    let libraries = match task.get_libraries()
    {
        Some(_) => {
            let javadocs = match task.javadocs() {
                Some(j) => j.clone(),
                None => HashMap::new()
            };

            let mut libraries = Vec::new(); 
            for lib in task.get_relative_libraries()
            {
                source::collect_files_relative(PathBuf::from(lib), ".jar", &mut libraries, flags);
            }

            let mut buffer = String::new();
            for lib in &libraries
            {
                match javadocs.get(lib)
                {
                    Some(j) => buffer.push_str(&format!("\n{}", CLASSPATH_LIBRARY_WITH_JAVADOC_TEMPLATE.replace("{RELATIVE_LIBRARY}", lib).replace("{JAVADOC}", j))),
                    None => buffer.push_str(&format!("\n\t{}", CLASSPATH_LIBRARY_TEMPLATE.replace("{RELATIVE_LIBRARY}", lib)))
                }
            }

            buffer
        }, 
        None => "".to_string()
    };

    CLASSPATH_TEMPLATE.replace("{SOURCES}", &sources).replace("{LIBRARIES}", &libraries)
}

//
// .project
// Project configuration file, for Eclipse compatibility
//-------------------------------------------------------------------------------- 
pub const PROJECT_XML_TEMPLATE: &str = 
r#"<?xml version="1.0" encoding="UTF-8"?>
<projectDescription>
    <name>{PROJECT_NAME}</name>
    <comment></comment>
    <projects></projects>
    <buildSpec>
        <buildCommand>
            <name>org.eclipse.jdt.core.javabuilder</name>
            <arguments></arguments>
        </buildCommand>
        <buildCommand>
            <name>org.eclipse.m2e.core.maven2Builder</name>
            <arguments></arguments>
        </buildCommand>
    </buildSpec>
    <natures>
        <nature>org.eclipse.jdt.core.javanature</nature>
        <nature>org.eclipse.m2e.core.maven2Nature</nature>
    </natures>
</projectDescription>"#;

pub fn generate_project(project: &Project) -> String
{
    PROJECT_XML_TEMPLATE.replace("{PROJECT_NAME}", project.get_name())
}

//
// .settings/org.eclipse.jdt.core.pref
// Eclipse preferences file
//-------------------------------------------------------------------------------- 
pub const ECLIPSE_CORE_PREF_TEMPLATE: &str = 
r#"eclipse.preferences.version=1
org.eclipse.jdt.core.compiler.codegen.targetPlatform={JAVA_VERSION}
org.eclipse.jdt.core.compiler.compliance={JAVA_VERSION}
org.eclipse.jdt.core.compiler.problem.assertIdentifier=error
org.eclipse.jdt.core.compiler.problem.enumIdentifier=error
org.eclipse.jdt.core.compiler.source={JAVA_VERSION}"#;

pub fn generate_eclipse_core_perfs(task: &Task) -> String
{
    let java_version = task.java_version().unwrap_or(8);

    ECLIPSE_CORE_PREF_TEMPLATE.replace("{}", &java_version.to_string())
}

//
// .settings/org.eclipse.m2e.core.pref
// Maven plugin preferences file
//-------------------------------------------------------------------------------- 
pub const MAVEN_CORE_PREF_TEMPLATE: &str = r#"eclipse.preferences.version=1"#;

pub fn generate_maven_core_prefs() -> String
{
    String::from(MAVEN_CORE_PREF_TEMPLATE)
}

//
// pom.xml
// Maven configuration file
//-------------------------------------------------------------------------------- 
pub const POM_XML_TEMPLATE: &str = 
r#"<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema/instance" xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>{USER_PACKAGE}</groupId>
    <artifactId>{PROJECT_NAME}</artifactId>
    <version>0.0.1-SNAPSHOT</version>
</project>"#;

pub fn generate_pom_xml(project: &Project, package: &String) -> String
{
    POM_XML_TEMPLATE.replace("{USER_PACKAGE}", package).replace("{PROJECT_NAME}", project.get_name())
}
