use std::{collections::HashMap, path::{Path, PathBuf}};

use regex::Regex;
use toml::Table;

use crate::{configuration::Configuration, util::{files, toml_utils}};

#[derive(Clone)]
pub enum Dependency
{
    LocalFile{ path: String, javadoc: Option<String> },
    LocalFolder{ path: String, recursive: bool },
    LocalRepository{ repository: String, name: String, version: String, update_policy: UpdatePolicy, javadoc: Option<String> },

    FetchFromUrl{ url: String, update_policy: UpdatePolicy, javadoc: Option<String> },
    FetchFromMaven{ url: String, group_id: String, artifact_id: String, version: String, artifact_name: String, classifier: Option<String>, update_policy: UpdatePolicy, javadoc: Option<String> },
    FetchFromGithub{ username: String, repository: String, asset: String, tag: String, update_policy: UpdatePolicy, javadoc: Option<String> },

    BuildFromScript{ run: Vec<String>, target: String, update_policy: UpdatePolicy, javadoc: Option<String> }
}

#[derive(Clone, Default)]
pub enum UpdatePolicy 
{
    Always,
    #[default]
    SwitchOrUpdate,
    UpdateOnly,
    SwitchOrTask,
    SwitchConfigurationOnly,
    TaskOrUpdate,
    TaskInvokedOnly,
    Never
}

pub enum UpdateContext 
{
    Update,
    SwitchConfiguration,
    TaskInvoked,
    ResolveOnly
}

#[allow(dead_code, unused_variables)]
impl Dependency
{
    pub fn load(toml: &Table) -> Result<Dependency, (String, u8)>
    {
        match toml.get("type")
        {
            Some(val) if val.is_str() => {
                if val.as_str().unwrap() == "loadFolder"
                {
                    let path: String = toml_utils::read_string("path", toml)?;
                    let recursive: bool = toml_utils::read_boolean("recursive", toml).unwrap_or(true);

                    return Ok(Dependency::LocalFolder { path, recursive })
                }

                // Common fields
                let update_policy: UpdatePolicy = match toml_utils::read_string("update_policy", toml).unwrap_or(String::from("SwitchOrUpdate")).as_str() {
                    "Always" => UpdatePolicy::Always,
                    "SwitchOrUpdate" => UpdatePolicy::SwitchOrUpdate,
                    "UpdateOnly" => UpdatePolicy::UpdateOnly,
                    "SwitchOrTask" => UpdatePolicy::SwitchOrTask,
                    "SwitchConfigurationOnly" => UpdatePolicy::SwitchConfigurationOnly,
                    "TaskOrUpdate" => UpdatePolicy::TaskOrUpdate,
                    "TaskInvokedOnly" => UpdatePolicy::TaskInvokedOnly,
                    "Never" => UpdatePolicy::Never,
                    _ => return Err((String::from("Unexpected update policy, expected one of [Always, SwitchOrUpdate, UpdateOnly, SwitchOrTask, SwitchConfigurationOnly, TaskOrUpdate, TaskInvokedOnly, Never]"), 30))
                };
                let javadoc: Option<String> = toml_utils::read_string("javadoc", toml).ok();

                // Followed by the dependency-specific fields
                match val.as_str().unwrap()
                {
                    // Local types
                    "loadArchive" => {
                        let path: String = toml_utils::read_string("path", toml)?;

                        Ok(Dependency::LocalFile{ path, javadoc })
                    }
                    "localRepository" => {
                        let repository: String = toml_utils::read_string("repository", toml)?;
                        let name: String = toml_utils::read_string("name", toml)?;
                        let version: String = toml_utils::read_string("version", toml)?;

                        Ok(Dependency::LocalRepository { repository, name, version, update_policy, javadoc })
                    }

                    // External types
                    "fetchFromUrl" => {
                        let url: String = toml_utils::read_string("url", toml)?;

                        Ok(Dependency::FetchFromUrl { url, update_policy, javadoc })
                    }
                    "fetchFromMaven" => {
                        let url: String = toml_utils::read_string("url", toml).unwrap_or(String::from("https://repo1.maven.org/maven2/"));
                        let group_id: String = toml_utils::read_string("group_id", toml)?;
                        let artifact_id: String = toml_utils::read_string("artifact_id", toml)?;
                        let version: String = toml_utils::read_string("version", toml).unwrap_or(String::from("NEXUS_LATEST"));
                        let artifact_name: String = toml_utils::read_string("artifact_name", toml)
                            .unwrap_or(format!("{artifact_id}-{version}"));
                        let classifier: Option<String> = toml_utils::read_string("classifier", toml).ok();

                        Ok(Dependency::FetchFromMaven { url, group_id, artifact_id, version, artifact_name, classifier, update_policy, javadoc })
                    }
                    "fetchFromGithub" => {
                        let username: String = toml_utils::read_string("username", toml)?;
                        let repository: String = toml_utils::read_string("repository", toml)?;
                        let tag: String = toml_utils::read_string("tag", toml)?;

                        let asset: String = toml_utils::read_string("asset", toml).unwrap_or(repository.to_string());

                        Ok(Dependency::FetchFromGithub { username, repository, asset, tag, update_policy, javadoc })
                    }

                    // Etc.
                    "buildFromScript" => {
                        let run: Vec<String> = toml_utils::read_string_array("run", toml)?;
                        let target: String = toml_utils::read_string("target", toml)?;

                        Ok(Dependency::BuildFromScript { run, target, update_policy, javadoc })
                    }
                    _ => Err((format!("Unknown dependency type \"{}\"", val.as_str().unwrap()), 31))
                }
            },
            Some(val) => Err((format!("Unexpected input for dependency type, expected a string, found {}", val.type_str()), 32)),
            None => Err((String::from("Dependency must explicitly define its type"), 32))
        }
    }

