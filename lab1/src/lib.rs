use fs2::free_space;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;
use xmltree::Element;
use zip::write::ExtendedFileOptions;
use zip::{read::ZipArchive, write::FileOptions, ZipWriter};

pub fn run() -> Result<()> {
    loop {
        println!("\nOS Utility Lab (osul)");
        println!("1. View disk info");
        println!("2. Filesystem manipulation command utilities");
        println!("3. JSON manipulation command utilities");
        println!("4. XML manipulation command utilities");
        println!("5. Zip files command utilities");
        println!("0. Exit");

        match get_choice()? {
            1 => cmd_disks()?,
            2 => file_menu()?,
            3 => json_menu()?,
            4 => xml_menu()?,
            5 => zip_menu()?,
            0 => break,
            _ => println!("Invalid choice, try again."),
        }
    }
    Ok(())
}

fn sanitize_path(input: &Path, allow_nonexistent: bool) -> Result<PathBuf> {
    let cwd =
        std::env::current_dir().context("getting current working directory")?;
    let canonical_cwd = std::fs::canonicalize(&cwd).with_context(|| {
        format!("failed to canonicalize cwd '{}'", cwd.display())
    })?;

    let abs = if input.is_absolute() {
        input.to_path_buf()
    } else {
        cwd.join(input)
    };

    if !allow_nonexistent {
        let canonical = std::fs::canonicalize(&abs).with_context(|| {
            format!("failed to canonicalize '{}'", abs.display())
        })?;
        if canonical.starts_with(&canonical_cwd) {
            return Ok(canonical);
        } else {
            return Err(anyhow!(
                "Access denied: '{}' is outside of working directory '{}'",
                canonical.display(),
                canonical_cwd.display(),
            ));
        }
    }

    let mut ancestor = abs.as_path();
    let mut missing: Vec<OsString> = Vec::new();

    while !ancestor.exists() {
        if let Some(name) = ancestor.file_name() {
            missing.push(name.to_os_string());
        } else {
            break;
        }
        if let Some(parent) = ancestor.parent() {
            ancestor = parent;
        } else {
            break;
        }
    }

    let mut canonical_base = if ancestor.exists() {
        std::fs::canonicalize(ancestor).with_context(|| {
            format!("failed to canonicalize ancestor '{}'", ancestor.display())
        })?
    } else {
        canonical_cwd.clone()
    };

    for comp in missing.iter().rev() {
        canonical_base.push(comp);
    }

    if canonical_base.starts_with(&canonical_cwd) {
        Ok(canonical_base)
    } else {
        Err(anyhow!(
            "Access denied: '{}' is outside of working directory '{}'",
            canonical_base.display(),
            canonical_cwd.display(),
        ))
    }
}

