use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use ini::{Ini, Properties};
use tree_sitter::Parser;

use crate::{Backend, TextDocument};

pub struct MPLABProjectConfig {
    pub file_version: String,
    pub device: String,
    pub files: HashMap<PathBuf, TextDocument>,
    pub suite_guid: String,
    pub tool_settings: Vec<(String, String)>,
}

impl MPLABProjectConfig {
    pub fn from_ini_with_root(
        ini: &Ini,
        root: PathBuf,
        parser: &mut Parser,
    ) -> Result<Self, String> {
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

        let header = get_section(ini, "HEADER")?;
        let file_version = get_field(header, "file_version")?;
        let device = get_field(header, "device")?;

        let suite_info = get_section(ini, "SUITE_INFO")?;
        let suite_guid = get_field(suite_info, "suite_guid")?;

        let tool_settings = get_section(ini, "TOOL_SETTINGS")?;
        let tool_settings = tool_settings
            .iter()
            .map(|(s1, s2)| (String::from(s1), String::from(s2)))
            .collect();

        let mut files = HashMap::new();
        for (key, value) in get_section(ini, "FILE_INFO")?.iter() {
            let mut absolute_path = root.clone();
            absolute_path.push(value);

            files.entry(key).or_insert(TextDocument {
                absolute_path,
                ..Default::default()
            });
        }

        for (key, value) in get_section(ini, "OTHER_FILES")?.iter() {
            files
                .get_mut(key)
                .ok_or(format!("File key '{}' not found...", key))?
                .is_other = value == "yes";
        }

        for (key, value) in get_section(ini, "GENERATED_FILES")?.iter() {
            files
                .get_mut(key)
                .ok_or(format!("File key '{}' not found...", key))?
                .is_generated = value == "yes";
        }

        let mut files = files
            .into_iter()
            .map(|(_, doc)| (doc.absolute_path.clone(), doc))
            // .map(|(absolute_path, mut doc)| {
            //     // let mut file = File::open(absolute_path.as_path()).map_err(|e| {
            //     //     format!(
            //     //         "Cannot open file '{}' ('{}')",
            //     //         absolute_path.display(),
            //     //         e.to_string()
            //     //     )
            //     // })?;
            //     // let mut raw = String::new();
            //     //
            //     // file.read_to_string(&mut raw).map_err(|_e| {
            //     //     format!(
            //     //         "Cannot read file '{}' ('{}')",
            //     //         absolute_path.display(),
            //     //         _e.to_string()
            //     //     )
            //     // })?;
            //     //
            //     // let syntax_tree = Some(
            //     //     parser
            //     //         .parse(raw.as_bytes(), None)
            //     //         .ok_or("Could not create syntax tree for '{}'...")?,
            //     // );
            //     (absolute_path, doc)
            // })
            .collect::<HashMap<_, _>>();

        for (absolute_path, doc) in files
            .iter_mut()
            .filter(|(_, doc)| !(doc.is_generated || doc.is_other))
        {
            let mut file = File::open(absolute_path.as_path()).map_err(|e| {
                format!(
                    "Cannot open file '{}' ('{}')",
                    absolute_path.display(),
                    e.to_string()
                )
            })?;
            let mut raw = String::new();

            file.read_to_string(&mut raw).map_err(|_e| {
                format!(
                    "Cannot read file '{}' ('{}')",
                    absolute_path.display(),
                    _e.to_string()
                )
            })?;

            let syntax_tree = Some(
                parser
                    .parse(raw.as_bytes(), None)
                    .ok_or("Could not create syntax tree for '{}'...")?,
            );

            doc.syntax_tree = syntax_tree;
            doc.raw = raw;
        }

        Ok(Self {
            file_version,
            device,
            files,
            suite_guid,
            tool_settings,
        })
    }
}