    pub fn type_str(&self) -> &str
    {
        match self
        {
            Dependency::LocalFile { path: _, javadoc: _ } => "loadArchive",
            Dependency::LocalFolder { path: _, recursive: _ } => "loadFolder",
            Dependency::LocalRepository { repository: _, name: _, version: _, update_policy: _, javadoc: _ } => "localRepository",
            Dependency::FetchFromUrl { url: _, update_policy: _, javadoc: _ } => "fetchFromUrl",
            Dependency::FetchFromMaven { url: _, group_id: _, artifact_id: _, version: _, artifact_name: _, classifier: _, update_policy: _, javadoc: _ } => "fetchFromMaven",
            Dependency::FetchFromGithub { username: _, repository: _, asset: _, tag: _, update_policy: _, javadoc: _ } => "fetchFromGithub",
            Dependency::BuildFromScript { run: _, target: _, update_policy: _, javadoc: _ } => "buildFromScript",
        }
    }

    pub fn resolve(&self, name: &str, environment: &HashMap<String, String>, regexes: &HashMap<&str, Regex>, update: UpdateContext) -> Result<Vec<PathBuf>, (String, u8)>
    {
        match self 
        {
            Dependency::LocalFile { path, javadoc } => {
                // Resolve path
                let path = files::resolve_filepath(path, environment, regexes)?;
                let pathbuf = PathBuf::from(&path);

                if !pathbuf.exists()
                {
                    return Err((format!("Dependency \"{path}\" does not exist"), 63));
                }

                if pathbuf.is_dir()
                {
                    return Err((format!("Dependency \"{path}\" is a file, not a library. To load a folder, use a \"loadFolder\" dependency type"), 63));
                }

                let canon_path = match pathbuf.canonicalize() {
                    Ok(p) => p,
                    Err(e) => return Err((format!("Could not canonicalize path \"{path}\": {e}"), 62))
                };

                Ok(vec![canon_path])
            }
            Dependency::LocalRepository { repository, name, version, update_policy, javadoc } => todo!(),
            Dependency::FetchFromUrl { url, update_policy, javadoc } => {
                let filepath: String = format!(".wisteria/cache/{name}/{name}.jar");

                if update_policy.should_update(&update) 
                {
                    files::ensure_parents(&filepath).map_err(| e | (e, 1))?;
                    files::download(name.to_string(), url.to_string(), filepath.clone())?;
                } else {
                    println!("Not updating");
                }

                Ok(vec![PathBuf::from(filepath)])
            }
            Dependency::FetchFromMaven { url, group_id, artifact_id, version, artifact_name, classifier, update_policy, javadoc } => {
                let classifier_string = match classifier {
                    Some(s) => format!("-{}", s.clone()),
                    None => String::new()
                };

                let filepath = format!(".wisteria/cache/{group_id}/{artifact_id}/{version}/{artifact_id}{classifier_string}.jar");
                let path: PathBuf = PathBuf::from(&filepath);

                if update_policy.should_update(&update)
                {
                    files::ensure_parents(&filepath)
                        .map_err(| e | (e, 1))?;

                    if path.exists()
                    {
                        println!("Nothing to do");
                        return Ok(vec![path]);
                    }

                    let full_url = format!("{url}{}/{artifact_id}/{version}/{artifact_name}{classifier_string}.jar", group_id.replace('.', "/"));

                    files::download(artifact_id.to_string(), full_url, filepath)?;
                } else {
                    println!("Not updating");
                }

                Ok(vec![path])
            }
            Dependency::FetchFromGithub { username, repository, asset, tag, update_policy, javadoc } => {
                let filepath = format!(".wisteria/cache/{username}/{repository}/{tag}/{repository}.jar");
                let path: PathBuf = PathBuf::from(&filepath);
                
                if update_policy.should_update(&update) 
                {
                    files::ensure_parents(&filepath)
                        .map_err(| e | (e, 1))?;

                    if path.exists()
                    {
                        println!("Nothing to do");
                        return Ok(vec![path]);
                    }

                    let full_url = format!("https://github.com/{username}/{repository}/releases/download/{tag}/{asset}.jar");

                    files::download(repository.clone(), full_url, filepath)?;
                } else {
                    println!("Not updating");
                }

                Ok(vec![path])
            }
            Dependency::BuildFromScript { run, target, update_policy, javadoc } => {
                todo!()
            }
            Dependency::LocalFolder { path, recursive } => {
                let path = files::resolve_filepath(path, environment, regexes)?;
                let pathbuf = PathBuf::from(&path);

                if !pathbuf.exists()
                {
                    return Err((format!("Dependency folder \"{path}\" does not exist"), 63));
                }

                if pathbuf.is_file()
                {
                    return Err((format!("Dependency folder \"{path}\" is a regular file, not a folder"), 1));
                }

                let mut files: Vec<PathBuf> = Vec::new();

                if let Ok(dir) = pathbuf.read_dir()
                {
                    for file in dir.flatten() 
                    {
                        if file.path().is_dir()
                        {
                            if *recursive
                            {
                                collect_recursive(&file.path(), &mut files)
                            }
                            continue;
                        }

                        if file.file_name().to_string_lossy().ends_with(".jar")
                        {
                            files.push(file.path());
                        }
                    }
                }

                Ok(files)
            }
        }
    }