fn get_choice() -> Result<u32> {
    print!("> ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().parse::<u32>().unwrap_or(999))
}

fn get_input(prompt: &str) -> Result<String> {
    print!("{prompt}: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

fn file_menu() -> Result<()> {
    loop {
        println!("\nFile System Utilities");
        println!("1. Create file");
        println!("2. Write to file");
        println!("3. Read file");
        println!("4. Delete file");
        println!("0. Cancel");

        match get_choice()? {
            1 => {
                let path = get_input("Enter file path")?;
                file_create(&PathBuf::from(path))?;
                return Ok(());
            }
            2 => {
                let path = get_input("Enter file path")?;
                let content = get_input("Enter content")?;
                file_write(&PathBuf::from(path), &content)?;
                return Ok(());
            }
            3 => {
                let path = get_input("Enter file path")?;
                file_read(&PathBuf::from(path))?;
                return Ok(());
            }
            4 => {
                let path = get_input("Enter file path")?;
                file_delete(&PathBuf::from(path))?;
                return Ok(());
            }
            0 => return Ok(()),
            _ => println!("Invalid choice"),
        }
    }
}

fn json_menu() -> Result<()> {
    loop {
        println!("\nJSON Utilities");
        println!("1. Create JSON (content or editor)");
        println!("2. New JSON object");
        println!("3. Read JSON file");
        println!("4. Delete JSON file");
        println!("0. Cancel");

        match get_choice()? {
            1 => {
                let path = get_input("Enter file path")?;
                let choice =
                    get_input("Provide content (c) or open editor (e)?")?;
                if choice == "c" {
                    let content = get_input("Enter JSON content")?;
                    json_create(PathBuf::from(path), Some(content), false)?;
                } else {
                    json_create(PathBuf::from(path), None, true)?;
                }
                return Ok(());
            }
            2 => {
                let path = get_input("Enter file path")?;
                json_interactive(&PathBuf::from(path))?;
                return Ok(());
            }
            3 => {
                let path = get_input("Enter file path")?;
                json_read(&PathBuf::from(path))?;
                return Ok(());
            }
            4 => {
                let path = get_input("Enter file path")?;
                file_delete(&PathBuf::from(path))?;
                return Ok(());
            }
            0 => return Ok(()),
            _ => println!("Invalid choice"),
        }
    }
}

fn json_interactive(path: &Path) -> Result<()> {
    use serde_json::json;
    let path = sanitize_path(path, true)?;
    let mut map = serde_json::Map::new();

    println!("\n--- Interactive JSON Creator ---");
    println!("Enter key-value pairs. Leave key empty to finish.\n");

    loop {
        let key = get_input("Key (empty to finish)")?;
        if key.is_empty() {
            break;
        }
        let value_str = get_input("Value")?;

        // Try to parse as JSON literal (number, bool, null, array, or object)
        let value: JsonValue = match serde_json::from_str(&value_str) {
            Ok(v) => v,
            Err(_) => json!(value_str), // treat as string
        };
        map.insert(key, value);
    }

    let json_obj = JsonValue::Object(map);
    let ser = serde_json::to_vec_pretty(&json_obj)?;
    fs::write(&path, ser)?;
    println!("Created interactive JSON file: {}", path.display());
    Ok(())
}

fn xml_menu() -> Result<()> {
    loop {
        println!("\nXML Utilities");
        println!("1. New XML file");
        println!("2. Write/append to XML");
        println!("3. Read XML file");
        println!("4. Delete XML file");
        println!("0. Cancel");

        match get_choice()? {
            1 => {
                let path = get_input("Enter file path")?;
                xml_new(&PathBuf::from(path))?;
                return Ok(());
            }
            2 => {
                let path = get_input("Enter file path")?;
                let content = get_input("Enter XML content or text")?;
                xml_write(&PathBuf::from(path), &content)?;
                return Ok(());
            }
            3 => {
                let path = get_input("Enter file path")?;
                xml_read(&PathBuf::from(path))?;
                return Ok(());
            }
            4 => {
                let path = get_input("Enter file path")?;
                file_delete(&PathBuf::from(path))?;
                return Ok(());
            }
            5 => {
                let path = get_input("Enter file path")?;
                xml_interactive(&PathBuf::from(path))?;
                return Ok(());
            }
            0 => return Ok(()),
            _ => println!("Invalid choice"),
        }
    }
}

fn xml_interactive(path: &Path) -> Result<()> {
    let path = sanitize_path(path, true)?;
    println!("\n--- Interactive XML Creator ---");
    println!("You will create a root element and add child elements.\n");

    let root_name = get_input("Enter root element name")?;
    let mut root = Element::new(root_name.as_str());

    loop {
        let tag = get_input("Child tag name (empty to finish)")?;
        if tag.is_empty() {
            break;
        }
        let value = get_input("Text content")?;
        let mut child = Element::new(tag.as_str());
        child
            .children
            .push(xmltree::XMLNode::Text(value.to_string()));
        root.children.push(xmltree::XMLNode::Element(child));
    }

    let mut file = File::create(&path)?;
    root.write_with_config(
        &mut file,
        xmltree::EmitterConfig::new().perform_indent(true),
    )?;

    println!("Created interactive XML file: {}", path.display());
    Ok(())
}

fn zip_menu() -> Result<()> {
    loop {
        println!("\nZip Utilities");
        println!("1. Create archive");
        println!("2. Add file to archive");
        println!("3. Extract file from archive");
        println!("4. Delete archive");
        println!("0. Cancel");

        match get_choice()? {
            // TODO: Zip bomb protection, is it possible to extract the file to the current dirrectory. Return error.
            1 => {
                let path = get_input("Enter archive path")?;
                zip_create(&PathBuf::from(path))?;
                return Ok(());
            }
            2 => {
                let archive = get_input("Enter archive path")?;
                let filename = get_input("Enter file to add")?;
                zip_add(&PathBuf::from(archive), &PathBuf::from(filename))?;
                return Ok(());
            }
            3 => {
                let archive = get_input("Enter archive path")?;
                let filename = get_input("Enter filename inside archive")?;
                zip_extract(&PathBuf::from(archive), &filename)?;
                return Ok(());
            }
            4 => {
                let path = get_input("Enter archive path")?;
                file_delete(&PathBuf::from(path))?;
                return Ok(());
            }
            0 => return Ok(()),
            _ => println!("Invalid choice"),
        }
    }
}

fn cmd_disks() -> Result<()> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    println!("Logical disks:");
    for disk in &disks {
        println!(
            "- {} (mounted at {})",
            disk.name().to_string_lossy(),
            disk.mount_point().to_string_lossy()
        );
        println!(
            "Filesystem: {}",
            String::from_utf8_lossy(disk.file_system().as_encoded_bytes())
        );
        println!("Size: {} bytes", disk.total_space());
        println!(
            "Used: {} bytes",
            disk.total_space() - disk.available_space()
        );
        println!("Available: {} bytes", disk.available_space());
    }
    Ok(())
}

fn file_create(path: &Path) -> Result<()> {
    let path = sanitize_path(path, true)?;
    if path.exists() {
        return Err(anyhow!("File '{}' already exists", path.display()));
    }
    File::create(&path)
        .with_context(|| format!("Creating file {}", path.display()))?;
    println!("Created {}", path.display());
    Ok(())
}

fn file_write(path: &Path, content: &str) -> Result<()> {
    let path = sanitize_path(path, false)?;
    let mut file = File::create(&path)
        .with_context(|| format!("Creating/overwriting {}", path.display()))?;
    file.write_all(content.as_bytes())?;
    println!("Wrote to {}", path.display());
    Ok(())
}

fn file_read(path: &Path) -> Result<()> {
    let path = sanitize_path(path, false)?;
    let mut content = String::new();
    let mut file = File::open(&path)
        .with_context(|| format!("Opening {}", path.display()))?;
    file.read_to_string(&mut content)?;
    print!("{content}");
    Ok(())
}

fn file_delete(path: &Path) -> Result<()> {
    let path = sanitize_path(path, false)?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Deleting {}", path.display()))?;
        println!("Deleted {}", path.display());
        Ok(())
    } else {
        Err(anyhow!("File '{}' does not exist", path.display()))
    }
}

fn json_create(
    path_buf: PathBuf,
    content_: Option<String>,
    edit: bool,
) -> Result<()> {
    let path = sanitize_path(&path_buf.into_boxed_path(), true)?;
    if edit {
        open_in_editor(&path)?;
        println!("Created via editor: {}", path.display());
        return Ok(());
    }
    if let Some(content) = content_ {
        let value: JsonValue = serde_json::from_str(&content)
            .with_context(|| "CONTENT is not valid JSON")?;
        let ser = serde_json::to_vec_pretty(&value)?;
        fs::write(&path, ser)?;
        println!("Created {} with provided JSON content", path.display());
        Ok(())
    } else {
        Err(anyhow!("Either -c CONTENT or -e must be provided"))
    }
}

fn _json_new(object: &str, path: &Path) -> Result<()> {
    let path = sanitize_path(path, false)?;
    let value: JsonValue = serde_json::from_str(object)
        .with_context(|| "Provided object is not valid JSON")?;
    let ser = serde_json::to_vec_pretty(&value)?;
    fs::write(&path, ser)?;
    println!("Wrote JSON object to {}", path.display());
    Ok(())
}

fn json_read(path: &Path) -> Result<()> {
    let path = sanitize_path(path, false)?;
    let mut string = String::new();
    File::open(&path)?.read_to_string(&mut string)?;
    let value: JsonValue = serde_json::from_str(&string)
        .with_context(|| "File is not valid JSON")?;
    let pretty = serde_json::to_string_pretty(&value)?;
    println!("{pretty}");
    Ok(())
}

fn xml_new(path: &Path) -> Result<()> {
    let path = sanitize_path(path, true)?;
    if path.exists() {
        return Err(anyhow!("File '{}' already exists", path.display()));
    }
    let root = Element::new("root");
    let mut file = File::create(&path)?;
    root.write(&mut file)?;
    println!("Created XML {}", path.display());
    Ok(())
}

fn xml_write(path: &Path, content: &str) -> Result<()> {
    let path = sanitize_path(path, false)?;
    if !path.exists() {
        return Err(anyhow!("File '{}' does not exist", path.display()));
    }
    let mut file = File::open(&path)?;
    let mut content_ = String::new();
    file.read_to_string(&mut content_)?;
    let mut root = Element::parse(content_.as_bytes())
        .with_context(|| "Parsing existing XML")?;
    match Element::parse(content.as_bytes()) {
        Ok(new_elem) => root.children.push(xmltree::XMLNode::Element(new_elem)),
        Err(_) => {
            let mut entry = Element::new("entry");
            entry
                .children
                .push(xmltree::XMLNode::Text(content.to_string()));
            root.children.push(xmltree::XMLNode::Element(entry));
        }
    }
    let mut out = File::create(&path)?;
    root.write_with_config(
        &mut out,
        xmltree::EmitterConfig::new().perform_indent(true),
    )?;
    println!("Appended to {}", path.display());
    Ok(())
}

fn xml_read(path: &Path) -> Result<()> {
    let path = sanitize_path(path, false)?;
    let mut s = String::new();
    File::open(&path)?.read_to_string(&mut s)?;
    let root = Element::parse(s.as_bytes())?;
    let mut buf = Vec::new();
    root.write_with_config(
        &mut buf,
        xmltree::EmitterConfig::new().perform_indent(true),
    )?;
    let pretty = String::from_utf8(buf)?;
    println!("{pretty}");
    Ok(())
}

fn open_in_editor(path: &Path) -> Result<()> {
    if !path.exists() {
        File::create(path)?;
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(editor).arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Editor exited with non-zero"))
    }
}

fn zip_create(path: &Path) -> Result<()> {
    let path = sanitize_path(path, true)?;
    if path.exists() {
        return Err(anyhow!("Archive '{}' already exists", path.display()));
    }
    let f = File::create(&path)?;
    let mut zip = ZipWriter::new(f);
    zip.finish()?;
    println!("Created archive {}", path.display());
    Ok(())
}

fn zip_add(archive_path: &Path, filename: &Path) -> Result<()> {
    let archive_path = sanitize_path(archive_path, false)?;
    let filename = sanitize_path(filename, true)?;

    if !archive_path.exists() {
        return Err(anyhow!(
            "Archive '{}' does not exist",
            archive_path.display()
        ));
    }
    if !filename.exists() {
        return Err(anyhow!("File '{}' does not exist", filename.display()));
    }

    let mut existing: Vec<(String, Vec<u8>)> = Vec::new();
    {
        let f = File::open(&archive_path)?;
        let mut za = ZipArchive::new(f)?;
        for i in 0..za.len() {
            let mut file = za.by_index(i)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            existing.push((file.name().to_string(), buf));
        }
    }

    let f = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(f);
    let options: FileOptions<ExtendedFileOptions> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    for (name, data) in existing {
        zip.start_file(name, options.clone())?;
        zip.write_all(&data)?;
    }

    let name = filename.file_name().unwrap().to_string_lossy().to_string();
    let mut fsrc = File::open(&filename)?;
    let mut buf = Vec::new();
    fsrc.read_to_end(&mut buf)?;
    zip.start_file(name.clone(), options)?;
    zip.write_all(&buf)?;
    zip.finish()?;

    println!("Added {} to {}", filename.display(), archive_path.display());
    Ok(())
}

fn zip_extract(archive_path: &Path, filename: &str) -> Result<()> {
    const EXPANSION_RATIO_LIMIT_PERCENT: u64 = 10_000;

    let archive_path = sanitize_path(archive_path, false)?;

    if !archive_path.exists() {
        return Err(anyhow!(
            "Archive '{}' does not exist",
            archive_path.display()
        ));
    }
    let file = File::open(&archive_path)?;
    let mut za = ZipArchive::new(file)?;
    let mut found = false;
    for i in 0..za.len() {
        let mut file = za.by_index(i)?;
        if file.name() == filename {
            found = true;
            let uncompressed_size = file.size();
            let compressed_size = file.compressed_size();

            if compressed_size > 0 {
                let expansion_ratio_percent =
                    (uncompressed_size as u128 * 100) / compressed_size as u128;

                if expansion_ratio_percent
                    > EXPANSION_RATIO_LIMIT_PERCENT as u128
                {
                    return Err(anyhow!(
                        "Expansion ratio of {}% exceeds the limit of {}% - potential zip bomb. Aborting.",
                        expansion_ratio_percent,
                        EXPANSION_RATIO_LIMIT_PERCENT
                    ));
                }
            } else if uncompressed_size > 0 {
                return Err(anyhow!(
                        "File has an infinite compression ratio ({} bytes from 0) - potential zip bomb. Aborting.",
                        uncompressed_size
                    ));
            }

            let outpath = sanitize_path(Path::new(filename), true)?;
            let parent_dir = outpath.parent().unwrap_or_else(|| Path::new("."));
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir)?;
            }
            let free_space = free_space(parent_dir)?;

            if uncompressed_size > free_space {
                return Err(anyhow!(
                    "Not enough disk space. Required: {}, Available: {}",
                    uncompressed_size,
                    free_space
                ));
            }

            let mut outfile = File::create(&outpath)?;
            let bytes_written = io::copy(&mut file, &mut outfile)?;

            let savings_percent = if uncompressed_size > 0 {
                let savings = uncompressed_size as f64 - compressed_size as f64;
                (savings / uncompressed_size as f64) * 100.0
            } else {
                0.0
            };

            println!(r"Extracted: {}", filename);
            println!(" - Uncompressed size: {} bytes", uncompressed_size);
            println!(" - Compressed size: {} bytes", compressed_size);
            println!(" - Compression savings: {:.2}%", savings_percent);
            println!(" - Last modified: {:?}", file.last_modified());
            println!(" - Written: {} bytes", bytes_written);
            break;
        }
    }
    if !found {
        return Err(anyhow!("File '{}' not found in archive", filename));
    }
    Ok(())
}
