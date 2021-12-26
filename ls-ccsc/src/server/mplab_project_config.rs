use std::collections::HashMap;

use ini::{Ini, Properties};
use tower_lsp::jsonrpc;

use crate::utils;

pub struct MPLABFile {
    pub path: String,
    pub subfolder: String,
    pub is_other: bool,
    pub is_generated: bool,
}

impl MPLABFile {
    fn new(path: String) -> Self {
        Self {
            path,
            subfolder: ".".to_owned(),
            is_other: false,
            is_generated: false,
        }
    }
}

// TODO: Implement more fields
pub struct MPLABProjectConfig {
    pub file_version: String,
    pub device: String,
    pub files: HashMap<String, MPLABFile>,
    pub suite_guid: String,
    pub tool_settings: Vec<(String, String)>,
}

type SResult<T> = Result<T, String>;

impl MPLABProjectConfig {
    pub fn from_ini_to_lsp_result(ini: &Ini) -> jsonrpc::Result<Self> {
        MPLABProjectConfig::from_ini(ini).map_err(|e| utils::create_server_error(5, e))
    }

    pub fn from_ini(ini: &Ini) -> SResult<Self> {
        fn get_section<'i>(ini: &'i Ini, section: &str) -> SResult<&'i Properties> {
            Ok(ini
                .section(Some(section))
                .ok_or(format!("Section '{}' not found in .mcp", section))?)
        }
        fn get_field(section: &Properties, field: &str) -> SResult<String> {
            Ok(String::from(
                section
                    .get(field)
                    .ok_or(format!("INI field '{}' not found...", field))?,
            ))
        }
        fn get_all_fields_in_section(ini: &Ini, section: &str) -> SResult<Vec<(String, String)>> {
            Ok(get_section(ini, section)?
                .iter()
                .map(|(k, v)| (String::from(k), String::from(v)))
                .collect())
        }
        fn get_files(ini: &Ini) -> SResult<HashMap<String, MPLABFile>> {
            type MPLABMap<'a> = HashMap<&'a str, MPLABFile>;
            fn get_file_names<'a>(ini: &'a Ini, mut f: MPLABMap<'a>) -> SResult<MPLABMap<'a>> {
                for (key, value) in get_section(ini, "FILE_INFO")?.iter() {
                    f.insert(key, MPLABFile::new(value.to_owned()));
                }
                Ok(f)
            }
            fn add_is_generated<'a>(ini: &'a Ini, mut f: MPLABMap<'a>) -> SResult<MPLABMap<'a>> {
                for (key, value) in get_section(ini, "GENERATED_FILES")?.iter() {
                    if value.contains("$(ProjectDir)") {
                        f.get_mut(key).unwrap().is_generated = true;
                    }
                }
                Ok(f)
            }
            fn add_is_other<'a>(ini: &'a Ini, mut f: MPLABMap<'a>) -> SResult<MPLABMap<'a>> {
                for (key, value) in get_section(ini, "OTHER_FILES")?.iter() {
                    f.get_mut(key)
                        .ok_or(format!("File key '{}' not found...", key))?
                        .is_other = value == "yes";
                }
                Ok(f)
            }
            fn add_subfolder<'a>(ini: &'a Ini, mut f: MPLABMap<'a>) -> SResult<MPLABMap<'a>> {
                for (key, value) in get_section(ini, "FILE_SUBFOLDERS")?.iter() {
                    f.get_mut(key)
                        .ok_or(format!("File key '{}' not found...", key))?
                        .subfolder = value.to_owned();
                }
                Ok(f)
            }
            fn key_to_owned(files: MPLABMap) -> HashMap<String, MPLABFile> {
                files.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
            }

            let files = HashMap::new();
            let files = get_file_names(ini, files)?;
            let files = add_is_other(ini, files)?;
            let files = add_is_generated(ini, files)?;
            let files = add_subfolder(ini, files)?;

            Ok(key_to_owned(files))
        }

        let header = get_section(ini, "HEADER")?;
        let file_version = get_field(header, "file_version")?;
        let device = get_field(header, "device")?;

        let suite_info = get_section(ini, "SUITE_INFO")?;
        let suite_guid = get_field(suite_info, "suite_guid")?;

        let tool_settings = get_all_fields_in_section(ini, "TOOL_SETTINGS")?;
        let files = get_files(ini)?;

        Ok(Self {
            file_version,
            device,
            files,
            suite_guid,
            tool_settings,
        })
    }
}
