use std::collections::HashMap;

use ini::{Ini, Properties};
use tower_lsp::jsonrpc;

use crate::utils;

pub struct MPLABFile {
    pub path: String,
    pub is_other: bool,
    pub is_generated: bool,
}

impl MPLABFile {
    fn new(path: String) -> Self {
        Self {
            path,
            is_other: false,
            is_generated: false,
        }
    }
}

pub struct MPLABProjectConfig {
    pub file_version: String,
    pub device: String,
    pub files: HashMap<String, MPLABFile>,
    pub suite_guid: String,
    pub tool_settings: Vec<(String, String)>,
}

impl MPLABProjectConfig {
    pub fn from_ini_to_lsp_result(ini: &Ini) -> jsonrpc::Result<Self> {
        MPLABProjectConfig::from_ini(ini).map_err(|e| utils::create_server_error(5, e))
    }

    pub fn from_ini(ini: &Ini) -> Result<Self, String> {
        fn get_section<'i>(ini: &'i Ini, section: &'static str) -> Result<&'i Properties, String> {
            Ok(ini
                .section(Some(section))
                .ok_or(format!("Section '{}' not found in .mcp", section))?)
        }
        fn get_field(section: &Properties, field: &'static str) -> Result<String, String> {
            Ok(String::from(
                section
                    .get(field)
                    .ok_or(format!("INI field '{}' not found...", field))?,
            ))
        }
        fn get_all_fields_in_section(
            ini: &Ini,
            section: &'static str,
        ) -> Result<Vec<(String, String)>, String> {
            Ok(get_section(ini, section)?
                .iter()
                .map(|(k, v)| (String::from(k), String::from(v)))
                .collect())
        }
        fn get_files(ini: &Ini) -> Result<HashMap<String, MPLABFile>, String> {
            fn get_file_names<'a>(
                ini: &'a Ini,
                mut files: HashMap<&'a str, MPLABFile>,
            ) -> Result<HashMap<&'a str, MPLABFile>, String> {
                for (key, value) in get_section(ini, "FILE_INFO")?.iter() {
                    files.insert(key, MPLABFile::new(value.to_owned()));
                }
                Ok(files)
            }
            fn add_is_generated<'a>(
                ini: &'a Ini,
                mut files: HashMap<&'a str, MPLABFile>,
            ) -> Result<HashMap<&'a str, MPLABFile>, String> {
                for (key, value) in get_section(ini, "GENERATED_FILES")?.iter() {
                    if value.contains("$(ProjectDir)") {
                        files.get_mut(key).unwrap().is_generated = true;
                    }
                }
                Ok(files)
            }
            fn add_is_other<'a>(
                ini: &'a Ini,
                mut files: HashMap<&'a str, MPLABFile>,
            ) -> Result<HashMap<&'a str, MPLABFile>, String> {
                for (key, value) in get_section(ini, "OTHER_FILES")?.iter() {
                    files
                        .get_mut(key)
                        .ok_or(format!("File key '{}' not found...", key))?
                        .is_other = value == "yes";
                }
                Ok(files)
            }
            fn key_to_owned(files: HashMap<&str, MPLABFile>) -> HashMap<String, MPLABFile> {
                files.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
            }

            let files = HashMap::new();
            let files = get_file_names(ini, files)?;
            let files = add_is_other(ini, files)?;
            let files = add_is_generated(ini, files)?;

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
