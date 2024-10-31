use toml::Value;

#[derive(Clone, PartialEq, Eq)]
pub enum CompilerFlags
{
    ReleaseTarget{ version: u8 },           // --release {VALUE}
    EnablePreviewFeatures{ setting: bool }, // --enable-preview

    // No more than one of:
    JavadocAllLints{ setting: bool },       // -Xdoclint:all
    JavadocLints{ lints: Vec<String> },     // -Xdoclint:{VALUE{,VALUE}}

    // No more than one of:
    SourceLintAll{ setting: bool },         // -Xlint:all
    SourceLints{ lints: Vec<String> },      // -Xlint:{VALUE{,VALUE}}
    NoWarnings{ setting: bool },            // -nowarn

    DeprecationInfo{ setting: bool },       // -deprecation
    StoreParameterNames{ setting: bool },   // -parameters
    Encoding{ encoding: String }            // --encoding {VALUE}
}

impl CompilerFlags
{
    pub fn from(name: &str, value: &Value) -> Result<Self, (String, u8)>
    {
        match name
        {
            "release_target" => {
                match value.as_integer()
                {
                    Some(i) => {
                        if i.is_negative() 
                        {
                            return Err((format!("Illegal Java version {i}, version must be positive"), 52));
                        }

                        match i.try_into()
                        {
                            Ok(version) => Ok(Self::ReleaseTarget { version }),
                            Err(_) => Err((format!("Java version out of range, expected a number between 255, found {i}"), 52))
                        }
                    }
                    None => Err((format!("Mismatched type for compiler flag \"release_target\", expected an integer, found {}", value.type_str()), 14))
                }
            }
            "enable_preview_features" => {
                match value.as_bool()
                {
                    Some(setting) => Ok(Self::EnablePreviewFeatures { setting }),
                    None => Err((format!("Mismatched type for compiler flag \"enable_preview_features\", expected a boolean, found {}", value.type_str()), 12))
                }
            }
            "javadoc_all_lints" => {
                match value.as_bool()
                {
                    Some(setting) => Ok(Self::JavadocAllLints { setting }),
                    None => Err((format!("Mismatched type for compiler flag \"javadoc_all_lints\", expected a boolean, found {}", value.type_str()), 12))
                }
            }
            "javadoc_lints" => {
                match value.as_array()
                {
                    Some(array) => {
                        let mut lints: Vec<String> = Vec::new();

                        for v in array 
                        {
                            match v.as_str()
                            {
                                Some(s) => lints.push(s.to_string()),
                                None => return Err((format!("Mismatched type for lint in \"javadoc_lints\", expected a string, found {}", v.type_str()), 15))
                            }
                        }

                        if lints.is_empty()
                        {
                            return Err((String::from("At least one lint must be given"), 51));
                        }

                        Ok(Self::JavadocLints { lints })
                    }
                    None => Err((format!("Mismatched type for compiler flag \"javadoc_lints\", expected a string array, found {}", value.type_str()), 13))
                }
            }
            "source_all_lints" => {
                match value.as_bool()
                {
                    Some(setting) => Ok(Self::SourceLintAll { setting }),
                    None => Err((format!("Mismatched type for compiler flag \"source_all_lints\", expected a boolean, found {}", value.type_str()), 12))
                }
            },
            "source_lints" => {
                match value.as_array()
                {
                    Some(array) => {
                        let mut lints: Vec<String> = Vec::new();

                        for v in array 
                        {
                            match v.as_str()
                            {
                                Some(s) => lints.push(s.to_string()),
                                None => return Err((format!("Mismatched type for lint in \"source_lints\", expected a string, found {}", v.type_str()), 15))
                            }
                        }

                        if lints.is_empty()
                        {
                            return Err((String::from("At least one lint must be given"), 51));
                        }

                        Ok(Self::SourceLints { lints })
                    }
                    None => Err((format!("Mismatched type for compiler flag \"source_lints\", expected a string array, found {}", value.type_str()), 13))
                }
            }
            "no_warnings" => {
                match value.as_bool()
                {
                    Some(setting) => Ok(Self::NoWarnings { setting }),
                    None => Err((format!("Mismatched type for compiler flag \"no_warnings\", expected a boolean, found {}", value.type_str()), 12))
                }
            },
            "deprecation_info" => {
                match value.as_bool()
                {
                    Some(setting) => Ok(Self::DeprecationInfo { setting }),
                    None => Err((format!("Mismatched type for compiler flag \"deprecation_info\", expected a boolean, found {}", value.type_str()), 12))
                }
            },
            "store_parameter_names" => {
                match value.as_bool()
                {
                    Some(setting) => Ok(Self::StoreParameterNames { setting }),
                    None => Err((format!("Mismatched type for compiler flag \"store_parameter_names\", expected a boolean, found {}", value.type_str()), 12))
                }
            },
            "source_encoding" => {
                match value.as_str()
                {
                    Some(encoding) => Ok(Self::Encoding { encoding: encoding.to_string() }),
                    None => Err((format!("Mismatched type for compiler flag \"source_encoding\", expected a string, found {}", value.type_str()), 11))
                }
            },
            _ => Err((format!("Unrecognized compiler flag {name}"), 50))
        }
    }

    pub fn get_canon_flag(&self) -> Vec<String> 
    {
        match self 
        {
            CompilerFlags::ReleaseTarget { version } => vec![String::from("-release"), version.to_string()],
            CompilerFlags::EnablePreviewFeatures { setting } => {
                if *setting 
                {
                    return vec![String::from("--enable-preview")];
                }

                Vec::new()
            }
            CompilerFlags::JavadocAllLints { setting } => {
                if *setting 
                {
                    return vec![String::from("-Xdoclint:all")];
                }

                Vec::new()
            }
            CompilerFlags::JavadocLints { lints } => {
                let mut flag: String = String::from("-Xdoclint:");

                for l in lints 
                {
                    flag.push_str(l);
                    flag.push(',');
                }

                flag.pop();

                vec![flag]
            }
            CompilerFlags::SourceLintAll { setting } => {
                if *setting 
                {
                    return vec![String::from("-Xlint:all")];
                }

                Vec::new()
            }
            CompilerFlags::SourceLints { lints } => {
                let mut flag: String = String::from("-Xlint:");

                for l in lints 
                {
                    flag.push_str(l);
                    flag.push(',');
                }

                flag.pop();

                vec![flag]
            }
            CompilerFlags::NoWarnings { setting } => {
                if *setting 
                {
                    return vec![String::from("-nowarn")];
                }

                Vec::new()
            }
            CompilerFlags::DeprecationInfo { setting } => {
                if *setting 
                {
                    return vec![String::from("-deprecation")];
                }

                Vec::new()
            }
            CompilerFlags::StoreParameterNames { setting } => {
                if *setting 
                {
                    return vec![String::from("-parameters")];
                }

                Vec::new()
            }
            CompilerFlags::Encoding { encoding } => vec![String::from("--encoding"), encoding.to_string()],
        }
    }
}