    pub fn is_shaded(&self, name: &str, configuration: &Configuration) -> Option<bool>
    {
        if configuration.shaded().is_none()
        {
            return None
        }

        let shaded = configuration.shaded().unwrap();
        match self
        {
            Dependency::LocalFile { path, javadoc } => Some(shaded.contains(&String::from(name))),
            Dependency::LocalFolder { path, recursive } => None,
            Dependency::LocalRepository { repository, name, version, update_policy, javadoc } => Some(shaded.contains(&String::from(name))),
            Dependency::FetchFromUrl { url, update_policy, javadoc } => Some(shaded.contains(&String::from(name))),
            Dependency::FetchFromMaven { url, group_id, artifact_id, version, artifact_name, classifier, update_policy, javadoc } => Some(shaded.contains(&String::from(name))),
            Dependency::FetchFromGithub { username, repository, asset, tag, update_policy, javadoc } => Some(shaded.contains(&String::from(name))),
            Dependency::BuildFromScript { run, target, update_policy, javadoc  } => Some(shaded.contains(&String::from(name)))
        }
    }
}

impl UpdatePolicy
{
    pub fn should_update(&self, context: &UpdateContext) -> bool
    {
        match self 
        {
            UpdatePolicy::Always => true,
            UpdatePolicy::Never => false,
            UpdatePolicy::SwitchOrUpdate => {
                match context {
                    UpdateContext::Update => true,
                    UpdateContext::SwitchConfiguration => true,
                    UpdateContext::TaskInvoked => false,
                    UpdateContext::ResolveOnly => false
                }
            }
            UpdatePolicy::UpdateOnly => {
                match context {
                    UpdateContext::Update => true,
                    UpdateContext::SwitchConfiguration => false,
                    UpdateContext::TaskInvoked => false,
                    UpdateContext::ResolveOnly => false
                }
            }
            UpdatePolicy::SwitchOrTask => {
                match context {
                    UpdateContext::Update => false,
                    UpdateContext::SwitchConfiguration => true,
                    UpdateContext::TaskInvoked => true,
                    UpdateContext::ResolveOnly => false
                }
            }
            UpdatePolicy::SwitchConfigurationOnly => {
                match context {
                    UpdateContext::Update => false,
                    UpdateContext::SwitchConfiguration => true,
                    UpdateContext::TaskInvoked => false,
                    UpdateContext::ResolveOnly => false
                }
            }
            UpdatePolicy::TaskOrUpdate => {
                match context {
                    UpdateContext::Update => true,
                    UpdateContext::SwitchConfiguration => false,
                    UpdateContext::TaskInvoked => true,
                    UpdateContext::ResolveOnly => false
                }
            }
            UpdatePolicy::TaskInvokedOnly => {
                match context {
                    UpdateContext::Update => false,
                    UpdateContext::SwitchConfiguration => false,
                    UpdateContext::TaskInvoked => true,
                    UpdateContext::ResolveOnly => false
                }
            }
        }
    }
}

// TODO Duplicate code
fn collect_recursive(path: &Path, files: &mut Vec<PathBuf>)
{
    if let Ok(dir) = path.read_dir()
    {
        for f in dir.flatten()
        {
            if f.path().is_dir()
            {
                collect_recursive(&f.path(), files);
                continue;
            }

            if f.file_name().to_string_lossy().ends_with(".jar")
            {
                files.push(f.path());
            }
        }
    }
}
