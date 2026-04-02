// src/utils/monet_scan.rs
use apk_info::{Apk, ARSC, AXML};
use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Debug, serde::Serialize)]
pub struct App {
    pub package_name: String,
    pub label: String,
}

pub fn check_monet(apk_path: &str) -> Option<App> {
    let apk = Apk::new(apk_path).ok()?;
    let package_name = apk.get_package_name()?;
    let label = apk
        .get_application_label()
        .unwrap_or_else(|| package_name.clone());

    let (arsc_data, _) = apk.read("resources.arsc").ok()?;
    let mut arsc_slice: &[u8] = &arsc_data;
    let arsc = ARSC::new(&mut arsc_slice).ok()?;

    let (manifest_data, _) = apk.read("AndroidManifest.xml").ok()?;
    let mut manifest_slice: &[u8] = &manifest_data;
    let manifest_axml = AXML::new(&mut manifest_slice, None).ok()?;

    let icon_raw = manifest_axml.get_attribute_value("application", "icon", None)?;
    let res_id = u32::from_str_radix(icon_raw.trim_start_matches('@'), 16).ok()?;
    let all_paths = arsc.get_all_resource_values(res_id);

    let xml_paths: Vec<_> = all_paths.iter().filter(|p| p.ends_with(".xml")).collect();

    for xml_path in &xml_paths {
        let (data, _) = apk.read(xml_path).ok()?;
        let mut slice: &[u8] = &data;
        let axml = AXML::new(&mut slice, None).ok()?;
        let xml = axml.get_xml_string();

        if xml.to_lowercase().contains("<monochrome") {
            return Some(App {
                package_name,
                label,
            });
        }
    }
    None
}

pub fn scan_dir(dir: &str) -> Vec<App> {
    let apk_paths: Vec<String> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension() == Some("apk".as_ref()))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    apk_paths
        .par_iter()
        .filter_map(|path| check_monet(path))
        .collect()
}
