use std::collections::HashMap;

use toml::{Table, Value};

use crate::{configuration::Configuration, dependency::Dependency, nature::Nature, util::toml_utils};

/// Collection of identifying information for a project.
#[derive(Clone)]
pub struct ProjectInfo
{
    name: String,
    description: String,
    authors: Vec<String>,
    version: String,
    license: Vec<String>,
    homepage: Option<String>,
    sourcepage: Option<String>,
    natures: Vec<Nature>,
    configurations: HashMap<String, Configuration>
}

#[derive(Clone)]
pub struct Project
{
    info: ProjectInfo,
    dependencies: HashMap<String, Dependency>
}

impl Project
{
    pub fn from(toml: &Table, configuration_map: Option<&Value>, dependencies_map: Option<&Value>) -> Result<Self, (String, u8)> 
    {
        let name: String = toml_utils::read_string("name", toml)?;
        let version: String = toml_utils::read_string("version", toml)?;
        let info = ProjectInfo {
            name: name.clone(),
            description: toml_utils::read_string("description", toml)?,
            authors: toml_utils::read_string_array("authors", toml).unwrap_or(Vec::new()),
            version: version.clone(), 
            license: toml_utils::read_string_array("license", toml).unwrap_or(Vec::new()),
            homepage: toml_utils::read_string("homepage", toml).ok(),
            sourcepage: toml_utils::read_string("sourcepage", toml).ok(),
            natures: {
                let natures = toml_utils::read_string_array("natures", toml).unwrap_or(Vec::new())
                    .iter()
                    .filter_map(| v | {
                        match v.as_str() 
                        {
                            "eclipse" => Some(Nature::Eclipse),
                            "maven" => Some(Nature::Maven),
                            _ => None
                        }
                    })
                    .collect();


                natures
            },
            configurations: match configuration_map {
                Some(v) if v.is_table() => {
                    let v = v.as_table().unwrap();
                    let mut configurations: HashMap<String, Configuration> = HashMap::new();

                    for key in v.keys()
                    {
                        match v.get(key)
                        {
                            Some(config) if config.is_table() => {
                                let mut configuration = Configuration::from(key.clone(), config.as_table().unwrap(), name.clone(), version.clone())?;
                                configuration.apply_implicit();
                                configurations.insert(key.clone(), configuration)
                            }
                            Some(v) => return Err((format!("Mismatched type for task \"{key}\", expected a table, found {}", v.type_str()), 16)),
                            None => None
                        };
                    }

                    let mut updated_configurations: HashMap<String, Configuration> = HashMap::new();
                    for (config_name, configuration) in configurations.iter()
                    {
                        if let Some(target) = configuration.inherits()
                        {
                            if config_name.eq(target)
                            {
                                return Err((format!("Configuration \"{config_name}\" cannot inherit from itself"), 40));
                            }

                            let target = match configurations.get(target)
                            {
                                Some(c) => c,
                                None => return Err((format!("No such configuration \"{target}\" to be inherited by \"{config_name}\""), 41))
                            };


                            let mut inheritor: Configuration = configuration.clone();
                            inheritor.inherit_from(target);
                            inheritor.apply_implicit();
                            updated_configurations.insert(config_name.clone(), inheritor);
                        }
                    }

                    for (k, v) in updated_configurations
                    {
                        configurations.insert(k, v);
                    }

                    configurations
                }
                Some(v) => return Err((format!("Mismatched type for \"configuration\", expected a table, found {}", v.type_str()), 16)),
                None => HashMap::new()
            }
        };

        let dependencies: HashMap<String, Dependency> = match dependencies_map {
            Some(v) if v.is_table() => {
                let v = v.as_table().unwrap();
                let mut dependencies: HashMap<String, Dependency> = HashMap::new();

                for (name, t) in v
                {
                    match t.as_table() 
                    {
                        Some(t) => dependencies.insert(name.clone(), Dependency::load(t)?),
                        None => return Err((format!("Mismatched type for dependency \"{name}\", expected a table, found {}", t.type_str()), 16))
                    };
                }

                dependencies
            }
            Some(v) => return Err((format!("Mismatched type for \"dependencies\", expected a table, found {}", v.type_str()), 16)),
            None => HashMap::new()
        };

        Ok(Project { info, dependencies })
    }

    pub fn info(&self) -> &ProjectInfo
    {
        &self.info
    }

    pub fn dependencies(&self) -> &HashMap<String, Dependency>
    {
        &self.dependencies
    }

    pub fn print_info(&self) 
    {
        println!("╒══[ Information for project \"{}\" ]═════════════", self.info.name);
        println!("│\tDescription      {}", self.info.description);

        match self.info.authors.len()
        {
            0 => {}
            _ => println!("│\tAuthors          {}", toml_utils::string_vec_to_string(&self.info.authors))
        }

        println!("│\tVersion          {}", self.info.version);

        match self.info.license.len()
        {
            0 => {}
            _ => println!("│\tLicenses         {}", toml_utils::string_vec_to_string(&self.info.license))
        }

        if let Some(s) = &self.info.homepage
        {
            println!("│\tWebsite          {s}")
        }

        if let Some(s) = &self.info.sourcepage
        {
            println!("│\tSource           {s}")
        }

        println!("│\tConfigurations   {}", self.info.configurations.len());
        println!("│\tDependencies     {}", self.dependencies.len());

        if !self.dependencies.is_empty()
        {
            println!("╞ Dependencies:");
            for (name, dependency) in &self.dependencies
            {
                println!("│\t{:<16} ({})", name, dependency.type_str())
            }
        }
        println!("│");

        for c in self.info.configurations.values()
        {
            c.print_info()
        }
        println!("│\t*Depending on the configuration, Wisteria may automatically provide tasks such as \"build\".")
    }
}

#[allow(dead_code)]
impl ProjectInfo 
{
    pub fn name(&self) -> &str 
    {
        &self.name
    }

    pub fn description(&self) -> &str 
    {
        &self.description
    }

    pub fn authors(&self) -> &[String] 
    {
        &self.authors
    }

    pub fn version(&self) -> &str 
    {
        &self.version
    }

    pub fn license(&self) -> &[String] 
    {
        &self.license
    }

    pub fn homepage(&self) -> Option<&String> 
    {
        self.homepage.as_ref()
    }

    pub fn sourcepage(&self) -> Option<&String> 
    {
        self.sourcepage.as_ref()
    }

    pub fn natures(&self) -> &Vec<Nature>
    {
        self.natures.as_ref()
    }

    pub fn configurations(&self) -> &HashMap<String, Configuration>
    {
        &self.configurations
    }
}
